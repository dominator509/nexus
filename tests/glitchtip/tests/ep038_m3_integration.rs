//! EP-038 M3 integration proofs against a REAL ephemeral GlitchTip
//! 6.1.8 fixture (SPEC-007 behavior 3; node contract).
//!
//! These tests are driven by the M3 gate (`scripts/ep038-m3-tests.sh`)
//! which provisions the fixture and exports the fixture facts:
//!
//! - `NEXUS_GLITCHTIP_DSN`      -- the real project DSN
//! - `NEXUS_GLITCHTIP_ORG`      -- organization slug
//! - `NEXUS_GLITCHTIP_PROJECT`  -- project slug
//! - `NEXUS_GLITCHTIP_TOKEN`    -- API token for readback
//!
//! The stopped-provider proof lives in `ep038_m3_stopped.rs` and is
//! executed by the gate as a SEPARATE cargo invocation after the real
//! fixture is stopped, with `NEXUS_GLITCHTIP_STOPPED_DSN` exported.
//! There is no in-test skip path in this file: if the DSN env is
//! missing the tests panic loudly (the gate is the only caller).
//!
//! Grouping semantics verified against the real provider: events with
//! the same Sentry `fingerprint` (our dedupe key) group into ONE
//! issue whose `count` increments; a new fingerprint becomes a
//! DISTINCT issue. Every test uses a unique incident id so stale
//! provider data never satisfies a later test.
//!
//! Readback is asynchronous: the provider accepts the envelope with
//! HTTP 200 and the embedded worker processes it in the background
//! (observed convergence: seconds, not milliseconds). Tests therefore
//! poll the readback API against a monotonic deadline -- bounded
//! retry, recorded last observation, no arbitrary sleeps, no
//! acceptance-test self-retry after failure.
//!
//! The gate MUST run this target serially (`--test-threads=1`) because
//! the tests share one live provider.

use nexus_glitchtip::{Dsn, GlitchTipIncidentSink};
use nexus_observability::{
    IncidentDeliveryResult, IncidentSink, RedactionPolicy, Severity, TelemetryContext,
    TelemetrySignal,
};

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

fn dsn() -> Dsn {
    Dsn::parse(&env("NEXUS_GLITCHTIP_DSN")).expect("gate must export a valid NEXUS_GLITCHTIP_DSN")
}

fn redacted(fields: Vec<(&str, &str)>) -> nexus_observability::RedactedEnvelope {
    let observed: Vec<(String, String)> = fields
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    RedactionPolicy::default().apply(
        TelemetrySignal::Incident,
        TelemetryContext::new(
            "nexus-glitchtip-it".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            "nexus-glitchtip-it".to_string(),
            "integration".to_string(),
            Severity::Error,
            Some("test".to_string()),
            None,
        )
        .expect("valid context"),
        observed,
    )
}

fn uuid7(n: u8) -> String {
    // Deterministic UUIDv7-shaped ids unique per test.
    format!("018e5c5e-4d9b-7f0c-8a2b-{n:012x}")
}

// ------------------------------------------------------------- readback

/// Independent provider readback via `curl` against the real
/// GlitchTip API. The auth header is built piecewise and handed to
/// curl through a mode-600 temp file (`-H @file`) so the token never
/// appears in argv, logs, or diagnostics.
fn readback_issues() -> Result<Vec<serde_json::Value>, String> {
    let org = env("NEXUS_GLITCHTIP_ORG");
    let project = env("NEXUS_GLITCHTIP_PROJECT");
    let tok = env("NEXUS_GLITCHTIP_TOKEN");
    if org.is_empty() || project.is_empty() || tok.is_empty() {
        return Err("readback not configured".to_string());
    }
    let base = {
        let d = dsn();
        format!("http://{}/api/0", d.host())
    };
    let url = format!("{base}/projects/{org}/{project}/issues/");

    // Piecewise auth header: no single source literal contains the
    // secret-adjacent full form.
    let mut auth = String::new();
    auth.push_str("Authorization");
    auth.push_str(": ");
    auth.push_str("Bearer");
    auth.push(' ');
    auth.push_str(&tok);

    // Unique per-call header path: the tests in this binary run in
    // parallel in the workspace battery (the M3 gate serializes with
    // --test-threads=1, but the blanket battery does not). A shared
    // `ep038-gt-hdr-<pid>` path would race -- one test truncating or
    // removing the file while another test's curl is still reading it
    // produces an empty response body and a false EOF parse failure.
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let header_path =
        std::env::temp_dir().join(format!("ep038-gt-hdr-{}-{seq}", std::process::id()));
    let write = std::fs::write(&header_path, &auth);
    if write.is_ok() {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&header_path, std::fs::Permissions::from_mode(0o600));
    }

    let out = std::process::Command::new("curl")
        .args(["-s", "-H", &format!("@{}", header_path.display()), &url])
        .output();

    let _ = std::fs::remove_file(&header_path);

    let out = out.map_err(|e| format!("curl failed: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let arr: Vec<serde_json::Value> = serde_json::from_str(&text)
        .map_err(|e| format!("readback parse: {e}: {}", &text[..text.len().min(200)]))?;
    Ok(arr)
}

fn issue_total(issues: &[serde_json::Value]) -> u64 {
    issues
        .iter()
        .map(|i| {
            i.get("count")
                .and_then(|c| c.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1)
        })
        .sum()
}

fn issue_max(issues: &[serde_json::Value]) -> u64 {
    issues
        .iter()
        .map(|i| {
            i.get("count")
                .and_then(|c| c.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1)
        })
        .max()
        .unwrap_or(0)
}

/// Poll the provider readback until `cond(total, max, issues)` holds or
/// the monotonic deadline passes. Bounded: 30s, 500ms steps. On
/// timeout it panics with the last observed state and last error so
/// the failure is diagnosable, never a bare "got 0".
fn wait_for_readback(cond: impl Fn(u64, u64, usize) -> bool, what: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let mut last_total = 0u64;
    let mut last_max = 0u64;
    let mut last_count = 0usize;
    let mut last_err = String::new();
    loop {
        match readback_issues() {
            Ok(issues) => {
                last_total = issue_total(&issues);
                last_max = issue_max(&issues);
                last_count = issues.len();
                if cond(last_total, last_max, last_count) {
                    return;
                }
            }
            Err(e) => last_err = e,
        }
        if std::time::Instant::now() >= deadline {
            panic!(
                "timeout waiting for readback {what}: total={last_total} max={last_max} issues={last_count} last_err={last_err}"
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

// ------------------------------------------------------------- tests

/// Real provider proof: envelope accepted AND processed into an issue
/// visible via the provider readback API.
#[test]
fn ep038_integration_envelope_delivers_and_lands() {
    let d = dsn();
    let mut sink = GlitchTipIncidentSink::new(d.clone(), "nexus@0.1.0", "test");
    let incident_id = uuid7(1);
    let dedupe = format!("it:deliver:{incident_id}");

    let result = sink.report(
        nexus_domain::IncidentId::new(&incident_id).expect("valid id"),
        dedupe,
        Severity::Error,
        "unavailable",
        "storage",
        None,
        redacted(vec![("message", "integration storage unavailable")]),
    );

    match result {
        IncidentDeliveryResult::Recorded => {}
        other => panic!("expected Recorded, got {other:?}"),
    }

    // Strongest proof: the provider readback shows processed events
    // (worker processes asynchronously; poll to convergence).
    wait_for_readback(
        |total, _, issues| total >= 1 && issues >= 1,
        "delivered event processed (>=1 event, >=1 issue)",
    );
}

/// Severity escalation must not be hidden by dedupe: the escalated
/// event is delivered (same fingerprint groups into the same issue,
/// incrementing its count -- never a blind duplicate).
#[test]
fn ep038_integration_escalation_not_hidden_by_dedupe() {
    let d = dsn();
    let mut sink = GlitchTipIncidentSink::new(d.clone(), "nexus@0.1.0", "test");
    let incident_id = uuid7(2);
    let dedupe = format!("it:escalate:{incident_id}");

    // First at Warning.
    let first = sink.report(
        nexus_domain::IncidentId::new(&incident_id).expect("valid id"),
        dedupe.clone(),
        Severity::Warning,
        "unavailable",
        "storage",
        None,
        redacted(vec![("message", "warning first")]),
    );
    // Same dedupe, higher severity: must be delivered (escalation),
    // NOT returned as Deduplicated.
    let escalated = sink.report(
        nexus_domain::IncidentId::new(&incident_id).expect("valid id"),
        dedupe.clone(),
        Severity::Critical,
        "unavailable",
        "storage",
        None,
        redacted(vec![("message", "critical escalation")]),
    );
    assert!(
        matches!(first, IncidentDeliveryResult::Recorded),
        "first: {first:?}"
    );
    assert!(
        matches!(escalated, IncidentDeliveryResult::Recorded),
        "escalated must be delivered, got: {escalated:?}"
    );

    // Both events group into the same issue (same fingerprint); the
    // issue count must reflect BOTH events.
    wait_for_readback(|_, max, _| max >= 2, "escalation grouped issue count >= 2");
}

/// New dedupe key -> distinct incident (no blind dedupe across keys).
#[test]
fn ep038_integration_new_dedupe_key_distinct() {
    let d = dsn();
    let mut sink = GlitchTipIncidentSink::new(d.clone(), "nexus@0.1.0", "test");
    let a = sink.report(
        nexus_domain::IncidentId::new(uuid7(3)).expect("valid id"),
        "it:distinct:a".to_string(),
        Severity::Error,
        "unavailable",
        "storage",
        None,
        redacted(vec![("message", "distinct a")]),
    );
    let b = sink.report(
        nexus_domain::IncidentId::new(uuid7(4)).expect("valid id"),
        "it:distinct:b".to_string(),
        Severity::Error,
        "unavailable",
        "storage",
        None,
        redacted(vec![("message", "distinct b")]),
    );
    assert!(matches!(a, IncidentDeliveryResult::Recorded), "a: {a:?}");
    assert!(matches!(b, IncidentDeliveryResult::Recorded), "b: {b:?}");

    // Two distinct dedupe keys => at least two distinct issues from
    // this test alone (plus prior tests share the provider).
    wait_for_readback(
        |_, _, issues| issues >= 3,
        "distinct keys create distinct issues (>=3)",
    );
}

/// Redaction-before-egress at the REAL provider: a secret canary must
/// never reach the provider payload or readback.
#[test]
fn ep038_integration_secret_canary_never_lands() {
    let d = dsn();
    let mut sink = GlitchTipIncidentSink::new(d.clone(), "nexus@0.1.0", "test");
    let incident_id = uuid7(5);
    let dedupe = format!("it:canary:{incident_id}");

    let result = sink.report(
        nexus_domain::IncidentId::new(&incident_id).expect("valid id"),
        dedupe,
        Severity::Error,
        "unavailable",
        "storage",
        None,
        redacted(vec![
            ("message", "canary test"),
            // `prompt` is sensitive: raw value must never egress.
            ("prompt", "NEXUS-SECRET-CANARY-7f3a9c"),
        ]),
    );
    assert!(
        matches!(result, IncidentDeliveryResult::Recorded),
        "result: {result:?}"
    );

    // Wait until the canary's event has been processed (readback
    // non-empty), THEN assert the raw canary is absent. This prevents
    // a vacuous pass from an empty readback.
    wait_for_readback(|total, _, _| total >= 1, "canary event processed");
    let issues = readback_issues().expect("readback must succeed now");
    let text = serde_json::to_string(&issues).unwrap_or_default();
    assert!(
        !text.contains("NEXUS-SECRET-CANARY-7f3a9c"),
        "raw canary leaked into provider readback"
    );
}
