//! Security evidence: current-run, redacted, bound to run_id + git_commit.
//! Stale evidence, empty evidence, and secret-shaped evidence are
//! rejected. A file existing is not proof of current verification.

use std::fs;
use std::path::{Path, PathBuf};

use nexus_test_contract::error::{TestingError, TestingErrorCode, TestingResult};

/// Current-run security evidence.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityEvidence {
    /// run_id bound at collection time.
    pub run_id: String,
    /// git_commit bound at collection time.
    pub git_commit: String,
    /// Target id the scan covered.
    pub target: String,
    /// Number of findings observed (0 only when the scan really ran).
    pub finding_count: usize,
    /// Whether the scan executed against a live scanner.
    pub executed: bool,
}

impl SecurityEvidence {
    pub fn new(
        run_id: impl Into<String>,
        git_commit: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            git_commit: git_commit.into(),
            target: target.into(),
            finding_count: 0,
            executed: false,
        }
    }

    pub fn with_findings(mut self, count: usize) -> Self {
        self.finding_count = count;
        self
    }

    pub fn mark_executed(mut self) -> Self {
        self.executed = true;
        self
    }
}

/// Real evidence store on disk under an owned directory.
#[derive(Debug, Clone)]
pub struct SecurityEvidenceStore {
    pub root: PathBuf,
    pub run_id: String,
    pub git_commit: String,
}

impl SecurityEvidenceStore {
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

    /// Redact secret-shaped values from evidence BEFORE serialization so
    /// the JSON always stays valid and canaries never enter the record.
    pub fn to_redacted_json(&self, evidence: &SecurityEvidence) -> String {
        let mut redacted = evidence.clone();
        redacted.run_id = nexus_test_contract::redact_secret_shaped(&redacted.run_id);
        redacted.git_commit = nexus_test_contract::redact_secret_shaped(&redacted.git_commit);
        redacted.target = nexus_test_contract::redact_secret_shaped(&redacted.target);
        serde_json::to_string(&redacted).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn write(&self, evidence: &SecurityEvidence) -> TestingResult<PathBuf> {
        if self.run_id.trim().is_empty() || self.git_commit.trim().is_empty() {
            return Err(TestingError::missing_evidence(
                "security evidence requires run_id and git_commit",
            ));
        }
        if !self.root.exists() {
            fs::create_dir_all(&self.root).map_err(|e| {
                TestingError::new(
                    TestingErrorCode::Unavailable,
                    format!("cannot create security evidence root: {e}"),
                )
            })?;
        }
        let file = self
            .root
            .join(format!("{}-{}.json", self.run_id, evidence.target));
        let json = self.to_redacted_json(evidence);
        fs::write(&file, json).map_err(|e| {
            TestingError::new(
                TestingErrorCode::Unavailable,
                format!("cannot write security evidence: {e}"),
            )
        })?;
        Ok(file)
    }

    /// Verify a written record round-trips and remains redacted.
    pub fn verify_record(&self, path: &Path) -> TestingResult<()> {
        let content = fs::read_to_string(path)
            .map_err(|e| TestingError::verification(format!("cannot read evidence: {e}")))?;
        let record: SecurityEvidence = serde_json::from_str(&content)
            .map_err(|e| TestingError::verification(format!("evidence malformed: {e}")))?;
        if record.run_id != self.run_id || record.git_commit != self.git_commit {
            return Err(TestingError::verification(
                "security evidence run_id/git_commit mismatch",
            ));
        }
        let redacted = nexus_test_contract::redact_secret_shaped(&content);
        if redacted != content {
            return Err(TestingError::verification(
                "security evidence contains secret-shaped values",
            ));
        }
        Ok(())
    }
}
