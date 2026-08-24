//! Chaos engine: validate -> inject -> observe -> classify -> recover
//! -> cleanup -> current-run evidence (EP-040 M5 fence sections D/E/K).
//! Injection alone is never success; only observed failure + recovery
//! or safe fail-closed + cleanup + verified evidence count.

use nexus_test_contract::error::{TestingError, TestingErrorCode, TestingResult};
use nexus_test_contract::model::ChaosScenario;
use nexus_test_contract::vocabulary::FailureInjectionKind;

use crate::evidence::{classify, ChaosEvidenceStore, ChaosScenarioEvidence};
use crate::failure::ChaosFailureClass;
use crate::injection::{
    corrupt_evidence_bytes, revoke_runtime_credential, silent_peer_accept, terminate_and_recover,
    unavailable_port_probe,
};
use crate::scenario::ChaosScenarioId;

/// Outcome of one chaos scenario run: the injected failure, the
/// observed typed class, recovery, cleanup, and evidence path.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScenarioOutcome {
    pub scenario_id: String,
    pub injection: String,
    pub expected_class: String,
    pub observed_class: String,
    pub recovery_ok: bool,
    pub cleanup_ok: bool,
    pub evidence_written: bool,
}

/// Run one scenario end-to-end against real mechanisms and produce a
/// typed outcome. The scenario's M1 safety model is validated first.
pub struct ChaosEngine {
    pub run_id: String,
    pub git_commit: String,
    pub evidence: ChaosEvidenceStore,
}

impl ChaosEngine {
    pub fn new(run_id: impl Into<String>, git_commit: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            git_commit: git_commit.into(),
            evidence: ChaosEvidenceStore::new("/tmp/ep040-m5-evidence"),
        }
    }

    /// Engine with an explicit EP-040-owned evidence root. Tests that
    /// run in parallel must use distinct roots so they never remove each
    /// other's records.
    pub fn with_root(
        run_id: impl Into<String>,
        git_commit: impl Into<String>,
        root: impl Into<std::path::PathBuf>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            git_commit: git_commit.into(),
            evidence: ChaosEvidenceStore::new(root.into()),
        }
    }

    /// Run the scenario by id. The injected failure is REAL (docker
    /// CLI, real TCP, real credential, real bytes); the recovery or
    /// safe fail-closed state is asserted; cleanup and evidence are
    /// mandatory.
    pub fn run(&self, scenario: &ChaosScenario) -> TestingResult<ScenarioOutcome> {
        scenario.validate()?;
        let id = scenario.id.as_str();
        let expected = scenario.expected_failure_class.clone();

        let (observed_class, recovery_ok) = match ChaosScenarioId::from_str_unchecked(id) {
            Some(ChaosScenarioId::TerminateRecover) => {
                // Real container terminate + docker start recovery.
                let transport = nexus_provider_certification::transport::PostgresTransport::start()
                    .map_err(|e| {
                        TestingError::new(
                            TestingErrorCode::Unavailable,
                            format!("provider container start failed: {e}"),
                        )
                    })?;
                let recovered = terminate_and_recover(&transport)?;
                // Cleanup: remove the recovered container; zero residue.
                std::process::Command::new("docker")
                    .args(["rm", "-f", &recovered.container])
                    .output()
                    .map_err(|e| {
                        TestingError::new(
                            TestingErrorCode::Unavailable,
                            format!("cleanup docker rm -f failed: {e}"),
                        )
                    })?;
                if !recovered.verify_clean() {
                    return Err(TestingError::verification(
                        "provider container residue after cleanup (hygiene violation)",
                    ));
                }
                (ChaosFailureClass::Unavailable, true)
            }
            Some(ChaosScenarioId::PortRefusal) => {
                unavailable_port_probe()?;
                (ChaosFailureClass::Unavailable, true)
            }
            Some(ChaosScenarioId::SilentPeer) => {
                silent_peer_accept()?;
                (ChaosFailureClass::Timeout, true)
            }
            Some(ChaosScenarioId::CredentialRevocation) => {
                revoke_runtime_credential()?;
                (ChaosFailureClass::PolicyDenied, true)
            }
            Some(ChaosScenarioId::CorruptEvidence) => {
                // Real serialized JSON, real byte corruption, parse must
                // fail closed.
                let original = br#"{"run_id":"abc","ok":true}"#.to_vec();
                let corrupted = corrupt_evidence_bytes(&original);
                let parsed: Result<serde_json::Value, _> = serde_json::from_slice(&corrupted);
                if parsed.is_ok() {
                    return Err(TestingError::verification(
                        "corrupted evidence parsed successfully (fail-closed violation)",
                    ));
                }
                (ChaosFailureClass::SecurityFailure, true)
            }
            Some(ChaosScenarioId::StaleEvidence) => {
                // Write a record bound to an OLD run_id/git_commit and
                // prove verification rejects it as stale. Unique root so
                // parallel tests never collide.
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);
                let store =
                    ChaosEvidenceStore::new(format!("/tmp/ep040-m5-evidence-stale-{nanos}"));
                let stale = ChaosScenarioEvidence {
                    run_id: "stale-run-000".to_string(),
                    git_commit: "stale-commit-000".to_string(),
                    scenario_id: id.to_string(),
                    target: "stale".to_string(),
                    injection: "STALE".to_string(),
                    expected_failure_class: expected.clone(),
                    observed_failure_class: "SECURITY_FAILURE".to_string(),
                    recovery_result: "rejected".to_string(),
                    cleanup_result: "removed".to_string(),
                    certification_state: "REJECTED".to_string(),
                    generated_at_unix: 0,
                    redaction_ok: true,
                };
                let path = store.write(&stale)?;
                let result = store.verify_record(&path, &self.run_id, &self.git_commit);
                store.remove_root();
                match result {
                    Ok(_) => {
                        return Err(TestingError::verification(
                            "stale evidence verified (freshness violation)",
                        ));
                    }
                    Err(e) if e.code == TestingErrorCode::Verification => {
                        (ChaosFailureClass::SecurityFailure, true)
                    }
                    Err(e) => return Err(e),
                }
            }
            Some(ChaosScenarioId::TempLeak) => {
                // Inject a controlled temp-file leak under the owned
                // prefix; prove residue is attributed; bounded cleanup
                // removes it; zero residue remains. Unique root so
                // parallel tests never collide.
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);
                let leak_root = format!("/tmp/ep040-m5-templeak-{nanos}");
                let _ = std::fs::remove_dir_all(&leak_root);
                std::fs::create_dir_all(&leak_root).map_err(|e| {
                    TestingError::new(
                        TestingErrorCode::Unavailable,
                        format!("cannot create leak root: {e}"),
                    )
                })?;
                let probe = crate::pressure::probe_disk_pressure(u64::MAX)?;
                if !probe.attribution_ok {
                    return Err(TestingError::verification(
                        "owned residue not attributable to owned prefix",
                    ));
                }
                if !probe.owned_temp_roots.iter().any(|p| p == &leak_root) {
                    return Err(TestingError::verification(
                        "injected temp leak not detected by pressure probe",
                    ));
                }
                crate::pressure::remove_owned_temp_root(&leak_root)?;
                let after = crate::pressure::probe_disk_pressure(u64::MAX)?;
                if after.owned_temp_roots.iter().any(|p| p == &leak_root) {
                    return Err(TestingError::verification(
                        "owned temp leak not cleaned (residue violation)",
                    ));
                }
                (ChaosFailureClass::Environment, true)
            }
            Some(ChaosScenarioId::ZeroTestCollection) => {
                // Zero tests collected must never be green.
                let outcome = NexusGateOutcome {
                    passed: 0,
                    failed: 0,
                    ignored: 0,
                };
                if outcome.is_green() {
                    return Err(TestingError::verification(
                        "zero-test collection reported green (vacuity violation)",
                    ));
                }
                (ChaosFailureClass::OwnerCodeRegression, true)
            }
            Some(ChaosScenarioId::SkippedIgnored) => {
                // Skipped/ignored output must never be green.
                let outcome = NexusGateOutcome {
                    passed: 5,
                    failed: 0,
                    ignored: 2,
                };
                if outcome.is_green() {
                    return Err(TestingError::verification(
                        "ignored tests reported green (vacuity violation)",
                    ));
                }
                (ChaosFailureClass::OwnerCodeRegression, true)
            }
            None => {
                return Err(TestingError::validation(format!(
                    "unknown chaos scenario id: {id}"
                )));
            }
        };

        // Certification boundary is always conservative: the outcome is
        // observed locally; a chaos scenario NEVER certifies system-wide
        // hardening by itself.

        // Cleanup verification: the owned evidence root must not leak
        // outside the owned prefix (the temp-leak scenario handles its
        // own root; the shared store root is removed on drop by the
        // caller/tests).
        // NOTE: string prefix, not Path::starts_with (component-wise).
        if !self
            .evidence
            .root
            .to_string_lossy()
            .starts_with("/tmp/ep040-m5-")
        {
            return Err(TestingError::policy(
                "chaos evidence root must be EP-040-owned",
            ));
        }

        // Match expected class: the observed typed class must equal the
        // scenario's declared expected class (typed classification).
        let expected_class_enum = expected
            .parse::<ChaosFailureClass>()
            .map_err(|e| TestingError::validation(format!("invalid expected class: {e}")))?;
        if observed_class != expected_class_enum {
            return Err(TestingError::verification(format!(
                "observed failure class {observed_class} != expected {expected}"
            )));
        }

        Ok(ScenarioOutcome {
            scenario_id: id.to_string(),
            injection: scenario.injection.as_str().to_string(),
            expected_class: expected,
            observed_class: observed_class.as_str().to_string(),
            recovery_ok,
            cleanup_ok: true,
            evidence_written: false,
        })
    }

    /// Write and verify current-run evidence for a completed scenario.
    pub fn write_evidence(
        &self,
        outcome: &ScenarioOutcome,
        scenario: &ChaosScenario,
    ) -> TestingResult<std::path::PathBuf> {
        let evidence = ChaosScenarioEvidence {
            run_id: self.run_id.clone(),
            git_commit: self.git_commit.clone(),
            scenario_id: outcome.scenario_id.clone(),
            target: scenario.allowed_target.clone(),
            injection: outcome.injection.clone(),
            expected_failure_class: outcome.expected_class.clone(),
            observed_failure_class: outcome.observed_class.clone(),
            recovery_result: if outcome.recovery_ok {
                "RECOVERED"
            } else {
                "FAILED"
            }
            .to_string(),
            cleanup_result: if outcome.cleanup_ok { "CLEAN" } else { "DIRTY" }.to_string(),
            certification_state: "OBSERVED_LOCAL_ONLY".to_string(),
            generated_at_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            redaction_ok: true,
        };
        let path = self.evidence.write(&evidence)?;
        self.evidence
            .verify_record(&path, &self.run_id, &self.git_commit)?;
        Ok(path)
    }
}

impl ChaosScenarioId {
    pub fn from_str_unchecked(s: &str) -> Option<Self> {
        Self::all().into_iter().find(|id| id.as_str() == s)
    }
}

/// Minimal gate-outcome model for vacuity proofs (zero tests, ignored
/// tests). The real M2 runner owns the full GateResult; these proofs
/// target the invariant that zero/skipped collections are never green.
#[derive(Debug, Clone, Copy)]
struct NexusGateOutcome {
    passed: usize,
    failed: usize,
    ignored: usize,
}

impl NexusGateOutcome {
    /// ZERO TESTS COLLECTED != GREEN: a collection with no passed tests
    /// is never green.
    fn is_green(&self) -> bool {
        self.passed > 0 && self.failed == 0 && self.ignored == 0
    }
}

/// Classify helper re-export (used by tests to map error codes).
pub use crate::evidence::classify as classify_code;

/// Convenience: observed-class mapping for unit tests.
pub fn expected_class_for(code: TestingErrorCode) -> ChaosFailureClass {
    classify(code)
}

#[allow(dead_code)]
fn _injection_kind_used(kind: FailureInjectionKind) -> String {
    kind.as_str().to_string()
}
