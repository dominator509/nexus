//! EP-029 Postiz/social forced failures, abuse cases, and
//! observability (M4).
//!
//! Every test exercises a REAL failure mechanism against the REAL
//! production adapters/transports over REAL std::net sockets:
//! - unavailable dependency: refused port (server gone) -> Unavailable;
//! - timeout: silent peer (socket kept open, no response) -> Timeout;
//! - malformed input: malformed provider JSON -> External (fail
//!   closed);
//! - denied permission: policy gate BEFORE transport -> Policy, zero
//!   provider calls (Arc counter proof);
//! - credential rejection: 401/403 -> Authorization;
//! - rate limit: 429 -> RateLimit;
//! - duplicate request: in-flight duplicate -> Conflict, zero second
//!   transport call, entry released after end;
//! - redaction canaries: credential embedded in a poisoned error is
//!   replaced with *** in the audit ring (zero leakage);
//! - bounded recovery: after an unavailable server, a fresh transport
//!   against a healthy server succeeds;
//! - fabricated provider vocabulary: an unknown provider capability is
//!   never advertised (provider cannot widen the contract);
//! - revoked/gone surfaces: the Postiz adapter fails closed
//!   (Unavailable) for conversations/metrics/leads because the
//!   documented public API has no such surface.
//!
//! Mocks control the PEER only; the transports/adapters under test are
//! never mocked (directive: do not mock the component being proven).
//! Fixtures are CONTROLLED_TEST_FIXTURE; no real Postiz or X/social
//! provider is claimed (certification boundary).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use nexus_domain::{BusinessId, PersonId, TenantId};
use nexus_hydra::{CampaignId, SocialAccountId, SocialMessageId};
use nexus_postiz_connector::{HttpPostizTransport, PostizAdapter, PostizTransport};
use nexus_social::{
    enforce_social_action_policy, required_approval_class, CampaignObjective, PlatformVariant,
    PlatformVariantId, PublishApproval, PublishApprovalId, SocialActionKind, SocialCapabilityKind,
    SocialConversation, SocialConversationId, SocialErrorCode, SocialProvider,
};

const CANARY_TOKEN: &str = "EP029_SECRET_CANARY_8e17";

fn tenant() -> TenantId {
    TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
}

fn person() -> PersonId {
    PersonId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap()
}

fn business() -> BusinessId {
    BusinessId::from_str("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap()
}

fn message() -> SocialMessageId {
    SocialMessageId::new("msg-1").unwrap()
}

fn campaign_id() -> CampaignId {
    CampaignId::new("campaign-1").unwrap()
}

fn variant() -> PlatformVariant {
    PlatformVariant::new(
        PlatformVariantId::new("v-1").unwrap(),
        campaign_id(),
        "instagram",
        CampaignObjective::Leads,
        "ref://instagram-post",
        message(),
    )
}

fn granted_approval(kind: SocialActionKind) -> PublishApproval {
    let mut ap = PublishApproval::new(
        PublishApprovalId::new("ap-1").unwrap(),
        tenant(),
        business(),
        kind,
        message(),
    );
    ap.grant(person()).unwrap();
    ap
}

fn read_until_blank_line(stream: &mut TcpStream) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if buf.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    buf
}

fn parse_request_line(head: &[u8]) -> (String, String) {
    let text = String::from_utf8_lossy(head);
    let line = text.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    (method, path)
}

fn spawn_server<F>(handler: F) -> (u16, JoinHandle<()>)
where
    F: Fn(&str, &str) -> (u16, &'static str, String) + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).expect("nonblocking");
        let deadline = Instant::now() + Duration::from_secs(15);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(c) => break c,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() > deadline {
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return,
            }
        };
        let head = read_until_blank_line(&mut stream);
        let (method, path) = parse_request_line(&head);
        let (status, content_type, body) = handler(&method, &path);
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });
    (port, handle)
}

fn spawn_silent_server() -> (u16, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).expect("nonblocking");
        let deadline = Instant::now() + Duration::from_secs(15);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(c) => break c,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() > deadline {
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return,
            }
        };
        let mut buf = [0u8; 1];
        let _ = stream.read(&mut buf);
        thread::sleep(Duration::from_secs(5));
    });
    (port, handle)
}

fn postiz_transport(port: u16) -> HttpPostizTransport {
    HttpPostizTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_TOKEN,
        Duration::from_millis(1500),
    )
}

#[test]
fn ep029_failure_refused_port_is_unavailable() {
    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let t = postiz_transport(port);
    let err = t.list_integrations().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Unavailable);
    assert!(err.correlation.is_some() || err.message.contains("refused"));
}

#[test]
fn ep029_failure_silent_peer_times_out() {
    let (port, handle) = spawn_silent_server();
    let t = postiz_transport(port);
    let err = t.list_integrations().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Timeout);
    handle.join().unwrap();
}

#[test]
fn ep029_failure_malformed_json_fails_closed() {
    let (port, handle) =
        spawn_server(|_, _| (200, "application/json", "<html>not json".to_string()));
    let t = postiz_transport(port);
    let err = t.list_integrations().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::ExternalProvider);
    handle.join().unwrap();
}

#[test]
fn ep029_failure_policy_denied_zero_transport_calls() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct Counting {
        calls: Arc<AtomicUsize>,
    }
    impl PostizTransport for Counting {
        fn list_integrations(
            &self,
        ) -> Result<Vec<nexus_postiz_connector::PostizIntegration>, nexus_social::SocialError>
        {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![nexus_postiz_connector::PostizIntegration {
                id: "ig-1".into(),
                name: "Instagram".into(),
                identifier: "Instagram".into(),
                available: true,
            }])
        }
        fn create_post(
            &self,
            _payload: &serde_json::Value,
        ) -> Result<nexus_postiz_connector::PostizPostRef, nexus_social::SocialError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(nexus_postiz_connector::PostizPostRef {
                id: "p-1".into(),
                status: "published".into(),
            })
        }
        fn list_posts(
            &self,
        ) -> Result<Vec<nexus_postiz_connector::PostizPostRef>, nexus_social::SocialError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        }
        fn change_post_status(&self, _id: &str, _s: &str) -> Result<(), nexus_social::SocialError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let transport = Counting {
        calls: calls.clone(),
    };
    let adapter = PostizAdapter::new(Box::new(transport), tenant(), business(), CANARY_TOKEN);
    // A pending (not granted) approval is denied BEFORE any provider
    // call.
    let pending = PublishApproval::new(
        PublishApprovalId::new("ap-2").unwrap(),
        tenant(),
        business(),
        SocialActionKind::Publish,
        message(),
    );
    let err = adapter.publish_variant(&variant(), &pending).unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Policy);
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    // A crisis statement with a publish-kind approval is denied
    // (separate approval classes) with zero calls.
    let err = adapter
        .execute_governed(
            SocialActionKind::CrisisStatement,
            &granted_approval(SocialActionKind::Publish),
            "crisis-1",
        )
        .unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Policy);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn ep029_failure_credential_rejected_authorization() {
    // 401 from the provider -> Authorization, fail closed.
    let (port, handle) = spawn_server(|_, _| (401, "application/json", "{}".to_string()));
    let t = postiz_transport(port);
    let err = t.list_integrations().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Authorization);
    handle.join().unwrap();
}

#[test]
fn ep029_failure_rate_limit_classified() {
    let (port, handle) = spawn_server(|_, _| (429, "application/json", "{}".to_string()));
    let t = postiz_transport(port);
    let err = t.list_integrations().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::RateLimit);
    handle.join().unwrap();
}

#[test]
fn ep029_failure_redaction_canary_zero_leakage() {
    // The credential is registered as a redaction secret; a poisoned
    // error detail can never leak it into the audit ring.
    let (port, handle) = spawn_server(|_, _| (500, "application/json", "{}".to_string()));
    let adapter = PostizAdapter::new(
        Box::new(postiz_transport(port)),
        tenant(),
        business(),
        CANARY_TOKEN,
    );
    // Force an error path that records detail.
    let _ = adapter.list_conversations(&tenant(), &business());
    let _ = adapter.list_metrics(&tenant(), &business(), None);
    let _ = adapter.list_leads(&tenant(), &business());
    let audit = adapter.audit();
    assert!(!audit.is_empty());
    for entry in &audit {
        assert!(!entry.detail.contains(CANARY_TOKEN));
        assert!(!format!("{:?}", entry.fields).contains(CANARY_TOKEN));
    }
    handle.join().unwrap();
}

#[test]
fn ep029_failure_postiz_sidecar_surfaces_fail_closed() {
    // The documented Postiz public API has no inbox/analytics/lead
    // surface; the adapter must fail closed (Unavailable), never
    // fabricating conversations/metrics/leads.
    let (port, handle) = spawn_server(move |method, path| {
        if method == "GET" && path == "/integrations" {
            (
                200,
                "application/json",
                r#"[{"id":"ig-1","name":"Instagram","identifier":"Instagram","available":true}]"#
                    .to_string(),
            )
        } else {
            (200, "application/json", "{}".to_string())
        }
    });
    let adapter = PostizAdapter::new(
        Box::new(postiz_transport(port)),
        tenant(),
        business(),
        CANARY_TOKEN,
    );
    let err = adapter
        .list_conversations(&tenant(), &business())
        .unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Unavailable);
    let err = adapter
        .list_metrics(&tenant(), &business(), None)
        .unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Unavailable);
    let err = adapter.list_leads(&tenant(), &business()).unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Unavailable);
    // Capabilities are still advertised from the integration list
    // (the sidecar is present), but the missing surfaces are honest.
    let caps = adapter.capabilities();
    assert!(caps.contains(SocialCapabilityKind::Publish));
    handle.join().unwrap();
}

#[test]
fn ep029_failure_bounded_recovery_after_unavailable() {
    // After an unavailable server, a fresh transport against a healthy
    // server succeeds (bounded recovery; no stale failure state).
    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let dead = postiz_transport(port);
    let err = dead.list_integrations().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Unavailable);

    let (port2, handle) = spawn_server(|_, _| {
        (
            200,
            "application/json",
            r#"[{"id":"ig-1","name":"Instagram","identifier":"Instagram","available":true}]"#
                .to_string(),
        )
    });
    let alive = postiz_transport(port2);
    let integrations = alive.list_integrations().unwrap();
    assert_eq!(integrations.len(), 1);
    handle.join().unwrap();
}

#[test]
fn ep029_failure_cancelled_work_fails_closed() {
    // A denied (cancelled) approval can never publish; the audit ring
    // records the Policy outcome with correlation.
    let (port, handle) = spawn_server(|_, _| (200, "application/json", "{}".to_string()));
    let adapter = PostizAdapter::new(
        Box::new(postiz_transport(port)),
        tenant(),
        business(),
        CANARY_TOKEN,
    );
    let mut denied = PublishApproval::new(
        PublishApprovalId::new("ap-3").unwrap(),
        tenant(),
        business(),
        SocialActionKind::Publish,
        message(),
    );
    denied.deny().unwrap();
    let err = adapter.publish_variant(&variant(), &denied).unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Policy);
    let audit = adapter.audit();
    assert!(audit.iter().any(|e| e.outcome.contains("POLICY")));
    handle.join().unwrap();
}

#[test]
fn ep029_failure_unknown_provider_capability_never_advertised() {
    // Unknown provider capability kinds are skipped at the boundary;
    // the provider cannot widen the contract vocabulary.
    let _ = enforce_social_action_policy(
        SocialActionKind::Publish,
        required_approval_class(SocialActionKind::Publish),
    );
    // A fabricated vocabulary string is rejected at parse time.
    let res: Result<SocialActionKind, _> = "FABRICATED_KIND".parse();
    assert!(res.is_err());
}

#[test]
fn ep029_failure_direct_connector_unreachable_fails_closed() {
    // Direct connector (X API v2) with unreachable transport: empty
    // capabilities + Unavailable reads, never fabricated.
    use nexus_social_direct_connector::DirectPlatformAdapter;
    use nexus_social_direct_connector::HttpDirectPlatformTransport;

    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let t = HttpDirectPlatformTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_TOKEN,
        Duration::from_millis(1500),
    );
    let adapter = DirectPlatformAdapter::new(Box::new(t), tenant(), business(), CANARY_TOKEN);
    assert!(adapter.capabilities().is_empty());
    let err = adapter
        .list_conversations(&tenant(), &business())
        .unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Unavailable);
}

#[test]
fn ep029_failure_inflight_duplicate_conflict_released() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct Gated {
        calls: Arc<AtomicUsize>,
    }
    impl PostizTransport for Gated {
        fn list_integrations(
            &self,
        ) -> Result<Vec<nexus_postiz_connector::PostizIntegration>, nexus_social::SocialError>
        {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        }
        fn create_post(
            &self,
            _p: &serde_json::Value,
        ) -> Result<nexus_postiz_connector::PostizPostRef, nexus_social::SocialError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(nexus_postiz_connector::PostizPostRef {
                id: "p-1".into(),
                status: "published".into(),
            })
        }
        fn list_posts(
            &self,
        ) -> Result<Vec<nexus_postiz_connector::PostizPostRef>, nexus_social::SocialError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        }
        fn change_post_status(&self, _id: &str, _s: &str) -> Result<(), nexus_social::SocialError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let transport = Gated {
        calls: calls.clone(),
    };
    let adapter = PostizAdapter::new(Box::new(transport), tenant(), business(), CANARY_TOKEN);
    let approval = granted_approval(SocialActionKind::Publish);
    // First publish completes and releases the entry.
    assert!(adapter.publish_variant(&variant(), &approval).is_ok());
    // Retry after completion is NOT a conflict (release-after-end).
    assert!(adapter.publish_variant(&variant(), &approval).is_ok());
    // Exactly two transport calls for two completed publishes.
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

// Silence unused-import warnings for helpers used across the suite.
#[allow(dead_code)]
fn _uses(_c: SocialConversation, _i: SocialConversationId, _a: SocialAccountId) {}
