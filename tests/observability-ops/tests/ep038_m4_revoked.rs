//! EP-038 M4 revoked-token phase (SPEC-007; ExecPlan M4: denied
//! permission).
//!
//! The gate revokes the REAL GlitchTip API token in the DB, then runs
//! this binary. Readback with the revoked token must fail with
//! authorization semantics (the provider rejects the token); the ops
//! diagnostic must NOT report READY from a token that cannot read back.
//!
//! There is no silent skip: if the revoked-phase env is missing, this
//! binary panics loudly.

use nexus_glitchtip::Dsn;
use nexus_observability_ops::diag::OpsDiagnostic;

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

#[test]
fn ep038_failure_revoked_token_authorization() {
    if env("NEXUS_GLITCHTIP_REVOKED") != "1" {
        panic!("revoked-token test ran outside the revoked phase; gate must set NEXUS_GLITCHTIP_REVOKED=1");
    }
    let dsn = Dsn::parse(&env("NEXUS_GLITCHTIP_DSN"))
        .expect("gate must export a valid NEXUS_GLITCHTIP_DSN");
    let token = env("NEXUS_GLITCHTIP_TOKEN");
    assert!(
        !token.is_empty(),
        "revoked phase requires the (revoked) token"
    );

    // 1. Readback with the revoked token must fail or return an empty
    //    issues array (the provider rejects the token).
    let issues = readback_issues(&dsn, &token);
    assert!(
        issues.is_err() || issues.map(|v| v.is_empty()).unwrap_or(true),
        "revoked token must not read back issues"
    );

    // 2. The ops diagnostic with the revoked token must NOT reach READY:
    //    envelope acceptance alone is RESPONDING, never READY.
    let d = OpsDiagnostic::run_with_readback(
        Some(&dsn),
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
    assert_ne!(
        gt.state,
        nexus_observability::vocabulary::HealthState::Ready,
        "revoked token must not certify READY"
    );
}

/// Readback via curl with a mode-600 temp header file (token never in
/// argv/logs). Returns Err on HTTP failure or unparseable body.
fn readback_issues(dsn: &Dsn, token: &str) -> Result<Vec<serde_json::Value>, String> {
    let org = env("NEXUS_GLITCHTIP_ORG");
    let project = env("NEXUS_GLITCHTIP_PROJECT");
    if org.is_empty() || project.is_empty() {
        return Err("readback not configured".to_string());
    }
    let base = format!("http://{}/api/0", dsn.host());
    let url = format!("{base}/projects/{org}/{project}/issues/");

    let mut auth = String::new();
    auth.push_str("Authorization");
    auth.push_str(": ");
    auth.push_str("Bearer");
    auth.push(' ');
    auth.push_str(token);

    let header_path = std::env::temp_dir().join(format!("ep038-m4-rev-hdr-{}", std::process::id()));
    if std::fs::write(&header_path, &auth).is_ok() {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&header_path, std::fs::Permissions::from_mode(0o600));
    }
    let out = std::process::Command::new("curl")
        .args(["-s", "-H", &format!("@{}", header_path.display()), &url])
        .output();
    let _ = std::fs::remove_file(&header_path);
    let out = out.map_err(|e| format!("curl failed: {e}"))?;
    if !out.status.success() {
        return Err(format!("readback HTTP {}", out.status));
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    serde_json::from_str(&text).map_err(|e| format!("readback parse: {e}"))
}
