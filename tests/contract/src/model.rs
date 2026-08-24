//! EP-040 testing/hardening/chaos contract models (SPEC-008; TESTING.md;
//! node contract interfaces).
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
//!
//! Every model fails closed: unknown vocabulary, missing evidence,
//! unbounded chaos, and vacuous gate proofs are rejected.

use std::collections::BTreeMap;

use nexus_domain::CorrelationId;
use serde::{Deserialize, Serialize};

use crate::error::{redact_secret_shaped, TestingError, TestingResult};
use crate::vocabulary::{
    BlastRadius, CertificationStatus, FailureInjectionKind, FlakeClassification,
    HardeningControlState, ResourceKind, TestLayer, TestOutcome,
};

/// A single test's evidence record. TEST EXISTS != TEST RAN: a record may
/// describe a test that was never executed. TEST RAN != BEHAVIOR VERIFIED:
/// running green is not the same as behavior verification against the
/// contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestEvidence {
    /// Canonical test id (owner crate + test name).
    pub test_id: String,
    /// Test layer.
    pub layer: TestLayer,
    /// Outcome of the most recent run, if any.
    pub outcome: Option<TestOutcome>,
    /// Flake classification if the run was flaky.
    pub flake_classification: Option<FlakeClassification>,
    /// Whether the test actually executed (TEST EXISTS != TEST RAN).
    pub executed: bool,
    /// Whether behavior was verified against the contract (TEST RAN !=
    /// BEHAVIOR VERIFIED).
    pub behavior_verified: bool,
    /// Whether the proof used a real production path (MOCK PASSED !=
    /// PRODUCTION PATH VERIFIED).
    pub production_path: bool,
    /// Correlation id for the run, if any.
    pub correlation: Option<CorrelationId>,
}

impl TestEvidence {
    pub fn new(test_id: impl Into<String>, layer: TestLayer) -> Self {
        Self {
            test_id: test_id.into(),
            layer,
            outcome: None,
            flake_classification: None,
            executed: false,
            behavior_verified: false,
            production_path: false,
            correlation: None,
        }
    }

    /// Record a completed run. Passing does not imply behavior verified.
    pub fn record_run(mut self, outcome: TestOutcome) -> Self {
        self.outcome = Some(outcome);
        self.executed = true;
        self
    }

    /// A test that ran and passed is not automatically behavior verified.
    pub fn is_green_but_unverified(&self) -> bool {
        self.executed && self.outcome == Some(TestOutcome::Passed) && !self.behavior_verified
    }

    /// A mock/fixture-only proof can never certify a production path.
    pub fn certify_production(&mut self) -> TestingResult<()> {
        if !self.executed {
            return Err(TestingError::zero_test_collection(
                "cannot certify a test that never ran",
            ));
        }
        if self.outcome != Some(TestOutcome::Passed) {
            return Err(TestingError::verification(format!(
                "cannot certify {} with outcome {:?}",
                self.test_id, self.outcome
            )));
        }
        if !self.production_path {
            return Err(TestingError::mock_only(
                "mock/fixture-only proof cannot certify a production path",
            ));
        }
        self.behavior_verified = true;
        Ok(())
    }
}

/// A gate result. ZERO TESTS COLLECTED != GREEN; SKIPPED/IGNORED required
/// tests are never passes; a vacuous proof is rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateResult {
    /// Canonical gate name.
    pub gate: String,
    /// Number of tests collected by the gate.
    pub collected: usize,
    /// Number of tests that passed.
    pub passed: usize,
    /// Number of tests that failed.
    pub failed: usize,
    /// Number of required tests skipped.
    pub skipped: usize,
    /// Number of required tests ignored.
    pub ignored: usize,
    /// Whether the gate ran against real evidence.
    pub evidence_bound: bool,
    /// Redacted evidence path(s) or run ids.
    pub evidence: Vec<String>,
}

impl GateResult {
    pub fn new(gate: impl Into<String>) -> Self {
        Self {
            gate: gate.into(),
            collected: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            ignored: 0,
            evidence_bound: false,
            evidence: Vec::new(),
        }
    }

    /// A gate is green only when it collected a non-zero number of tests,
    /// every collected test passed, no required test was skipped or
    /// ignored, and the proof is bound to real evidence.
    pub fn is_green(&self) -> bool {
        self.collected > 0
            && self.passed == self.collected
            && self.failed == 0
            && self.skipped == 0
            && self.ignored == 0
            && self.evidence_bound
    }

    /// Zero collected tests is never green.
    pub fn is_vacuous(&self) -> bool {
        self.collected == 0
    }
}

/// TestMatrix: the owned test inventory model. Maps every spec behavior to
/// a test path and enforces coverage/collection policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestMatrix {
    /// Owning node.
    pub owner_node: String,
    /// Layer -> ordered test ids that must run for this node.
    pub required: BTreeMap<TestLayer, Vec<String>>,
    /// Zero-test guard: an empty matrix fails closed.
    pub zero_test_guard: bool,
}

impl TestMatrix {
    pub fn new(owner_node: impl Into<String>) -> Self {
        Self {
            owner_node: owner_node.into(),
            required: BTreeMap::new(),
            zero_test_guard: true,
        }
    }

    pub fn add_required(mut self, layer: TestLayer, test_id: impl Into<String>) -> Self {
        self.required.entry(layer).or_default().push(test_id.into());
        self
    }

    /// Validate the matrix: owner present, non-zero required tests when
    /// the guard is on, no duplicate test ids within a layer.
    pub fn validate(&self) -> TestingResult<()> {
        if self.owner_node.trim().is_empty() {
            return Err(TestingError::validation("owner_node is required"));
        }
        if self.zero_test_guard && self.required.values().all(|v| v.is_empty()) {
            return Err(TestingError::zero_test_collection(
                "test matrix has zero required tests",
            ));
        }
        for (layer, ids) in &self.required {
            let mut seen = std::collections::HashSet::new();
            for id in ids {
                if id.trim().is_empty() {
                    return Err(TestingError::validation(format!(
                        "empty test id in layer {layer}"
                    )));
                }
                if !seen.insert(id) {
                    return Err(TestingError::validation(format!(
                        "duplicate test id {id} in layer {layer}"
                    )));
                }
            }
        }
        Ok(())
    }
}

/// A chaos scenario. CHAOS INJECTED != SYSTEM HARDENED; NO FAILURE
/// OBSERVED != RESILIENCE PROVEN. Every scenario requires a bounded blast
/// radius, a rollback path, a cleanup policy, and a declared expected
/// failure class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChaosScenario {
    /// Canonical scenario id.
    pub id: String,
    /// Owning node.
    pub owner_node: String,
    /// Exact allowed target (test, fixture, or service).
    pub allowed_target: String,
    /// Failure injection kind.
    pub injection: FailureInjectionKind,
    /// Bounded blast radius. GLOBAL is prohibited unless a later
    /// milestone explicitly owns it.
    pub blast_radius: BlastRadius,
    /// Timeout budget in seconds.
    pub timeout_budget_secs: u64,
    /// Rollback path that restores the target.
    pub rollback_path: String,
    /// Safety preconditions that must hold before injection.
    pub safety_preconditions: Vec<String>,
    /// Observability requirement for the injection window.
    pub observability_requirement: String,
    /// Expected failure class the scenario must produce.
    pub expected_failure_class: String,
    /// Recovery assertion that must hold after rollback.
    pub recovery_assertion: String,
    /// Cleanup assertion that must hold after teardown.
    pub cleanup_assertion: String,
    /// Prohibited target classes (never inject into these).
    pub prohibited_targets: Vec<String>,
}

impl ChaosScenario {
    pub fn new(id: impl Into<String>, owner_node: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            owner_node: owner_node.into(),
            allowed_target: String::new(),
            injection: FailureInjectionKind::UnavailableDependency,
            blast_radius: BlastRadius::Single,
            timeout_budget_secs: 0,
            rollback_path: String::new(),
            safety_preconditions: Vec::new(),
            observability_requirement: String::new(),
            expected_failure_class: String::new(),
            recovery_assertion: String::new(),
            cleanup_assertion: String::new(),
            prohibited_targets: Vec::new(),
        }
    }

    /// Validate the chaos safety model: owner, target, budget, rollback,
    /// cleanup, expected failure class, observability, and blast radius
    /// bound. No scenario is valid without a bounded blast radius and
    /// cleanup policy.
    pub fn validate(&self) -> TestingResult<()> {
        if self.id.trim().is_empty() {
            return Err(TestingError::validation("scenario id is required"));
        }
        if self.owner_node.trim().is_empty() {
            return Err(TestingError::validation("owner_node is required"));
        }
        if self.allowed_target.trim().is_empty() {
            return Err(TestingError::validation("allowed_target is required"));
        }
        if self.blast_radius == BlastRadius::Global {
            return Err(TestingError::policy(
                "GLOBAL blast radius is prohibited without explicit ownership",
            ));
        }
        if self.timeout_budget_secs == 0 {
            return Err(TestingError::validation("timeout budget is required"));
        }
        if self.rollback_path.trim().is_empty() {
            return Err(TestingError::rollback_unavailable(
                "chaos scenario requires a rollback path",
            ));
        }
        if self.cleanup_assertion.trim().is_empty() {
            return Err(TestingError::validation(
                "chaos scenario requires a cleanup assertion",
            ));
        }
        if self.expected_failure_class.trim().is_empty() {
            return Err(TestingError::validation(
                "chaos scenario requires an expected failure class",
            ));
        }
        if self.observability_requirement.trim().is_empty() {
            return Err(TestingError::validation(
                "chaos scenario requires an observability requirement",
            ));
        }
        Ok(())
    }
}

/// Hardening control. CONTROL DEFINED != CONTROL APPLIED != CONTROL
/// VERIFIED != CONTROL REGRESSED. A written control is not proof; only
/// verified controls with evidence count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardeningControl {
    /// Canonical control id.
    pub id: String,
    /// Current state on the ladder.
    pub state: HardeningControlState,
    /// Evidence reference that verified the control, if any.
    pub evidence_ref: Option<String>,
}

impl HardeningControl {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            state: HardeningControlState::Defined,
            evidence_ref: None,
        }
    }

    pub fn apply(mut self) -> Self {
        self.state = HardeningControlState::Applied;
        self
    }

    /// Verify only with evidence. A control without evidence can never be
    /// Verified.
    pub fn verify(mut self, evidence_ref: impl Into<String>) -> TestingResult<Self> {
        let evidence = evidence_ref.into();
        if evidence.trim().is_empty() {
            return Err(TestingError::missing_evidence(
                "hardening control requires evidence to verify",
            ));
        }
        self.state = HardeningControlState::Verified;
        self.evidence_ref = Some(evidence);
        Ok(self)
    }

    pub fn regress(mut self) -> Self {
        self.state = HardeningControlState::Regressed;
        self
    }

    /// A written/defined/applied control is never proof of hardening.
    pub fn is_proof(&self) -> bool {
        self.state == HardeningControlState::Verified && self.evidence_ref.is_some()
    }
}

/// Fixture ownership model: every fixture is owned by exactly one node,
/// uses a unique owned prefix, and declares its resource kinds and
/// teardown requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureOwnership {
    /// Owning node.
    pub owner_node: String,
    /// Unique owned resource prefix (e.g. nexus-ep040-*).
    pub owned_prefix: String,
    /// Resource kinds the fixture may own.
    pub resource_kinds: Vec<ResourceKind>,
    /// Whether teardown is required on success/failure/panic.
    pub teardown_required: bool,
}

impl FixtureOwnership {
    pub fn new(owner_node: impl Into<String>, owned_prefix: impl Into<String>) -> Self {
        Self {
            owner_node: owner_node.into(),
            owned_prefix: owned_prefix.into(),
            resource_kinds: Vec::new(),
            teardown_required: true,
        }
    }

    pub fn with_kind(mut self, kind: ResourceKind) -> Self {
        self.resource_kinds.push(kind);
        self
    }

    pub fn validate(&self) -> TestingResult<()> {
        if self.owner_node.trim().is_empty() {
            return Err(TestingError::validation("fixture owner_node is required"));
        }
        if !self.owned_prefix.starts_with("nexus-") {
            return Err(TestingError::validation(
                "owned prefix must start with nexus-",
            ));
        }
        if !self.teardown_required {
            return Err(TestingError::validation(
                "fixture teardown must be required",
            ));
        }
        Ok(())
    }
}

/// Resource residue record. RESOURCE CLEANUP ATTEMPTED != RESOURCE CLEAN:
/// only verified absence is clean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceResidue {
    /// Owning node.
    pub owner_node: String,
    /// Resource kind.
    pub kind: ResourceKind,
    /// Resource name.
    pub name: String,
    /// Whether teardown was attempted.
    pub cleanup_attempted: bool,
    /// Whether absence was verified after teardown.
    pub verified_clean: bool,
}

impl ResourceResidue {
    pub fn new(owner_node: impl Into<String>, kind: ResourceKind, name: impl Into<String>) -> Self {
        Self {
            owner_node: owner_node.into(),
            kind,
            name: name.into(),
            cleanup_attempted: false,
            verified_clean: false,
        }
    }

    pub fn attempt_cleanup(mut self) -> Self {
        self.cleanup_attempted = true;
        self
    }

    pub fn verify_clean(mut self) -> Self {
        self.verified_clean = true;
        self
    }

    /// Clean only when teardown was attempted AND absence verified.
    pub fn is_clean(&self) -> bool {
        self.cleanup_attempted && self.verified_clean
    }
}

/// Flaky-test record. FLAKE RETRIED GREEN != ROOT CAUSE FIXED: a retry may
/// classify a flake but never erases it; fixing requires a root cause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlakeRecord {
    /// Canonical test id.
    pub test_id: String,
    /// Classification.
    pub classification: FlakeClassification,
    /// Number of times the flake was retried green.
    pub retry_count: u32,
    /// Root cause, required before the flake can be considered fixed.
    pub root_cause: Option<String>,
}

impl FlakeRecord {
    pub fn new(test_id: impl Into<String>, classification: FlakeClassification) -> Self {
        Self {
            test_id: test_id.into(),
            classification,
            retry_count: 0,
            root_cause: None,
        }
    }

    /// A retry that goes green never fixes the flake by itself.
    pub fn retried_green(mut self) -> Self {
        self.retry_count += 1;
        self
    }

    pub fn fix(mut self, root_cause: impl Into<String>) -> TestingResult<Self> {
        let cause = root_cause.into();
        if cause.trim().is_empty() {
            return Err(TestingError::flake_unresolved(
                "root cause is required to fix a flake",
            ));
        }
        self.root_cause = Some(cause);
        Ok(self)
    }

    pub fn is_fixed(&self) -> bool {
        self.root_cause.is_some()
    }
}

/// Regression requirement: a required test/gate that must stay green.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegressionRequirement {
    /// Owning node.
    pub owner_node: String,
    /// Required test id.
    pub test_id: String,
    /// Gate that must observe it.
    pub gate: String,
}

impl RegressionRequirement {
    pub fn new(owner_node: impl Into<String>, test_id: impl Into<String>) -> Self {
        Self {
            owner_node: owner_node.into(),
            test_id: test_id.into(),
            gate: String::new(),
        }
    }

    pub fn with_gate(mut self, gate: impl Into<String>) -> Self {
        self.gate = gate.into();
        self
    }

    pub fn validate(&self) -> TestingResult<()> {
        if self.owner_node.trim().is_empty() {
            return Err(TestingError::validation(
                "regression owner_node is required",
            ));
        }
        if self.test_id.trim().is_empty() {
            return Err(TestingError::validation("regression test_id is required"));
        }
        if self.gate.trim().is_empty() {
            return Err(TestingError::validation("regression gate is required"));
        }
        Ok(())
    }
}

/// Provider certification suite. Provider and hardware certifications use
/// real controlled dependencies; mock-only evidence can never certify.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderCertificationSuite {
    /// Provider id.
    pub provider: String,
    /// Release profile.
    pub profile: String,
    /// Evidence paths or run ids.
    pub evidence: Vec<String>,
    /// Status.
    pub status: CertificationStatus,
}

impl ProviderCertificationSuite {
    pub fn new(provider: impl Into<String>, profile: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            profile: profile.into(),
            evidence: Vec::new(),
            status: CertificationStatus::NotAsserted,
        }
    }

    pub fn certify(mut self, evidence: Vec<String>) -> TestingResult<Self> {
        if evidence.is_empty() {
            return Err(TestingError::missing_evidence(
                "provider certification requires real controlled-dependency evidence",
            ));
        }
        for e in &evidence {
            if redact_secret_shaped(e) != *e {
                return Err(TestingError::validation(
                    "certification evidence must be redacted",
                ));
            }
        }
        self.evidence = evidence;
        self.status = CertificationStatus::Certified;
        Ok(self)
    }
}

/// Hardware certification suite. Physical model and firmware evidence is
/// required; vendor-family inference is never certification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardwareCertificationSuite {
    /// Hardware target id.
    pub target: String,
    /// Physical model.
    pub model: String,
    /// Firmware version.
    pub firmware: String,
    /// Evidence paths or run ids.
    pub evidence: Vec<String>,
    /// Status.
    pub status: CertificationStatus,
}

impl HardwareCertificationSuite {
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            model: String::new(),
            firmware: String::new(),
            evidence: Vec::new(),
            status: CertificationStatus::NotAsserted,
        }
    }

    pub fn certify(
        mut self,
        model: impl Into<String>,
        firmware: impl Into<String>,
        evidence: Vec<String>,
    ) -> TestingResult<Self> {
        let model = model.into();
        let firmware = firmware.into();
        if model.trim().is_empty() || firmware.trim().is_empty() {
            return Err(TestingError::missing_evidence(
                "hardware certification requires model and firmware",
            ));
        }
        if evidence.is_empty() {
            return Err(TestingError::missing_evidence(
                "hardware certification requires real physical evidence",
            ));
        }
        self.model = model;
        self.firmware = firmware;
        self.evidence = evidence;
        self.status = CertificationStatus::Certified;
        Ok(self)
    }
}

/// Accessibility audit record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessibilityAudit {
    /// Target surface.
    pub target: String,
    /// Declared standard (e.g. WCAG 2.1 AA).
    pub standard: String,
    /// Violation list.
    pub violations: Vec<String>,
    /// Status.
    pub status: CertificationStatus,
}

impl AccessibilityAudit {
    pub fn new(target: impl Into<String>, standard: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            standard: standard.into(),
            violations: Vec::new(),
            status: CertificationStatus::NotAsserted,
        }
    }

    pub fn validate(&self) -> TestingResult<()> {
        if self.target.trim().is_empty() {
            return Err(TestingError::validation("accessibility target is required"));
        }
        if self.standard.trim().is_empty() {
            return Err(TestingError::validation(
                "accessibility standard is required",
            ));
        }
        Ok(())
    }
}

/// Performance budget contract (SPEC-008; node contract). BUILD PASSED !=
/// RUNTIME SAFE: a performance budget is only met by real observed
/// evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerformanceBudget {
    /// Budget id.
    pub id: String,
    /// Owning node.
    pub owner_node: String,
    /// Metric name.
    pub metric: String,
    /// Upper bound (inclusive).
    pub max_value: f64,
    /// Unit label (e.g. ms, MiB, req/s).
    pub unit: String,
    /// Whether the budget was observed against a real run.
    pub observed: bool,
    /// Observed value, present only when observed.
    pub observed_value: Option<f64>,
}

impl PerformanceBudget {
    pub fn new(
        id: impl Into<String>,
        owner_node: impl Into<String>,
        metric: impl Into<String>,
        max_value: f64,
        unit: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            owner_node: owner_node.into(),
            metric: metric.into(),
            max_value,
            unit: unit.into(),
            observed: false,
            observed_value: None,
        }
    }

    pub fn observe(mut self, value: f64) -> Self {
        self.observed = true;
        self.observed_value = Some(value);
        self
    }

    /// A budget is met only when a real observed value is within the
    /// bound. Missing observation is never green.
    pub fn met(&self) -> bool {
        match self.observed_value {
            Some(v) => self.observed && v <= self.max_value,
            None => false,
        }
    }
}
