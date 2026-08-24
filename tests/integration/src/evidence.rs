//! Evidence store: records current-run TestEvidence to disk, redacted and
//! bound to run_id + git_commit. A file existing is not proof; evidence
//! only counts when the record is bound and verifiable.

use std::fs;
use std::path::{Path, PathBuf};

use nexus_test_contract::error::{TestingError, TestingErrorCode, TestingResult};
use nexus_test_contract::model::TestEvidence;
use nexus_test_contract::EvidencePort;

/// A single current-run evidence record.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceRecord {
    pub run_id: String,
    pub git_commit: String,
    pub evidence: TestEvidence,
}

/// Real filesystem evidence store under an owned directory.
#[derive(Debug, Clone)]
pub struct FileEvidenceStore {
    pub root: PathBuf,
    pub run_id: String,
    pub git_commit: String,
}

impl FileEvidenceStore {
    pub fn new(
        root: impl Into<PathBuf>,
        run_id: impl Into<String>,
        git_commit: impl Into<String>,
    ) -> Self {
        Self {
            root: root.into(),
            run_id: run_id.into(),
            git_commit: git_commit.into(),
        }
    }

    /// Serialize one evidence record as redacted JSON (secret-shaped
    /// values scrubbed BEFORE serialization so the JSON always stays
    /// valid and canaries never enter the record).
    pub fn to_redacted_json(&self, evidence: &TestEvidence) -> String {
        let mut redacted = evidence.clone();
        redacted.test_id = nexus_test_contract::redact_secret_shaped(&redacted.test_id);
        let record = EvidenceRecord {
            run_id: nexus_test_contract::redact_secret_shaped(&self.run_id),
            git_commit: nexus_test_contract::redact_secret_shaped(&self.git_commit),
            evidence: redacted,
        };
        serde_json::to_string(&record).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn write(&self, evidence: &TestEvidence) -> TestingResult<PathBuf> {
        if self.run_id.trim().is_empty() || self.git_commit.trim().is_empty() {
            return Err(TestingError::missing_evidence(
                "evidence requires run_id and git_commit",
            ));
        }
        if !self.root.exists() {
            fs::create_dir_all(&self.root).map_err(|e| {
                TestingError::new(
                    TestingErrorCode::Unavailable,
                    format!("cannot create evidence root: {e}"),
                )
            })?;
        }
        let file = self.root.join(format!(
            "{}-{}.json",
            self.run_id,
            evidence.test_id.replace(':', "_")
        ));
        let json = self.to_redacted_json(evidence);
        fs::write(&file, json).map_err(|e| {
            TestingError::new(
                TestingErrorCode::Unavailable,
                format!("cannot write evidence: {e}"),
            )
        })?;
        Ok(file)
    }

    /// Verify a written record round-trips and remains redacted.
    pub fn verify_record(&self, path: &Path) -> TestingResult<()> {
        let content = fs::read_to_string(path)
            .map_err(|e| TestingError::verification(format!("cannot read evidence: {e}")))?;
        let record: EvidenceRecord = serde_json::from_str(&content)
            .map_err(|e| TestingError::verification(format!("evidence malformed: {e}")))?;
        if record.run_id != self.run_id || record.git_commit != self.git_commit {
            return Err(TestingError::verification(
                "evidence run_id/git_commit mismatch",
            ));
        }
        let redacted = nexus_test_contract::redact_secret_shaped(&content);
        if redacted != content {
            return Err(TestingError::verification(
                "evidence contains secret-shaped values",
            ));
        }
        Ok(())
    }
}

impl EvidencePort for FileEvidenceStore {
    fn record(&self, evidence: TestEvidence) -> TestingResult<()> {
        let path = self.write(&evidence)?;
        self.verify_record(&path)
    }
}
