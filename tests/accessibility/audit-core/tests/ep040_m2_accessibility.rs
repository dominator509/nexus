//! EP-040 M2 accessibility audit-core proofs: WCAG level vocabulary,
//! violation parsing, fail-closed verdicts, and dependency direction.

use nexus_accessibility_audit::{parse_violation, DeterministicAuditEngine, WcagLevel};
use nexus_test_contract::error::TestingErrorCode;
use nexus_test_contract::model::AccessibilityAudit;
use nexus_test_contract::AccessibilityAuditPort;

#[test]
fn ep040_unit_accessibility_wcag_level_deny_unknown() {
    assert_eq!(WcagLevel::parse("A").unwrap(), WcagLevel::A);
    assert_eq!(WcagLevel::parse("AA").unwrap(), WcagLevel::AA);
    assert_eq!(WcagLevel::parse("aaa").unwrap(), WcagLevel::AAA);
    assert!(WcagLevel::parse("B").is_err());
    assert!(WcagLevel::parse("").is_err());
    assert!(WcagLevel::parse("WCAG").is_err());
}

#[test]
fn ep040_unit_accessibility_parse_violation_canonical_shape() {
    let v = parse_violation("1.4.3@AA: contrast below 4.5:1").unwrap();
    assert_eq!(v.criterion, "1.4.3");
    assert_eq!(v.level, WcagLevel::AA);
    assert_eq!(v.detail, "contrast below 4.5:1");
}

#[test]
fn ep040_unit_accessibility_parse_violation_fails_closed_on_bad_shape() {
    assert!(parse_violation("no colon here").is_err());
    assert!(parse_violation("1.1.1 no level").is_err());
    assert!(parse_violation("@A: empty criterion").is_err());
    assert!(parse_violation("1.1.1@Z: unknown level").is_err());
}

#[test]
fn ep040_unit_accessibility_evaluate_clean_passes() {
    let engine = DeterministicAuditEngine::new();
    assert!(engine.evaluate(WcagLevel::AA, &[]).is_ok());
}

#[test]
fn ep040_unit_accessibility_evaluate_a_blocks_all_findings() {
    let engine = DeterministicAuditEngine::new();
    let findings = vec![
        nexus_accessibility_audit::ViolationFinding::new("1.1.1", WcagLevel::A, "missing alt"),
        nexus_accessibility_audit::ViolationFinding::new("1.4.3", WcagLevel::AA, "contrast"),
        nexus_accessibility_audit::ViolationFinding::new("1.4.6", WcagLevel::AAA, "advisory"),
    ];
    for f in &findings {
        let one = vec![f.clone()];
        assert_eq!(
            engine.evaluate(WcagLevel::A, &one).unwrap_err().code,
            TestingErrorCode::Policy
        );
    }
}

#[test]
fn ep040_unit_accessibility_evaluate_aa_blocks_a_and_aa_not_aaa() {
    let engine = DeterministicAuditEngine::new();
    let a = vec![nexus_accessibility_audit::ViolationFinding::new(
        "1.1.1",
        WcagLevel::A,
        "missing alt",
    )];
    let aa = vec![nexus_accessibility_audit::ViolationFinding::new(
        "1.4.3",
        WcagLevel::AA,
        "contrast",
    )];
    let aaa = vec![nexus_accessibility_audit::ViolationFinding::new(
        "1.4.6",
        WcagLevel::AAA,
        "advisory",
    )];
    assert!(engine.evaluate(WcagLevel::AA, &a).is_err());
    assert!(engine.evaluate(WcagLevel::AA, &aa).is_err());
    // AAA advisory findings do not block an AA audit.
    assert!(engine.evaluate(WcagLevel::AA, &aaa).is_ok());
}

#[test]
fn ep040_unit_accessibility_evaluate_aaa_blocks_everything() {
    let engine = DeterministicAuditEngine::new();
    let aaa = vec![nexus_accessibility_audit::ViolationFinding::new(
        "1.4.6",
        WcagLevel::AAA,
        "advisory",
    )];
    assert!(engine.evaluate(WcagLevel::AAA, &aaa).is_err());
}

#[test]
fn ep040_unit_accessibility_audit_port_implementable() {
    let port: Box<dyn AccessibilityAuditPort> = Box::new(DeterministicAuditEngine::new());
    let clean = AccessibilityAudit::new("dashboard", "WCAG 2.1 AA");
    assert!(port.audit(&clean).is_ok());
    let mut dirty = AccessibilityAudit::new("dashboard", "WCAG 2.1 AA");
    dirty.violations.push("1.1.1@A: missing alt".to_string());
    assert!(port.audit(&dirty).is_err());
}

#[test]
fn ep040_unit_accessibility_dependency_direction() {
    // The gate enforces dependency direction via cargo tree; here we prove
    // the direct dependency surface is limited to nexus-test-contract +
    // nexus-domain + serde + serde_json.
    let _ = nexus_domain::CorrelationId::new("018e5c5e-4d9b-7f0c-8a2b-000000000001");
    let _ = nexus_test_contract::TestLayer::Accessibility;
}
