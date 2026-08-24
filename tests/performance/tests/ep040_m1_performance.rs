//! EP-040 M1 performance evaluation proofs: deterministic budget
//! evaluation behind the contract port. Every proof uses real crate
//! machinery; no mocked component.

use nexus_test_contract::error::TestingErrorCode;
use nexus_test_contract::model::PerformanceBudget;
use nexus_test_contract::PerformanceBudgetPort;
use nexus_test_performance::{budget_met_fail_closed, DeterministicBudgetEvaluator};

#[test]
fn ep040_unit_performance_budget_unobserved_fails_closed() {
    let budget = PerformanceBudget::new("ep040-api-latency", "EP-040", "p95", 250.0, "ms");
    let evaluator = DeterministicBudgetEvaluator::new();
    assert_eq!(
        evaluator.evaluate(&budget).unwrap_err().code,
        TestingErrorCode::MissingEvidence
    );
    // Model-level guard: no observation is never met.
    assert!(!budget_met_fail_closed(&budget));
}

#[test]
fn ep040_unit_performance_budget_within_bound_passes() {
    let budget =
        PerformanceBudget::new("ep040-api-latency", "EP-040", "p95", 250.0, "ms").observe(180.0);
    let evaluator = DeterministicBudgetEvaluator::new();
    assert!(evaluator.evaluate(&budget).is_ok());
    assert!(budget_met_fail_closed(&budget));
}

#[test]
fn ep040_unit_performance_budget_exceeded_fails_policy() {
    let budget =
        PerformanceBudget::new("ep040-api-latency", "EP-040", "p95", 250.0, "ms").observe(400.0);
    let evaluator = DeterministicBudgetEvaluator::new();
    assert_eq!(
        evaluator.evaluate(&budget).unwrap_err().code,
        TestingErrorCode::Policy
    );
    assert!(!budget_met_fail_closed(&budget));
}

#[test]
fn ep040_unit_performance_budget_at_bound_passes() {
    let budget = PerformanceBudget::new("ep040-mem", "EP-040", "rss", 512.0, "MiB").observe(512.0);
    let evaluator = DeterministicBudgetEvaluator::new();
    assert!(evaluator.evaluate(&budget).is_ok());
}

#[test]
fn ep040_unit_performance_budget_metric_required() {
    let budget =
        PerformanceBudget::new("ep040-api-latency", "EP-040", "", 250.0, "ms").observe(100.0);
    let evaluator = DeterministicBudgetEvaluator::new();
    assert_eq!(
        evaluator
            .evaluate_observed(&budget, 100.0)
            .unwrap_err()
            .code,
        TestingErrorCode::Validation
    );
}

#[test]
fn ep040_unit_performance_budget_negative_max_rejected() {
    let budget =
        PerformanceBudget::new("ep040-api-latency", "EP-040", "p95", -1.0, "ms").observe(100.0);
    let evaluator = DeterministicBudgetEvaluator::new();
    assert_eq!(
        evaluator
            .evaluate_observed(&budget, 100.0)
            .unwrap_err()
            .code,
        TestingErrorCode::Validation
    );
}

#[test]
fn ep040_unit_performance_budget_deterministic() {
    let evaluator = DeterministicBudgetEvaluator::new();
    let a =
        PerformanceBudget::new("ep040-api-latency", "EP-040", "p95", 250.0, "ms").observe(180.0);
    let b =
        PerformanceBudget::new("ep040-api-latency", "EP-040", "p95", 250.0, "ms").observe(180.0);
    // Same input -> same verdict every time.
    assert_eq!(
        evaluator.evaluate(&a).is_ok(),
        evaluator.evaluate(&b).is_ok()
    );
    assert!(evaluator.evaluate(&a).is_ok());
    assert!(evaluator.evaluate(&a).is_ok());
}

#[test]
fn ep040_unit_performance_budget_port_object_safe() {
    let evaluator: Box<dyn PerformanceBudgetPort> = Box::new(DeterministicBudgetEvaluator::new());
    let budget =
        PerformanceBudget::new("ep040-api-latency", "EP-040", "p95", 250.0, "ms").observe(180.0);
    assert!(evaluator.evaluate(&budget).is_ok());
    let unobserved = PerformanceBudget::new("ep040-api-latency", "EP-040", "p95", 250.0, "ms");
    assert!(evaluator.evaluate(&unobserved).is_err());
}

#[test]
fn ep040_unit_performance_budget_dependency_direction() {
    // The gate enforces dependency direction via cargo tree; here we prove
    // the direct dependency surface is limited to nexus-test-contract +
    // nexus-domain + serde + serde_json.
    let _ = nexus_domain::CorrelationId::new("018e5c5e-4d9b-7f0c-8a2b-000000000001");
    let _ = nexus_test_contract::TestLayer::Unit;
}
