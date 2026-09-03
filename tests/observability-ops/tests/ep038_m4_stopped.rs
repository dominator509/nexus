//! EP-038 M4 stopped-provider phase (SPEC-007; ExecPlan M4: unavailable
//! dependency, exhausted budget).
//!
//! The gate stops the REAL GlitchTip container, then runs this binary
//! with `NEXUS_GLITCHTIP_STOPPED_DSN` set. The runtime's incident
//! delivery must classify the failure (refused -> Unavailable) and
//! retain the incident for bounded recovery; the bounded recovery must
//! fail closed when the provider stays down (budget exhausted), never
//! hang, never leak the DSN key.
//!
//! There is no silent skip: if the stopped-phase env is missing, this
//! binary panics loudly.

use nexus_glitchtip::Dsn;
use nexus_observability_ops::recovery::{recover_with_budget, RecoveryBudget};
use nexus_observability_ops::runtime::{fields, RuntimeConfig};
use nexus_observability_ops::{ops_metric_definitions, ObservabilityRuntime};

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

#[test]
fn ep038_failure_stopped_provider_unavailable_and_budget_exhausted() {
    let stopped_dsn = env("NEXUS_GLITCHTIP_STOPPED_DSN");
    if stopped_dsn.is_empty() {
        panic!(
            "stopped-provider phase requires NEXUS_GLITCHTIP_STOPPED_DSN (gate must set it before running this binary)"
        );
    }
    let dsn = Dsn::parse(&stopped_dsn).expect("valid stopped DSN");

    // 1. Delivery against the stopped provider classifies Unavailable.
    let mut rt = ObservabilityRuntime::new(RuntimeConfig {
        node: "n1".to_string(),
        environment: "test".to_string(),
        release: "nexus@0.1.0".to_string(),
        glitchtip_dsn: Some(dsn),
        slos: vec![],
        metrics: ops_metric_definitions(),
        state_dir: None,
    })
    .expect("runtime builds");

    let result = rt.report_incident(
        "m4:stopped:probe".to_string(),
        nexus_observability::Severity::Error,
        "unavailable",
        "storage",
        None,
        fields(vec![("message", "stopped provider")]),
    );
    match &result {
        nexus_observability::IncidentDeliveryResult::Failed { reason } => {
            assert!(
                reason.contains("Unavailable"),
                "stopped provider must classify Unavailable, got: {reason}"
            );
        }
        other => panic!("expected Failed(Unavailable), got {other:?}"),
    }
    // The incident is retained (quarantined) for recovery.
    assert!(rt.quarantined_count() >= 1, "incident must be quarantined");

    // 2. Bounded recovery against the still-stopped provider: budget
    //    exhausted, fail closed, truthful last failure, no hang.
    let budget = RecoveryBudget {
        max_elapsed: std::time::Duration::from_secs(10),
        max_attempts: 3,
        backoff: std::time::Duration::from_millis(200),
    };
    let outcome = rt.recover_incident(&budget);
    assert!(
        !outcome.recovered,
        "recovery must not succeed while provider is stopped"
    );
    assert!(
        outcome.budget_exhausted || outcome.attempts >= 1,
        "recovery must fail closed with attempts recorded"
    );
    assert!(
        outcome.last_failure.is_some(),
        "recovery must record last observed failure"
    );
    let describe = outcome.describe();
    assert!(
        !describe.contains("0123456789abcdef0123456789abcdef"),
        "recovery description must not leak the DSN key"
    );

    // 3. Generic bounded recovery helper also fails closed.
    let generic = recover_with_budget(
        &budget,
        Box::new(
            || nexus_observability_ops::recovery::RecoveryVerdict::Retryable {
                detail: "still down".to_string(),
            },
        ),
    );
    assert!(!generic.recovered);
    assert!(generic.budget_exhausted);

    println!("EP-038 M4 stopped-phase: ok (Unavailable + budget fail-closed)");
}
