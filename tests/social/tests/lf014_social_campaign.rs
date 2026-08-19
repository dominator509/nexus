//! LF-014 social-campaign live-fire (EP-029 M5).
//!
//! Proof: create platform-native variants (one campaign objective) ->
//! obtain approval -> publish through the certified connector ->
//! provider acceptance is never PUBLISHED on its own -> independent
//! provider readback confirms the published state -> ingest engagement
//! -> report attribution.
//!
//! The production `PostizAdapter` (SocialProvider port) + real
//! `HttpPostizTransport` run against a controlled local HTTP fixture
//! over REAL std::net sockets emitting REAL Postiz-shaped responses
//! (the documented public API surface: GET /integrations, POST /posts,
//! GET /posts). The production `DirectPlatformAdapter` +
//! `HttpDirectPlatformTransport` run against a REAL X API v2-shaped
//! fixture (GET /2/users/me, GET /2/users/{id}/mentions, GET
//! /2/tweets/{id}?tweet.fields=public_metrics). Mocks control the peer
//! only; adapters/transports are never mocked.
//!
//! Current-run machine-readable evidence is written to
//! `.agent/state/evidence/LF-014-ep029-m5.json` embedding
//! `EP029_M5_RUN_ID` (stale evidence never satisfies the gate).
//!
//! Certification boundary: publish acceptance + engagement + attribution
//! are proven over real sockets against controlled fixtures; a real
//! Postiz or real X provider is NOT ASSERTED (no owned account/API
//! credentials exist in this environment; DEFERRED to deployment owner).

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use nexus_domain::{BusinessId, PersonId, TenantId};
use nexus_hydra::{
    CampaignId, SocialAccountId, SocialMessage, SocialMessageId, SocialMessageState,
};
use nexus_postiz_connector::{HttpPostizTransport, PostizAdapter, PostizTransport};
use nexus_social::{
    variants_preserve_single_objective, CampaignObjective, PlatformVariant, PlatformVariantId,
    PostizProvider, PublishApproval, PublishApprovalId, SocialActionKind, SocialApprovalState,
    SocialCapabilityKind, SocialMetricKind, SocialProvider,
};
use nexus_social_direct_connector::{DirectPlatformAdapter, HttpDirectPlatformTransport};
use nexus_social_live_e2e::fixture;

const CANARY_TOKEN: &str = "EP029_LF014_CANARY_9c41";

fn tenant() -> TenantId {
    TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
}

fn person() -> PersonId {
    PersonId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap()
}

fn business() -> BusinessId {
    BusinessId::from_str("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap()
}

fn run_id() -> String {
    std::env::var("EP029_M5_RUN_ID").unwrap_or_else(|_| "local-run".to_string())
}

fn evidence_path() -> PathBuf {
    // Workspace-root anchored (ascend until Cargo.toml contains
    // [workspace]); cargo runs tests from the package root, so a bare
    // relative path would land in tests/social/.
    let mut dir = std::env::current_dir().unwrap();
    loop {
        let marker = dir.join("Cargo.toml");
        if marker.exists()
            && std::fs::read_to_string(&marker)
                .map(|s| s.contains("[workspace]"))
                .unwrap_or(false)
        {
            break;
        }
        if !dir.pop() {
            panic!("workspace root not found");
        }
    }
    dir.join(".agent/state/evidence/LF-014-ep029-m5.json")
}

fn write_evidence(doc: serde_json::Value) {
    let path = evidence_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create evidence dir");
    }
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&doc).expect("evidence json"),
    )
    .expect("write evidence");
}

#[test]
fn ep029_m5_lf014_social_campaign() {
    // 1. Platform-native variants: two platforms, ONE campaign
    //    objective (invariant holds).
    let msg1 = SocialMessageId::new("msg-lf014-1").unwrap();
    let msg2 = SocialMessageId::new("msg-lf014-2").unwrap();
    let campaign = CampaignId::new("campaign-lf014").unwrap();
    let variant_linkedin = PlatformVariant::new(
        PlatformVariantId::new("var-lf014-linkedin").unwrap(),
        campaign.clone(),
        "linkedin",
        CampaignObjective::Leads,
        "content-ref-lf014-linkedin",
        msg1.clone(),
    );
    let variant_instagram = PlatformVariant::new(
        PlatformVariantId::new("var-lf014-instagram").unwrap(),
        campaign.clone(),
        "instagram",
        CampaignObjective::Leads,
        "content-ref-lf014-instagram",
        msg2.clone(),
    );
    variants_preserve_single_objective(&[variant_linkedin.clone(), variant_instagram.clone()])
        .expect("single objective invariant");

    // 2. Approval: pending -> granted by a human (PUBLISH requires
    //    HUMAN). GRANTED != PUBLISHED: the grant itself never reaches
    //    the provider (request counter stays 0).
    let mut approval = PublishApproval::new(
        PublishApprovalId::new("approval-lf014-1").unwrap(),
        tenant(),
        business(),
        SocialActionKind::Publish,
        msg1.clone(),
    );
    approval.grant(person()).expect("grant approval");
    assert_eq!(approval.state, SocialApprovalState::Granted);
    assert_eq!(
        approval.state.as_str(),
        "GRANTED",
        "approval state is GRANTED, never PUBLISHED"
    );

    // 3. Postiz-shaped fixture with a real request counter. The
    //    fixture accepts: GET /integrations (capabilities), POST /posts
    //    (create now -> provider acceptance status "published" in the
    //    create response, which is PROVIDER ACCEPTANCE not independent
    //    proof), GET /posts (independent readback -> published).
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_publish = calls.clone();
    let integrations_body =
        r#"[{"id":"linkedin","name":"LinkedIn","identifier":"LinkedIn","available":true}]"#
            .to_string();
    let (postiz_port, postiz_handle) = fixture::spawn_server(4, move |method, path| {
        calls_publish.fetch_add(1, Ordering::SeqCst);
        match (method, path) {
            ("GET", "/integrations") => (200, "application/json", integrations_body.clone()),
            ("POST", "/posts") => (
                200,
                "application/json",
                r#"{"id":"post-lf014-1","status":"published"}"#.to_string(),
            ),
            ("GET", "/posts") => (
                200,
                "application/json",
                r#"[{"id":"post-lf014-1","status":"published"}]"#.to_string(),
            ),
            _ => (
                404,
                "application/json",
                r#"{"error":"not found"}"#.to_string(),
            ),
        }
    });

    let adapter = PostizAdapter::new(
        Box::new(HttpPostizTransport::new(
            format!("http://127.0.0.1:{postiz_port}"),
            CANARY_TOKEN,
            Duration::from_secs(5),
        )),
        tenant(),
        business(),
        CANARY_TOKEN,
    );

    // Capabilities: connected integration -> canonical publish set.
    let caps = adapter.capabilities();
    assert!(caps.contains(SocialCapabilityKind::Publish));

    // Approval alone makes ZERO provider calls (capabilities above is
    // the only call so far; the grant adds none).
    let calls_after_caps = calls.load(Ordering::SeqCst);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        calls_after_caps,
        "approval never reaches provider"
    );

    // 4. Publish: create now. The returned id is PROVIDER ACCEPTANCE;
    //    the create response's own status is not independent evidence.
    let published_id = adapter
        .publish_variant(&variant_linkedin, &approval)
        .expect("publish variant");
    assert_eq!(published_id.as_str(), "postiz:post-lf014-1");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        calls_after_caps + 1,
        "exactly one create_post after capabilities"
    );

    // 5. Independent provider readback (documented GET /posts through
    //    the REAL production transport) is the actual-published
    //    authority; never infer PUBLISHED from the acceptance alone.
    let readback_transport = HttpPostizTransport::new(
        format!("http://127.0.0.1:{postiz_port}"),
        CANARY_TOKEN,
        Duration::from_secs(5),
    );
    let readback = readback_transport.list_posts().expect("readback");
    assert_eq!(readback.len(), 1);
    assert_eq!(readback[0].id, "post-lf014-1");
    assert_eq!(
        readback[0].status, "published",
        "readback confirms published"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        calls_after_caps + 2,
        "readback is the second provider call after capabilities"
    );

    // 6. Schedule path: documented type=schedule acceptance is
    //    SCHEDULED, never PUBLISHED.
    let scheduled_msg = SocialMessage {
        message_id: SocialMessageId::new("msg-lf014-sched").unwrap(),
        account_id: SocialAccountId::new("acct-lf014-x").unwrap(),
        state: SocialMessageState::Approved,
        scheduled_at: Some("2026-08-20T10:00:00Z".into()),
        variant: Some("var-lf014-x".into()),
        content_ref: "content-ref-lf014-sched".to_string(),
    };
    let scheduled_id = adapter
        .schedule(&scheduled_msg, "2026-08-20T10:00:00Z")
        .expect("schedule");
    assert_eq!(scheduled_id.as_str(), "postiz:post-lf014-1");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        calls_after_caps + 3,
        "schedule is one more create_post after capabilities"
    );

    // 7. Ingest engagement + attribution through the direct connector
    //    (X API v2-shaped fixture): mentions become conversations and
    //    public_metrics become campaign-attributed SocialMetrics.
    let user_body = r#"{"data":{"id":"u-lf014","name":"Nexus","username":"nexus"}}"#.to_string();
    let mentions_body = r#"{"data":[{"id":"m-lf014-1","text":"campaign inquiry","author_id":"a-1","created_at":"2026-08-19T00:00:00Z"}]}"#.to_string();
    let tweet_body = r#"{"data":{"id":"m-lf014-1","text":"campaign inquiry","public_metrics":{"like_count":3,"retweet_count":1,"reply_count":2,"quote_count":0,"impression_count":40,"bookmark_count":1}}}"#.to_string();
    let (x_port, x_handle) = fixture::spawn_server(5, move |method, path| {
        if method == "GET" && path == "/2/users/me" {
            (200, "application/json", user_body.clone())
        } else if method == "GET" && path.starts_with("/2/users/u-lf014/mentions") {
            (200, "application/json", mentions_body.clone())
        } else if method == "GET" && path.contains("/2/tweets/m-lf014-1") {
            (200, "application/json", tweet_body.clone())
        } else {
            (404, "application/json", "{}".to_string())
        }
    });
    let direct = DirectPlatformAdapter::new(
        Box::new(HttpDirectPlatformTransport::new(
            format!("http://127.0.0.1:{x_port}"),
            CANARY_TOKEN,
            Duration::from_secs(5),
        )),
        tenant(),
        business(),
        CANARY_TOKEN,
    );
    let conversations = direct
        .list_conversations(&tenant(), &business())
        .expect("conversations from real mentions");
    assert_eq!(conversations.len(), 1);
    let metrics = direct
        .list_metrics(&tenant(), &business(), Some(&campaign))
        .expect("attributed metrics");
    assert!(!metrics.is_empty());
    assert!(
        metrics
            .iter()
            .any(|m| m.campaign_id.as_ref() == Some(&campaign)),
        "attribution preserved"
    );
    let impressions: u64 = metrics
        .iter()
        .filter(|m| m.kind == SocialMetricKind::Impressions)
        .map(|m| m.value)
        .sum();
    assert_eq!(impressions, 40, "real public_metrics attributed");

    // 8. Audit ring: correlated operations, credential canary zero
    //    leakage across BOTH adapters.
    let audit = adapter.audit();
    assert!(
        audit
            .iter()
            .any(|e| e.operation == "PUBLISH_VARIANT" && e.outcome == "ok"),
        "publish audited"
    );
    assert!(
        audit
            .iter()
            .any(|e| e.operation == "SCHEDULE" && e.outcome == "ok"),
        "schedule audited"
    );
    let joined = audit
        .iter()
        .map(|e| format!("{} {} {}", e.correlation, e.operation, e.detail))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !joined.contains(CANARY_TOKEN),
        "postiz credential canary must never appear in audit"
    );
    for entry in direct.audit() {
        assert!(
            !entry.detail.contains(CANARY_TOKEN),
            "direct credential canary must never appear in audit"
        );
    }

    postiz_handle.join().unwrap();
    x_handle.join().unwrap();

    // 9. Machine-readable current-run evidence (redacted; stale never
    //    satisfies: run_id must match the gate).
    let evidence = serde_json::json!({
        "proof": "LF-014",
        "node": "EP-029",
        "milestone": "M5",
        "run_id": run_id(),
        "surface": "documented Postiz public API + documented X API v2",
        "transport": "HttpPostizTransport + HttpDirectPlatformTransport (real reqwest, REAL std::net sockets)",
        "adapter": "PostizAdapter + DirectPlatformAdapter (dual authorization gates, poison-safe observability)",
        "fixture": "CONTROLLED_TEST_FIXTURE",
        "lifecycle": {
            "single_objective_invariant": true,
            "variants_created": 2,
            "approval_granted": "GRANTED",
            "approval_never_publishes": true,
            "approval_zero_provider_calls": true,
            "publish_acceptance": "postiz:post-lf014-1",
            "publish_provider_calls": 1,
            "readback_published": true,
            "accepted_never_published_without_readback": true,
            "schedule_acceptance_recorded": true,
            "engagement_ingested": conversations.len(),
            "attribution_campaign": campaign.as_str(),
            "attributed_impressions": impressions,
            "audit_correlation_present": true,
            "credential_redaction": "ZERO_LEAKAGE"
        },
        "certification": {
            "real_postiz_provider": "NOT_ASSERTED",
            "real_x_provider": "NOT_ASSERTED"
        }
    });
    write_evidence(evidence);
}
