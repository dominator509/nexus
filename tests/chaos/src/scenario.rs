//! Canonical chaos scenario registry (EP-040 M5 fence section D/E).
//! Every scenario carries the M1 ChaosScenario safety model: target,
//! owner, preconditions, bounded blast radius, injection method,
//! expected failure class, observability requirement, recovery
//! assertion, timeout/budget, rollback/teardown, residue check, and
//! certification boundary.

use nexus_test_contract::model::ChaosScenario;
use nexus_test_contract::vocabulary::{BlastRadius, FailureInjectionKind};

/// Canonical EP-040 chaos scenario ids (fixed vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChaosScenarioId {
    /// Terminate a real provider container, then recover it.
    TerminateRecover,
    /// Point at an unavailable port (connection refused).
    PortRefusal,
    /// Silent peer accepts but never answers (bounded timeout).
    SilentPeer,
    /// Revoke a runtime credential mid-operation.
    CredentialRevocation,
    /// Corrupt controlled evidence bytes at the boundary.
    CorruptEvidence,
    /// Replay stale evidence bound to an old run_id/git_commit.
    StaleEvidence,
    /// Inject a temp-file leak and prove residue detection + cleanup.
    TempLeak,
    /// Zero-test collection must never be green.
    ZeroTestCollection,
    /// Skipped/ignored output must never be green.
    SkippedIgnored,
}

impl ChaosScenarioId {
    pub const VOCAB: &'static str = "EP-040 M5 chaos scenario id";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::TerminateRecover => "ep040-m5-terminate-recover",
            Self::PortRefusal => "ep040-m5-port-refusal",
            Self::SilentPeer => "ep040-m5-silent-peer",
            Self::CredentialRevocation => "ep040-m5-credential-revocation",
            Self::CorruptEvidence => "ep040-m5-corrupt-evidence",
            Self::StaleEvidence => "ep040-m5-stale-evidence",
            Self::TempLeak => "ep040-m5-temp-leak",
            Self::ZeroTestCollection => "ep040-m5-zero-test-collection",
            Self::SkippedIgnored => "ep040-m5-skipped-ignored",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::TerminateRecover,
            Self::PortRefusal,
            Self::SilentPeer,
            Self::CredentialRevocation,
            Self::CorruptEvidence,
            Self::StaleEvidence,
            Self::TempLeak,
            Self::ZeroTestCollection,
            Self::SkippedIgnored,
        ]
    }
}

impl std::fmt::Display for ChaosScenarioId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The full M5 chaos scenario catalog with complete M1 safety models.
/// All scenarios are EP-040-owned, Single blast radius, bounded budget,
/// with rollback path, cleanup assertion, expected failure class,
/// observability requirement, and recovery assertion.
pub fn chaos_scenarios() -> Vec<ChaosScenario> {
    let owner = "EP-040";
    let mut scenarios = Vec::new();

    let mut terminate = ChaosScenario::new(ChaosScenarioId::TerminateRecover.as_str(), owner);
    terminate.allowed_target = "nexus-ep040-m3-<nanos> provider container".to_string();
    terminate.injection = FailureInjectionKind::Terminate;
    terminate.blast_radius = BlastRadius::Single;
    terminate.timeout_budget_secs = 90;
    terminate.rollback_path = "docker start <container>; wait_ready".to_string();
    terminate.safety_preconditions = vec![
        "provider container is EP-040-owned".to_string(),
        "docker CLI available".to_string(),
    ];
    terminate.observability_requirement =
        "typed Unavailable observed on next connect after termination".to_string();
    terminate.expected_failure_class = "UNAVAILABLE".to_string();
    terminate.recovery_assertion =
        "after docker start, reconnect succeeds and roundtrip returns a row".to_string();
    terminate.cleanup_assertion = "container removed; zero EP-040-owned residue".to_string();
    terminate.prohibited_targets = vec![
        "shared graph infrastructure".to_string(),
        "EP-044 control plane".to_string(),
    ];
    scenarios.push(terminate);

    let mut refusal = ChaosScenario::new(ChaosScenarioId::PortRefusal.as_str(), owner);
    refusal.allowed_target = "reserved host port with no listener".to_string();
    refusal.injection = FailureInjectionKind::UnavailableDependency;
    refusal.blast_radius = BlastRadius::Single;
    refusal.timeout_budget_secs = 10;
    refusal.rollback_path = "no rollback needed (no listener was started)".to_string();
    refusal.safety_preconditions = vec!["port is not used by a retained fixture".to_string()];
    refusal.observability_requirement = "typed connection-refused failure observed".to_string();
    refusal.expected_failure_class = "UNAVAILABLE".to_string();
    refusal.recovery_assertion =
        "fails closed with typed Unavailable; no silent success".to_string();
    refusal.cleanup_assertion = "no sockets or temp files left".to_string();
    refusal.prohibited_targets = vec![
        "retained MinIO/SWF/GlitchTip ports".to_string(),
        "EP-044 ports".to_string(),
    ];
    scenarios.push(refusal);

    let mut silent = ChaosScenario::new(ChaosScenarioId::SilentPeer.as_str(), owner);
    silent.allowed_target = "ephemeral listener that never answers".to_string();
    silent.injection = FailureInjectionKind::Timeout;
    silent.blast_radius = BlastRadius::Single;
    silent.timeout_budget_secs = 15;
    silent.rollback_path = "drop the ephemeral listener".to_string();
    silent.safety_preconditions = vec!["listener bound on loopback only".to_string()];
    silent.observability_requirement = "typed Timeout observed within the budget".to_string();
    silent.expected_failure_class = "TIMEOUT".to_string();
    silent.recovery_assertion = "operation fails closed with typed Timeout; no hang".to_string();
    silent.cleanup_assertion = "ephemeral listener closed; zero residue".to_string();
    silent.prohibited_targets = vec!["shared graph infrastructure".to_string()];
    scenarios.push(silent);

    let mut revoke = ChaosScenario::new(ChaosScenarioId::CredentialRevocation.as_str(), owner);
    revoke.allowed_target = "runtime-generated sandbox credential".to_string();
    revoke.injection = FailureInjectionKind::RevokedToken;
    revoke.blast_radius = BlastRadius::Single;
    revoke.timeout_budget_secs = 10;
    revoke.rollback_path = "generate a fresh credential".to_string();
    revoke.safety_preconditions = vec!["credential is runtime-generated".to_string()];
    revoke.observability_requirement =
        "typed Authorization/PolicyDenied after revocation".to_string();
    revoke.expected_failure_class = "POLICY_DENIED".to_string();
    revoke.recovery_assertion = "revoked use is denied; a fresh credential works".to_string();
    revoke.cleanup_assertion = "no credential material written to disk".to_string();
    revoke.prohibited_targets = vec!["production credentials".to_string()];
    scenarios.push(revoke);

    let mut corrupt = ChaosScenario::new(ChaosScenarioId::CorruptEvidence.as_str(), owner);
    corrupt.allowed_target = "controlled serialized evidence bytes".to_string();
    corrupt.injection = FailureInjectionKind::CorruptMessage;
    corrupt.blast_radius = BlastRadius::Single;
    corrupt.timeout_budget_secs = 10;
    corrupt.rollback_path = "re-serialize evidence from canonical fields".to_string();
    corrupt.safety_preconditions = vec!["bytes are controlled test evidence".to_string()];
    corrupt.observability_requirement = "typed Verification failure on parse".to_string();
    corrupt.expected_failure_class = "SECURITY_FAILURE".to_string();
    corrupt.recovery_assertion = "corrupted evidence is never accepted as verified".to_string();
    corrupt.cleanup_assertion = "no corrupted evidence file retained".to_string();
    corrupt.prohibited_targets = vec!["real evidence store".to_string()];
    scenarios.push(corrupt);

    let mut stale = ChaosScenario::new(ChaosScenarioId::StaleEvidence.as_str(), owner);
    stale.allowed_target = "evidence record bound to old run_id/git_commit".to_string();
    stale.injection = FailureInjectionKind::MalformedInput;
    stale.blast_radius = BlastRadius::Single;
    stale.timeout_budget_secs = 10;
    stale.rollback_path = "write a fresh current-run record".to_string();
    stale.safety_preconditions = vec!["evidence root is EP-040-owned temp".to_string()];
    stale.observability_requirement = "stale record rejected (Verification)".to_string();
    stale.expected_failure_class = "SECURITY_FAILURE".to_string();
    stale.recovery_assertion = "current-run evidence verifies; stale never satisfies".to_string();
    stale.cleanup_assertion = "stale record removed with evidence root".to_string();
    stale.prohibited_targets = vec!["real evidence store".to_string()];
    scenarios.push(stale);

    let mut leak = ChaosScenario::new(ChaosScenarioId::TempLeak.as_str(), owner);
    leak.allowed_target = "EP-040-owned temp root".to_string();
    leak.injection = FailureInjectionKind::PartialSideEffect;
    leak.blast_radius = BlastRadius::Single;
    leak.timeout_budget_secs = 10;
    leak.rollback_path = "remove EP-040-owned temp root".to_string();
    leak.safety_preconditions = vec!["temp root under /tmp/ep040-m5-".to_string()];
    leak.observability_requirement = "residue detected and attributed to owned prefix".to_string();
    leak.expected_failure_class = "ENVIRONMENT".to_string();
    leak.recovery_assertion = "cleanup removes the owned root; zero residue".to_string();
    leak.cleanup_assertion = "zero /tmp/ep040-m5-* roots remain".to_string();
    leak.prohibited_targets = vec!["retained fixture dirs".to_string()];
    scenarios.push(leak);

    let mut zero = ChaosScenario::new(ChaosScenarioId::ZeroTestCollection.as_str(), owner);
    zero.allowed_target = "empty test collection summary".to_string();
    zero.injection = FailureInjectionKind::MalformedInput;
    zero.blast_radius = BlastRadius::Single;
    zero.timeout_budget_secs = 10;
    zero.rollback_path = "run a real non-empty suite".to_string();
    zero.safety_preconditions = vec!["no test binary is mutated".to_string()];
    zero.observability_requirement = "zero tests collected is never green".to_string();
    zero.expected_failure_class = "OWNER_CODE_REGRESSION".to_string();
    zero.recovery_assertion = "the real gate requires a non-zero pass count".to_string();
    zero.cleanup_assertion = "no temp output retained".to_string();
    zero.prohibited_targets = vec!["real test binaries".to_string()];
    scenarios.push(zero);

    let mut skipped = ChaosScenario::new(ChaosScenarioId::SkippedIgnored.as_str(), owner);
    skipped.allowed_target = "test summary with skipped/ignored entries".to_string();
    skipped.injection = FailureInjectionKind::MalformedInput;
    skipped.blast_radius = BlastRadius::Single;
    skipped.timeout_budget_secs = 10;
    skipped.rollback_path = "re-run without skips".to_string();
    skipped.safety_preconditions = vec!["no test binary is mutated".to_string()];
    skipped.observability_requirement = "skipped/ignored output is not green".to_string();
    skipped.expected_failure_class = "OWNER_CODE_REGRESSION".to_string();
    skipped.recovery_assertion = "the real gate rejects skipped/ignored required tests".to_string();
    skipped.cleanup_assertion = "no temp output retained".to_string();
    skipped.prohibited_targets = vec!["real test binaries".to_string()];
    scenarios.push(skipped);

    scenarios
}

/// Register (validate) every scenario; each must pass the M1 chaos
/// safety model before any injection runs.
pub fn register_chaos_scenarios(
) -> Result<Vec<ChaosScenario>, nexus_test_contract::error::TestingError> {
    let scenarios = chaos_scenarios();
    for scenario in &scenarios {
        scenario.validate()?;
    }
    Ok(scenarios)
}
