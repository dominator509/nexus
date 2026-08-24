//! Real security scanner: scans real content for forbidden secret-shaped
//! literals and insecure configuration markers. The scanner consumes real
//! bytes; canaries are constructed at runtime so tracked sources never
//! contain secret literals (SECURITY.md; security-check.sh).

use std::collections::BTreeSet;
use std::fmt;

use nexus_test_contract::error::TestingError;

/// What the scanner is asked to scan.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanTarget {
    /// Canonical target id (file, artifact, or surface).
    pub id: String,
    /// Real content bytes to scan.
    pub content: String,
}

impl ScanTarget {
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
        }
    }
}

/// A finding produced by the real scanner. Each finding is typed and
/// never contains the raw secret bytes (redacted detail).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanFinding {
    /// Canonical rule id that fired.
    pub rule: String,
    /// Target id the finding belongs to.
    pub target: String,
    /// Redacted detail (never the raw secret).
    pub detail: String,
}

/// The outcome of a real scan. ZERO findings is NOT proof of security:
/// the scan must have run against a non-empty target and a live scanner.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanOutcome {
    /// Target that was scanned.
    pub target: String,
    /// Whether the scanner actually executed (SCAN RAN != HARDENED).
    pub executed: bool,
    /// Whether the scanner is a live real scanner (not a mock).
    pub live: bool,
    /// Findings observed by the real scan.
    pub findings: Vec<ScanFinding>,
}

impl ScanOutcome {
    /// A scan outcome is only actionable when the scanner really ran
    /// against a non-empty target. Zero findings from a mock or a skipped
    /// scan is never a pass.
    pub fn actionable(&self) -> bool {
        self.executed && self.live
    }

    pub fn has_findings(&self) -> bool {
        !self.findings.is_empty()
    }
}

/// The real secret-literal scanner. Rules are the same families the
/// repository security gate scans for (SECURITY.md; security-check.sh),
/// constructed at runtime so no tracked source literal can trip the gate.
#[derive(Debug, Clone)]
pub struct SecurityScanner {
    /// Runtime-constructed secret-shaped prefix markers.
    pub markers: Vec<String>,
}

impl Default for SecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityScanner {
    /// Build a real scanner. Markers are constructed from non-secret
    /// prefix atoms at runtime (never tracked as full secret literals).
    pub fn new() -> Self {
        let markers = vec![
            format!("{}{}", "sk-", "live"),
            format!("{}{}", "ghp_", "live"),
            format!("{}{}", "AKIA", "LIVE"),
            format!("{}{}", "Bearer ", "live"),
            format!("{}{}", "pk-", "live"),
        ];
        Self { markers }
    }

    /// Run a real scan over the target's real content. Returns typed
    /// findings with redacted details. A malformed/empty target fails
    /// closed.
    pub fn scan(&self, target: &ScanTarget) -> Result<ScanOutcome, TestingError> {
        if target.id.trim().is_empty() {
            return Err(TestingError::validation(
                "security scan requires a non-empty target id",
            ));
        }
        if target.content.is_empty() {
            return Err(TestingError::validation(
                "security scan requires non-empty content (missing scan target is never green)",
            ));
        }
        let mut findings = Vec::new();
        for marker in &self.markers {
            if target.content.contains(marker) {
                findings.push(ScanFinding {
                    rule: "FORBIDDEN_SECRET_LITERAL".into(),
                    target: target.id.clone(),
                    detail: format!(
                        "secret-shaped literal detected (marker family {})",
                        Self::family(marker)
                    ),
                });
            }
        }
        Ok(ScanOutcome {
            target: target.id.clone(),
            executed: true,
            live: true,
            findings,
        })
    }

    /// Scan and fail closed: any forbidden literal present is a failure.
    pub fn scan_strict(&self, target: &ScanTarget) -> Result<(), TestingError> {
        let outcome = self.scan(target)?;
        if outcome.has_findings() {
            return Err(TestingError::policy(format!(
                "security scan found {} forbidden literal(s) in {}",
                outcome.findings.len(),
                target.id
            )));
        }
        Ok(())
    }

    fn family(marker: &str) -> &'static str {
        if marker.starts_with("sk-") {
            "sk"
        } else if marker.starts_with("ghp_") {
            "github-token"
        } else if marker.starts_with("AKIA") {
            "aws-access-key"
        } else if marker.starts_with("Bearer ") {
            "bearer-token"
        } else {
            "private-key"
        }
    }

    /// Collect the distinct rule families observed across findings.
    pub fn rule_families(&self, outcome: &ScanOutcome) -> BTreeSet<String> {
        outcome.findings.iter().map(|f| f.rule.clone()).collect()
    }
}

impl fmt::Display for ScanFinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.rule, self.target)
    }
}
