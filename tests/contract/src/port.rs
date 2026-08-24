//! EP-040 testing/hardening/chaos port surface (SPEC-008; node contract).
//!
//! Every interface is provider-neutral and versioned. Implementations in
//! later milestones add internal types but cannot alter these contracts.

use crate::error::TestingResult;
use crate::model::{
    AccessibilityAudit, ChaosScenario, FlakeRecord, GateResult, PerformanceBudget,
    ProviderCertificationSuite, TestEvidence, TestMatrix,
};

/// Test matrix port: validate the owned test inventory model.
pub trait TestMatrixPort {
    fn validate(&self, matrix: &TestMatrix) -> TestingResult<()>;
}

/// Chaos scenario port: validate the chaos safety model.
pub trait ChaosScenarioPort {
    fn validate(&self, scenario: &ChaosScenario) -> TestingResult<()>;
}

/// Gate runner port: collect and evaluate a gate result.
pub trait GateRunnerPort {
    fn run(&self, gate: &str) -> TestingResult<GateResult>;
}

/// Evidence port: record and certify test evidence.
pub trait EvidencePort {
    fn record(&self, evidence: TestEvidence) -> TestingResult<()>;
}

/// Provider certification suite port.
pub trait ProviderCertificationPort {
    fn certify(
        &self,
        suite: ProviderCertificationSuite,
    ) -> TestingResult<ProviderCertificationSuite>;
}

/// Hardware certification suite port.
pub trait HardwareCertificationPort {
    fn certify(
        &self,
        suite: super::model::HardwareCertificationSuite,
    ) -> TestingResult<super::model::HardwareCertificationSuite>;
}

/// Performance budget port.
pub trait PerformanceBudgetPort {
    fn evaluate(&self, budget: &PerformanceBudget) -> TestingResult<()>;
}

/// Accessibility audit port.
pub trait AccessibilityAuditPort {
    fn audit(&self, audit: &AccessibilityAudit) -> TestingResult<()>;
}

/// Flaky-test policy port: classify and fix flake records.
pub trait FlakyTestPolicyPort {
    fn classify(&self, record: &FlakeRecord) -> TestingResult<()>;
}
