//! Current-run chaos evidence (EP-040 M5 fence section K). Evidence
//! binds run_id, git_commit, scenario id, target, injection, observed
//! failure class, recovery result, cleanup result, counts, and
//! certification state. Stale evidence never satisfies. Values are
//! redacted BEFORE serialization so the JSON stays valid and canaries
//! never enter the record.

use std::fs;
use std::path::{Path, PathBuf};

use nexus_test_contract::error::{TestingError, TestingErrorCode, TestingResult};

use crate::failure::ChaosFailureClass;

/// One current-run chaos scenario evidence record.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChaosScenarioEvidence {
    pub run_id: String,
    pub git_commit: String,
    pub scenario_id: String,
    pub target: String,
    pub injection: String,
    pub expected_failure_class: String,
    pub observed_failure_class: String,
    pub recovery_result: String,
    pub cleanup_result: String,
    pub certification_state: String,
    pub generated_at_unix: u64,
    pub redaction_ok: bool,
}

/// Real filesystem evidence store for chaos scenarios, under an
/// EP-040-owned root. Write then verify; a file existing is not proof.
#[derive(Debug, Clone)]
pub struct ChaosEvidenceStore {
    pub root: PathBuf,
}

impl ChaosEvidenceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Serialize one evidence record as redacted JSON (field values
    /// scrubbed BEFORE serialization).
    pub fn to_redacted_json(&self, evidence: &ChaosScenarioEvidence) -> String {
        let mut redacted = evidence.clone();
        redacted.run_id = nexus_test_contract::redact_secret_shaped(&redacted.run_id);
        redacted.git_commit = nexus_test_contract::redact_secret_shaped(&redacted.git_commit);
        redacted.target = nexus_test_contract::redact_secret_shaped(&redacted.target);
        redacted.injection = nexus_test_contract::redact_secret_shaped(&redacted.injection);
        serde_json::to_string(&redacted).unwrap_or_else(|_| "{}".to_string())
    }

    /// Write one record under the owned root.
    pub fn write(&self, evidence: &ChaosScenarioEvidence) -> TestingResult<PathBuf> {
        if evidence.run_id.trim().is_empty() || evidence.git_commit.trim().is_empty() {
            return Err(TestingError::missing_evidence(
                "chaos evidence requires run_id and git_commit",
            ));
        }
        if !self.root.exists() {
            fs::create_dir_all(&self.root).map_err(|e| {
                TestingError::new(
                    TestingErrorCode::Unavailable,
                    format!("cannot create chaos evidence root: {e}"),
                )
            })?;
        }
        let file = self.root.join(format!(
            "{}-{}.json",
            evidence.run_id,
            evidence.scenario_id.replace([':', '/'], "_")
        ));
        let json = self.to_redacted_json(evidence);
        fs::write(&file, json).map_err(|e| {
            TestingError::new(
                TestingErrorCode::Unavailable,
                format!("cannot write chaos evidence: {e}"),
            )
        })?;
        Ok(file)
    }

    /// Verify a written record round-trips, is bound to the current
    /// run, and contains no secret-shaped values.
    pub fn verify_record(&self, path: &Path, run_id: &str, git_commit: &str) -> TestingResult<()> {
        let content = fs::read_to_string(path)
            .map_err(|e| TestingError::verification(format!("cannot read evidence: {e}")))?;
        let record: ChaosScenarioEvidence = serde_json::from_str(&content)
            .map_err(|e| TestingError::verification(format!("evidence malformed: {e}")))?;
        if record.run_id != run_id || record.git_commit != git_commit {
            return Err(TestingError::verification(
                "chaos evidence run_id/git_commit mismatch",
            ));
        }
        let redacted = nexus_test_contract::redact_secret_shaped(&content);
        if redacted != content {
            return Err(TestingError::verification(
                "chaos evidence contains secret-shaped values",
            ));
        }
        Ok(())
    }

    /// Remove the owned evidence root entirely (teardown).
    pub fn remove_root(&self) {
        let _ = fs::remove_dir_all(&self.root);
    }

    /// True when the owned root does not exist (zero residue).
    pub fn is_clean(&self) -> bool {
        !self.root.exists()
    }
}

/// Map an observed TestingError code to the EP-040 chaos failure class.
pub fn classify(code: TestingErrorCode) -> ChaosFailureClass {
    match code {
        TestingErrorCode::Timeout | TestingErrorCode::RateLimit => ChaosFailureClass::Timeout,
        TestingErrorCode::Unavailable | TestingErrorCode::ExternalProvider => {
            ChaosFailureClass::Unavailable
        }
        TestingErrorCode::Authorization | TestingErrorCode::Policy => {
            ChaosFailureClass::PolicyDenied
        }
        TestingErrorCode::Verification => ChaosFailureClass::SecurityFailure,
        TestingErrorCode::ZeroTestCollection
        | TestingErrorCode::RequiredTestSkipped
        | TestingErrorCode::RequiredTestIgnored
        | TestingErrorCode::VacuousGate => ChaosFailureClass::OwnerCodeRegression,
        TestingErrorCode::ResourceResidue => ChaosFailureClass::Environment,
        TestingErrorCode::MissingEvidence | TestingErrorCode::MockOnlyCertification => {
            ChaosFailureClass::SecurityFailure
        }
        TestingErrorCode::BlastRadiusExceeded
        | TestingErrorCode::RollbackUnavailable
        | TestingErrorCode::FlakeUnresolved => ChaosFailureClass::OwnerCodeRegression,
        _ => ChaosFailureClass::Environment,
    }
}
