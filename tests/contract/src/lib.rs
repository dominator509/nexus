//! nexus-test-contract: EP-040 provider-neutral testing/hardening/chaos
//! contracts (SPEC-008; TESTING.md; node contract).
//!
//! This crate owns the canonical testing, hardening, and chaos model:
//! TestMatrix, ChaosScenario, ProviderCertificationSuite,
//! HardwareCertificationSuite, PerformanceBudget, AccessibilityAudit,
//! FlakyTestPolicy, plus the supporting evidence, gate, fixture, residue,
//! flake, and regression models.
//!
//! M1 is the contract layer only. No test runner, chaos injector,
//! certification harness, performance harness, or accessibility scanner is
//! asserted in M1; real injection, real certification, and real budgets
//! are NOT certified until later milestones.
//!
//! Permanent invariants encoded here and proven by tests:
//! - TEST EXISTS != TEST RAN
//! - TEST RAN != BEHAVIOR VERIFIED
//! - MOCK PASSED != PRODUCTION PATH VERIFIED
//! - CHAOS INJECTED != SYSTEM HARDENED
//! - NO FAILURE OBSERVED != RESILIENCE PROVEN
//! - ZERO TESTS COLLECTED != GREEN
//! - SKIPPED TEST != PASSED TEST
//! - FLAKE RETRIED GREEN != ROOT CAUSE FIXED
//! - RESOURCE CLEANUP ATTEMPTED != RESOURCE CLEAN
//! - BUILD PASSED != RUNTIME SAFE

pub mod error;
pub mod model;
pub mod port;
pub mod vocabulary;

pub use error::{redact_secret_shaped, TestingError, TestingErrorCode, TestingResult};
pub use model::{
    AccessibilityAudit, ChaosScenario, FixtureOwnership, FlakeRecord, GateResult, HardeningControl,
    PerformanceBudget, ProviderCertificationSuite, RegressionRequirement, ResourceResidue,
    TestEvidence, TestMatrix,
};
pub use port::{
    AccessibilityAuditPort, ChaosScenarioPort, EvidencePort, FlakyTestPolicyPort, GateRunnerPort,
    HardwareCertificationPort, PerformanceBudgetPort, ProviderCertificationPort, TestMatrixPort,
};
pub use vocabulary::{
    BlastRadius, CertificationStatus, FailureInjectionKind, FlakeClassification,
    HardeningControlState, ResourceKind, TestLayer, TestOutcome, VocabularyError,
};
