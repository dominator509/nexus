//! EP-029 direct platform transport integration tests (M3).
//!
//! The production HTTP transport under test is REAL
//! (`HttpDirectPlatformTransport`, reqwest blocking). The peer is a
//! controlled local HTTP fixture over REAL std::net sockets that emits
//! REAL X API v2-shaped responses (the DOCUMENTED official surface):
//! 200 user, 200 mentions, 200 tweet with public metrics, 200 created
//! tweet, 401, 404, 429, 5xx, malformed JSON, silent peer. Mocks
//! control the peer only; the transport is never mocked.
//!
//! Certification boundary: these fixtures prove request construction,
//! response/status semantics, classification, and failure behavior
//! over real HTTP. They NEVER certify a real X/social platform
//! provider; real provider certification requires an owned account +
//! live API credentials and remains NOT ASSERTED (DEFERRED).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use nexus_social::{SocialErrorCode, SocialProvider};
use nexus_social_direct_connector::{
    DirectPlatformAdapter, DirectPlatformTransport, HttpDirectPlatformTransport,
};

const CANARY_TOKEN: &str = "EP029PW_CANARY_d7f2";

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

fn read_request_body(stream: &mut TcpStream, head: &[u8]) -> Vec<u8> {
    // Read the request body after the header block. The transport
    // always sends Content-Length for JSON bodies; parse it from the
    // head and read exactly that many bytes.
    let text = String::from_utf8_lossy(head);
    let mut content_length = 0usize;
    for line in text.lines() {
        let lower = line.to_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0u8; content_length];
    let mut read = 0usize;
    while read < content_length {
        match stream.read(&mut body[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(_) => break,
        }
    }
    body.truncate(read);
    body
}

fn spawn_server_with_body<F>(handler: F) -> (u16, JoinHandle<()>)
where
    F: Fn(&str, &str, &str) -> (u16, &'static str, String) + Send + 'static,
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
        let req_body = String::from_utf8_lossy(&read_request_body(&mut stream, &head)).to_string();
        let (status, content_type, body) = handler(&method, &path, &req_body);
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
        // Accept but never respond; keep the socket OPEN past the
        // transport timeout (true silent peer -> Timeout).
        let mut buf = [0u8; 1];
        let _ = stream.read(&mut buf);
        thread::sleep(Duration::from_secs(5));
    });
    (port, handle)
}

/// Multi-connection fixture that answers up to N sequential requests
/// and asserts the Authorization header on each.
fn spawn_authed_server<F>(handler: F) -> (u16, JoinHandle<()>)
where
    F: Fn(&str, &str) -> (u16, &'static str, String) + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(15);
        for _ in 0..12 {
            listener.set_nonblocking(true).expect("nonblocking");
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
            let text = String::from_utf8_lossy(&head);
            // The transport must present the Authorization header with
            // the canary token. reqwest may send the header name
            // lowercased, so compare case-insensitively; the token
            // value itself is case-sensitive.
            let lower = text.to_lowercase();
            let auth_ok = lower.contains("authorization:") && text.contains(CANARY_TOKEN);
            let (method, path) = parse_request_line(&head);
            let (status, content_type, body) = if auth_ok {
                handler(&method, &path)
            } else {
                (
                    401,
                    "application/json",
                    "{\"errors\":[{\"message\":\"unauthorized\"}]}".to_string(),
                )
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (port, handle)
}

fn transport(port: u16) -> HttpDirectPlatformTransport {
    HttpDirectPlatformTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_TOKEN,
        Duration::from_millis(1500),
    )
}

#[test]
fn ep029_integration_me_returns_documented_user() {
    let body = r#"{"data":{"id":"u-1","name":"Nexus","username":"nexus"}}"#.to_string();
    let (port, handle) = spawn_server(move |method, path| {
        assert_eq!(method, "GET");
        assert_eq!(path, "/2/users/me");
        (200, "application/json", body.clone())
    });
    let t = transport(port);
    let user = t.me().unwrap();
    assert_eq!(user.id, "u-1");
    assert_eq!(user.username, "nexus");
    handle.join().unwrap();
}

#[test]
fn ep029_integration_mentions_returns_documented_list() {
    let body = r#"{"data":[{"id":"m-1","text":"hello","author_id":"a-1","created_at":"2026-08-19T00:00:00Z"},{"id":"m-2","text":"hi","author_id":"a-2"}]}"#.to_string();
    let (port, handle) = spawn_server(move |method, path| {
        assert_eq!(method, "GET");
        assert!(path.starts_with("/2/users/u-1/mentions"));
        (200, "application/json", body.clone())
    });
    let t = transport(port);
    let mentions = t.mentions("u-1").unwrap();
    assert_eq!(mentions.len(), 2);
    assert_eq!(mentions[0].author_id, "a-1");
    handle.join().unwrap();
}

#[test]
fn ep029_integration_tweet_with_metrics_returns_documented_shape() {
    let body = r#"{"data":{"id":"t-1","text":"hi","public_metrics":{"like_count":3,"retweet_count":1,"reply_count":2,"quote_count":0,"impression_count":10,"bookmark_count":0}}}"#.to_string();
    let (port, handle) = spawn_server(move |method, path| {
        assert_eq!(method, "GET");
        assert!(path.contains("/2/tweets/t-1"));
        assert!(path.contains("tweet.fields=public_metrics"));
        (200, "application/json", body.clone())
    });
    let t = transport(port);
    let tweet = t.tweet_with_metrics("t-1").unwrap();
    assert_eq!(tweet.public_metrics.impression_count, 10);
    assert_eq!(tweet.public_metrics.like_count, 3);
    handle.join().unwrap();
}

#[test]
fn ep029_integration_create_tweet_posts_and_returns_id() {
    let body = r#"{"data":{"id":"t-9","text":"hello world"}}"#.to_string();
    let (port, handle) = spawn_server(move |method, path| {
        assert_eq!(method, "POST");
        assert_eq!(path, "/2/tweets");
        (200, "application/json", body.clone())
    });
    let t = transport(port);
    let created = t.create_tweet("hello world").unwrap();
    assert_eq!(created.id, "t-9");
    handle.join().unwrap();
}

#[test]
fn ep029_integration_reply_to_tweet_carries_official_reply_object() {
    // AUD-024: a reply is NEVER a standalone post. The request body
    // on the real wire MUST carry the official
    // `reply.in_reply_to_tweet_id` object (X API v2 CreatePostsReply
    // schema; the id is a 1..=19 digit string). The fixture reads the
    // actual POST body - not just the request line - so this proves
    // the wire shape, not merely the method/path.
    let body = r#"{"data":{"id":"t-10","text":"replying"}}"#.to_string();
    let (port, handle) = spawn_server_with_body(move |method, path, req_body| {
        assert_eq!(method, "POST");
        assert_eq!(path, "/2/tweets");
        let parsed: serde_json::Value =
            serde_json::from_str(req_body).expect("transport must send a JSON body");
        assert_eq!(parsed["text"], "replying");
        assert_eq!(
            parsed["reply"]["in_reply_to_tweet_id"],
            "1234567890123456789"
        );
        (200, "application/json", body.clone())
    });
    let t = transport(port);
    let created = t.reply_to_tweet("replying", "1234567890123456789").unwrap();
    assert_eq!(created.id, "t-10");
    handle.join().unwrap();
}

#[test]
fn ep029_integration_create_tweet_never_carries_reply_object() {
    // AUD-024 hostile regression: a standalone post and a reply are
    // DIFFERENT wire shapes. create_tweet must NOT smuggle the reply
    // object - if it did, every post would silently become a reply.
    let body = r#"{"data":{"id":"t-9","text":"hello world"}}"#.to_string();
    let (port, handle) = spawn_server_with_body(move |method, path, req_body| {
        assert_eq!(method, "POST");
        assert_eq!(path, "/2/tweets");
        let parsed: serde_json::Value =
            serde_json::from_str(req_body).expect("transport must send a JSON body");
        assert_eq!(parsed["text"], "hello world");
        assert!(
            parsed.get("reply").is_none(),
            "standalone post must not carry a reply object: {req_body}"
        );
        (200, "application/json", body.clone())
    });
    let t = transport(port);
    let created = t.create_tweet("hello world").unwrap();
    assert_eq!(created.id, "t-9");
    handle.join().unwrap();
}

#[test]
fn ep029_integration_reply_to_tweet_fails_closed_on_missing_thread_ref() {
    // AUD-024: a reply without a thread reference fails closed BEFORE
    // any provider call - a standalone post is never fabricated.
    let (port, handle) = spawn_server(|_, _| {
        panic!("server must not be reached when thread ref is invalid");
    });
    let t = transport(port);
    let err = t.reply_to_tweet("replying", "").unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Validation);
    let err = t.reply_to_tweet("replying", "not-a-number").unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Validation);
    let err = t
        .reply_to_tweet("replying", "123456789012345678901234567890")
        .unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Validation);
    handle.join().unwrap();
}

#[test]
fn ep029_integration_status_classification_real_http() {
    // 401 -> Authorization
    let (port, handle) = spawn_server(|_, _| (401, "application/json", "{}".to_string()));
    let err = transport(port).me().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Authorization);
    handle.join().unwrap();

    // 404 -> NotFound
    let (port, handle) = spawn_server(|_, _| (404, "application/json", "{}".to_string()));
    let err = transport(port).me().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::NotFound);
    handle.join().unwrap();

    // 429 -> RateLimit
    let (port, handle) = spawn_server(|_, _| (429, "application/json", "{}".to_string()));
    let err = transport(port).me().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::RateLimit);
    handle.join().unwrap();

    // 500 -> Unavailable
    let (port, handle) = spawn_server(|_, _| (500, "application/json", "{}".to_string()));
    let err = transport(port).me().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Unavailable);
    handle.join().unwrap();
}

#[test]
fn ep029_integration_malformed_json_fails_closed() {
    let (port, handle) =
        spawn_server(|_, _| (200, "application/json", "<html>not json".to_string()));
    let err = transport(port).me().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::ExternalProvider);
    handle.join().unwrap();
}

#[test]
fn ep029_integration_silent_peer_times_out() {
    let (port, handle) = spawn_silent_server();
    let t = transport(port);
    let err = t.me().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Timeout);
    handle.join().unwrap();
}

#[test]
fn ep029_integration_refused_port_is_unavailable() {
    // Bind a listener, grab the port, drop it -> refused connection.
    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let t = transport(port);
    let err = t.me().unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Unavailable);
}

#[test]
fn ep029_integration_adapter_capabilities_and_strategic_gaps() {
    // Real adapter over the documented surface: capabilities only
    // when the transport answers; conversations/metrics/leads come
    // from REAL mentions.
    use nexus_domain::{BusinessId, TenantId};
    use nexus_social::{SocialCapabilityKind, SocialProvider};
    use std::str::FromStr;

    let tenant = TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap();
    let business = BusinessId::from_str("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap();

    let user_body = r#"{"data":{"id":"u-1","name":"Nexus","username":"nexus"}}"#.to_string();
    let mentions_body = r#"{"data":[{"id":"m-1","text":"lead inquiry","author_id":"a-1","created_at":"2026-08-19T00:00:00Z"}]}"#.to_string();
    let tweet_body = r#"{"data":{"id":"m-1","text":"lead inquiry","public_metrics":{"like_count":1,"retweet_count":0,"reply_count":1,"quote_count":0,"impression_count":5,"bookmark_count":0}}}"#.to_string();

    let (port, handle) = spawn_authed_server(move |method, path| {
        if method == "GET" && path == "/2/users/me" {
            (200, "application/json", user_body.clone())
        } else if method == "GET" && path.starts_with("/2/users/u-1/mentions") {
            (200, "application/json", mentions_body.clone())
        } else if method == "GET" && path.contains("/2/tweets/m-1") {
            (200, "application/json", tweet_body.clone())
        } else {
            (404, "application/json", "{}".to_string())
        }
    });

    let adapter = DirectPlatformAdapter::new(
        Box::new(transport(port)),
        tenant.clone(),
        business.clone(),
        CANARY_TOKEN,
    );

    let caps = adapter.capabilities();
    assert!(caps.contains(SocialCapabilityKind::Publish));
    assert!(caps.contains(SocialCapabilityKind::ReadConversations));
    assert!(caps.contains(SocialCapabilityKind::ReadMetrics));
    assert!(caps.contains(SocialCapabilityKind::LeadHandoff));

    // Conversations from real mentions.
    let conversations = adapter.list_conversations(&tenant, &business).unwrap();
    assert_eq!(conversations.len(), 1);
    assert_eq!(conversations[0].thread_ref, "x:m-1");

    // Metrics from real public metrics, attributed to a campaign.
    let campaign = nexus_hydra::CampaignId::new("campaign-1").unwrap();
    let metrics = adapter
        .list_metrics(&tenant, &business, Some(&campaign))
        .unwrap();
    assert!(!metrics.is_empty());
    assert!(metrics
        .iter()
        .any(|m| m.campaign_id.as_ref() == Some(&campaign)));

    // Leads from real mentions, unlinked (deterministic/human link is
    // a later explicit step).
    let leads = adapter.list_leads(&tenant, &business).unwrap();
    assert_eq!(leads.len(), 1);

    // Audit ring records operations with no credential leakage.
    let audit = adapter.audit();
    assert!(!audit.is_empty());
    for entry in &audit {
        assert!(!entry.detail.contains(CANARY_TOKEN));
    }

    handle.join().unwrap();
}

#[test]
fn ep029_integration_adapter_fails_closed_on_unreachable_transport() {
    use nexus_domain::{BusinessId, TenantId};
    use std::str::FromStr;

    let tenant = TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap();
    let business = BusinessId::from_str("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap();

    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let adapter = DirectPlatformAdapter::new(
        Box::new(transport(port)),
        tenant.clone(),
        business.clone(),
        CANARY_TOKEN,
    );
    // Unreachable transport -> capabilities empty (fail closed) and
    // conversations Unavailable (never fabricated).
    assert!(adapter.capabilities().is_empty());
    let err = adapter.list_conversations(&tenant, &business).unwrap_err();
    assert_eq!(err.code, SocialErrorCode::Unavailable);
}

#[test]
fn ep029_integration_adapter_reply_binds_thread_ref_on_wire() {
    // AUD-024 end-to-end: adapter.reply() must resolve the
    // conversation's thread_ref ("x:{mention tweet id}") and carry the
    // official reply object with the prefix-stripped id on the REAL
    // wire. Without the fix this posts a standalone tweet - the
    // fixture would see no reply object and fail the test.
    use nexus_domain::{BusinessId, PersonId, TenantId};
    use nexus_hydra::{SocialAccountId, SocialMessageId};
    use nexus_social::{
        PublishApproval, PublishApprovalId, SocialActionKind, SocialApprovalState,
        SocialConversation, SocialConversationId,
    };
    use std::str::FromStr;

    let tenant = TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap();
    let business = BusinessId::from_str("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap();
    let person = PersonId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap();

    let created_body = r#"{"data":{"id":"t-77","text":"thanks"}}"#.to_string();
    let (port, handle) = spawn_server_with_body(move |method, path, req_body| {
        assert_eq!(method, "POST");
        assert_eq!(path, "/2/tweets");
        let parsed: serde_json::Value =
            serde_json::from_str(req_body).expect("transport must send a JSON body");
        assert_eq!(parsed["text"], "ref://reply-1");
        assert_eq!(
            parsed["reply"]["in_reply_to_tweet_id"], "9876543210987654321",
            "adapter must strip the x: prefix and bind the mention tweet id"
        );
        (200, "application/json", created_body.clone())
    });

    let adapter = DirectPlatformAdapter::new(
        Box::new(transport(port)),
        tenant.clone(),
        business.clone(),
        CANARY_TOKEN,
    );

    let conv = SocialConversation::new(
        SocialConversationId::new("conv-1").unwrap(),
        SocialAccountId::new("acct-1").unwrap(),
        business.clone(),
        "x",
        "x:9876543210987654321",
    );
    let mut approval = PublishApproval::new(
        PublishApprovalId::new("ap-1").unwrap(),
        tenant.clone(),
        business.clone(),
        SocialActionKind::Reply,
        SocialMessageId::new("x:9876543210987654321").unwrap(),
    );
    approval.grant(person).unwrap();
    assert_eq!(approval.state, SocialApprovalState::Granted);

    let id = adapter.reply(&conv, &approval, "ref://reply-1").unwrap();
    assert_eq!(id.as_str(), "x:t-77");
    handle.join().unwrap();
}
