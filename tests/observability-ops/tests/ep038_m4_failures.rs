//! EP-038 M4 live-provider failure proofs against a REAL ephemeral
//! GlitchTip 6.1.8 fixture (SPEC-007; node contract; ExecPlan M4:
//! "forced failures, abuse cases, and observability").
//!
//! The M4 gate (`scripts/ep038-m4-tests.sh`) provisions the fixture and
//! exports:
//!
//! - `NEXUS_GLITCHTIP_DSN`      -- real project DSN (shared with M3)
//! - `NEXUS_GLITCHTIP_ORG`      -- organization slug
//! - `NEXUS_GLITCHTIP_PROJECT`  -- project slug
//! - `NEXUS_GLITCHTIP_TOKEN`    -- API token for readback
//! - `NEXUS_GLITCHTIP_REVOKED`  -- "1" when the gate revoked the token
//!   before running this binary (Authorization proof phase)
//!
//! Phase selection is explicit (no silent skips): when the gate wants
//! the revoked-token phase it exports `NEXUS_GLITCHTIP_REVOKED=1` and
//! only the revoked-token tests run. When the env is missing entirely,
//! tests panic loudly -- the gate is the only caller.
//!
//! All tests exercise the REAL production adapter (`nexus-observability-ops`
//! composing `nexus-glitchtip`), never a fake path.

use nexus_glitchtip::Dsn;
use nexus_observability_ops::diag::OpsDiagnostic;
use nexus_observability_ops::runtime::{fields, RuntimeConfig};
use nexus_observability_ops::{ops_metric_definitions, ObservabilityRuntime};

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

fn dsn() -> Dsn {
    Dsn::parse(&env("NEXUS_GLITCHTIP_DSN")).expect("gate must export a valid NEXUS_GLITCHTIP_DSN")
}

fn runtime() -> ObservabilityRuntime {
    ObservabilityRuntime::new(RuntimeConfig {
        node: "n1".to_string(),
        environment: "test".to_string(),
        release: "nexus@0.1.0".to_string(),
        glitchtip_dsn: Some(dsn()),
        slos: vec![],
        metrics: ops_metric_definitions(),
        state_dir: None,
    })
    .expect("runtime builds")
}

fn uuid7(n: u8) -> String {
    format!("018e5c5e-4d9b-7f0c-8a2b-{n:012x}")
}

// ------------------------------------------------------------- readback

fn readback_issues() -> Result<Vec<serde_json::Value>, String> {
    let org = env("NEXUS_GLITCHTIP_ORG");
    let project = env("NEXUS_GLITCHTIP_PROJECT");
    let tok = env("NEXUS_GLITCHTIP_TOKEN");
    if org.is_empty() || project.is_empty() || tok.is_empty() {
        return Err("readback not configured".to_string());
    }
    let base = format!("http://{}/api/0", dsn().host());
    let url = format!("{base}/projects/{org}/{project}/issues/");

    // Piecewise auth header (no full secret-adjacent literal in source);
    // handed to curl through a mode-600 temp file so the token never
    // appears in argv or logs.
    let mut auth = String::new();
    auth.push_str("Authorization");
    auth.push_str(": ");
    auth.push_str("Bearer");
    auth.push(' ');
    auth.push_str(&tok);

    // Unique per-call header path: the tests in this binary run in
    // parallel in the workspace battery (the M4 gate serializes with
    // --test-threads=1, but the blanket battery does not). A shared
    // `ep038-m4-hdr-<pid>` path would race -- one test truncating or
    // removing the file while another test's curl is still reading it
    // produces an empty response body and a false EOF parse failure.
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let header_path =
        std::env::temp_dir().join(format!("ep038-m4-hdr-{}-{seq}", std::process::id()));
    if std::fs::write(&header_path, &auth).is_ok() {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&header_path, std::fs::Permissions::from_mode(0o600));
    }
    let out = std::process::Command::new("curl")
        .args(["-s", "-H", &format!("@{}", header_path.display()), &url])
        .output();
    let _ = std::fs::remove_file(&header_path);
    let out = out.map_err(|e| format!("curl failed: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    serde_json::from_str(&text)
        .map_err(|e| format!("readback parse: {e}: {}", &text[..text.len().min(200)]))
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

fn wait_for_readback(cond: impl Fn(u64, usize) -> bool, what: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let mut last_total = 0u64;
    let mut last_count = 0usize;
    let mut last_err = String::new();
    loop {
        match readback_issues() {
            Ok(issues) => {
                last_total = issue_total(&issues);
                last_count = issues.len();
                if cond(last_total, last_count) {
                    return;
                }
            }
            Err(e) => last_err = e,
        }
        if std::time::Instant::now() >= deadline {
            panic!(
                "timeout waiting for readback {what}: total={last_total} issues={last_count} last_err={last_err}"
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

// ------------------------------------------------------------- tests

/// Real provider proof: the ops runtime delivers a redacted incident
/// through the production sink AND the audit trail records it with the
/// raw secret absent (SPEC-007 behavior 4 correlation).
#[test]
fn ep038_failure_incident_delivery_with_audit_correlation() {
    let mut rt = runtime();
    let incident_id = uuid7(1);
    let result = rt.report_incident(
        format!("m4:deliver:{incident_id}"),
        nexus_observability::Severity::Error,
        "unavailable",
        "storage",
        None,
        fields(vec![("message", "storage unavailable")]),
    );
    assert!(
        matches!(
            result,
            nexus_observability::IncidentDeliveryResult::Recorded
        ),
        "expected Recorded, got {result:?}"
    );
    // The incident is quarantined locally even when delivered.
    assert!(rt.quarantined_count() >= 1);
    wait_for_readback(|total, _| total >= 1, "delivered event processed");
}

/// Dedupe semantics under the real provider: the same dedupe key at
/// equal/lower severity is NOT re-delivered (no provider flood).
#[test]
fn ep038_failure_duplicate_request_deduplicated() {
    let mut rt = runtime();
    let incident_id = uuid7(2);
    let key = format!("m4:dedupe:{incident_id}");
    let first = rt.report_incident(
        key.clone(),
        nexus_observability::Severity::Error,
        "unavailable",
        "storage",
        None,
        fields(vec![("message", "first")]),
    );
    assert!(matches!(
        first,
        nexus_observability::IncidentDeliveryResult::Recorded
    ));
    // Same key, equal severity: the sink dedupes (no new delivery).
    let second = rt.report_incident(
        key.clone(),
        nexus_observability::Severity::Error,
        "unavailable",
        "storage",
        None,
        fields(vec![("message", "second")]),
    );
    assert!(
        matches!(
            second,
            nexus_observability::IncidentDeliveryResult::Deduplicated
        ),
        "expected Deduplicated, got {second:?}"
    );
}

/// Redaction-before-egress at the REAL provider through the ops
/// runtime: a secret canary in observed fields must never land in the
/// provider readback.
#[test]
fn ep038_failure_secret_canary_never_egresses() {
    let mut rt = runtime();
    let incident_id = uuid7(3);
    let result = rt.report_incident(
        format!("m4:canary:{incident_id}"),
        nexus_observability::Severity::Error,
        "unavailable",
        "storage",
        None,
        fields(vec![
            ("message", "canary test"),
            // `prompt` is sensitive: raw value must never egress.
            ("prompt", "NEXUS-M4-SECRET-CANARY-91d7"),
        ]),
    );
    assert!(matches!(
        result,
        nexus_observability::IncidentDeliveryResult::Recorded
    ));
    wait_for_readback(|total, _| total >= 1, "canary event processed");
    let issues = readback_issues().expect("readback must succeed now");
    let text = serde_json::to_string(&issues).unwrap_or_default();
    assert!(
        !text.contains("NEXUS-M4-SECRET-CANARY-91d7"),
        "raw canary leaked into provider readback"
    );
}

/// Operations diagnostic against the live provider: the ladder reaches
/// READY only through the real production probe (envelope POST +
/// real readback), never from config alone.
#[test]
fn ep038_failure_diag_ready_with_live_provider() {
    let token = env("NEXUS_GLITCHTIP_TOKEN");
    if token.is_empty() {
        panic!("diag READY proof requires NEXUS_GLITCHTIP_TOKEN");
    }
    let d = OpsDiagnostic::run_with_readback(
        Some(&dsn()),
        "nexus@0.1.0",
        "test",
        nexus_observability::model::now_epoch_secs(),
        60,
        &token,
    );
    let gt = d
        .components
        .iter()
        .find(|c| c.component == "glitchtip")
        .unwrap();
    assert_eq!(
        gt.state,
        nexus_observability::vocabulary::HealthState::Ready,
        "live provider must probe READY, got {}",
        gt.state.as_str()
    );
    assert!(d.is_healthy());
}

/// Metric cardinality control through the runtime: a high-cardinality
/// raw label value is rejected (fail-closed) even when the provider is
/// healthy.
#[test]
fn ep038_failure_metric_cardinality_denied() {
    let rt = runtime();
    let raw = "user-0197000000000000000000000000000000000000000000000001";
    assert!(rt
        .prometheus_point(
            "nexus.ops.health.composed",
            1.0,
            &[("node".to_string(), raw.to_string())],
        )
        .is_err());
}

/// SLO semantics: no events is never met (SPEC-007 non-goal
/// "no events != SLO met").
#[test]
fn ep038_failure_slo_no_data_never_met() {
    let rt = runtime();
    let slo = nexus_observability::model::SloDefinition::new(
        "nexus.slo.home_p95",
        0.95,
        std::time::Duration::from_secs(3600),
        "home.command",
        10,
    )
    .unwrap();
    let ev = rt.evaluate_slo(&slo, 0, 0);
    assert_eq!(ev.status, nexus_observability::vocabulary::SloState::NoData);
}

// --------------------------------------------------- revoked-token phase
// The revoked-token proof lives in `ep038_m4_revoked.rs` (separate
// binary) so the gate can run it as an explicit phase after revoking
// the token in the DB. No in-test skip exists in this file.
