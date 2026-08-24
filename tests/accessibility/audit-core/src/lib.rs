//! nexus-accessibility-audit: EP-040 M2 deterministic accessibility audit
//! core (SPEC-008; TESTING.md accessibility layer).
//!
//! This crate implements the deterministic behavior behind the M1
//! AccessibilityAuditPort: WCAG standard validation, violation
//! classification against a declared standard, and fail-closed audit
//! verdicts. A written audit is not an audit; only a run with a declared
//! standard, a real target, and a violation verdict counts.
//!
//! M2 core invariants (proven by tests):
//! - ACCESSIBILITY TARGET DECLARED != AUDIT RAN
//! - VIOLATION LISTED != VIOLATION VERIFIED
//! - AUDIT RUN != ACCESSIBILITY CERTIFIED
//! - UNKNOWN STANDARD != VALID STANDARD (deny-unknown)
//!
//! Real browser/axe scanning is NOT asserted in M2; EP-033/EP-034 own the
//! live scan harness. This core is the deterministic verdict engine.

use nexus_test_contract::error::{TestingError, TestingErrorCode, TestingResult};
use nexus_test_contract::model::AccessibilityAudit;
use nexus_test_contract::AccessibilityAuditPort;

/// Canonical WCAG levels understood by the audit core. Deny-unknown:
/// any other string is rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WcagLevel {
    A,
    AA,
    AAA,
}

impl WcagLevel {
    pub fn parse(s: &str) -> TestingResult<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "A" => Ok(Self::A),
            "AA" => Ok(Self::AA),
            "AAA" => Ok(Self::AAA),
            _ => Err(TestingError::vocabulary(format!("unknown WCAG level: {s}"))),
        }
    }
}

/// A single violation finding: criterion id + severity level + detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViolationFinding {
    pub criterion: String,
    pub level: WcagLevel,
    pub detail: String,
}

impl ViolationFinding {
    pub fn new(criterion: impl Into<String>, level: WcagLevel, detail: impl Into<String>) -> Self {
        Self {
            criterion: criterion.into(),
            level,
            detail: detail.into(),
        }
    }
}

/// Deterministic audit verdict engine behind AccessibilityAuditPort.
/// Fail-closed: an audit without a declared target/standard, or with an
/// unknown standard, cannot pass; a violation at or above the audited
/// level blocks certification.
#[derive(Debug, Default)]
pub struct DeterministicAuditEngine;

impl DeterministicAuditEngine {
    pub fn new() -> Self {
        Self
    }

    /// Evaluate a list of violation findings against an audited level.
    /// Returns Err(Policy) when any violation meets or exceeds the
    /// audited level; returns Ok otherwise. Unknown violation levels are
    /// fail-closed (never silently ignored).
    pub fn evaluate(
        &self,
        audit_level: WcagLevel,
        findings: &[ViolationFinding],
    ) -> TestingResult<()> {
        for finding in findings {
            let blocks = match (audit_level, finding.level) {
                (WcagLevel::A, WcagLevel::A) => true,
                (WcagLevel::A, _) => true, // A audit blocks anything above A too
                (WcagLevel::AA, WcagLevel::A) | (WcagLevel::AA, WcagLevel::AA) => true,
                (WcagLevel::AA, WcagLevel::AAA) => false, // AAA findings do not block AA
                (WcagLevel::AAA, _) => true,              // AAA audit blocks everything
            };
            if blocks {
                return Err(TestingError::policy(format!(
                    "accessibility violation blocks {} audit: {} ({})",
                    audit_level_name(audit_level),
                    finding.criterion,
                    finding.detail
                )));
            }
        }
        Ok(())
    }
}

fn audit_level_name(level: WcagLevel) -> &'static str {
    match level {
        WcagLevel::A => "A",
        WcagLevel::AA => "AA",
        WcagLevel::AAA => "AAA",
    }
}

impl AccessibilityAuditPort for DeterministicAuditEngine {
    fn audit(&self, audit: &AccessibilityAudit) -> TestingResult<()> {
        audit.validate()?;
        // Parse the standard's level suffix (e.g. "WCAG 2.1 AA").
        let level = audit
            .standard
            .split_whitespace()
            .last()
            .ok_or_else(|| TestingError::validation("audit standard must name a level"))?;
        let level = WcagLevel::parse(level)?;
        // Violation strings carry a "criterion@LEVEL" shape; unknown
        // shapes fail closed.
        let findings: Vec<ViolationFinding> = audit
            .violations
            .iter()
            .map(|v| parse_violation(v))
            .collect::<TestingResult<_>>()?;
        self.evaluate(level, &findings)
    }
}

/// Parse a violation string of the form "1.1.1@A: detail" or "1.4.3@AA:
/// detail". Unknown shapes fail closed - never silently ignored.
pub fn parse_violation(raw: &str) -> TestingResult<ViolationFinding> {
    let (head, detail) = raw.split_once(':').ok_or_else(|| {
        TestingError::validation(format!(
            "violation must be criterion@LEVEL: detail - got {raw:?}"
        ))
    })?;
    let (criterion, level) = head.split_once('@').ok_or_else(|| {
        TestingError::validation(format!("violation must be criterion@LEVEL - got {head:?}"))
    })?;
    let criterion = criterion.trim().to_string();
    if criterion.is_empty() {
        return Err(TestingError::validation("violation criterion is empty"));
    }
    let level = WcagLevel::parse(level)?;
    Ok(ViolationFinding::new(
        criterion,
        level,
        detail.trim().to_string(),
    ))
}

/// Typed alias so tests can assert the exact failure code.
pub fn assert_violation_code(res: &TestingResult<()>) -> TestingErrorCode {
    match res {
        Ok(()) => TestingErrorCode::Validation,
        Err(e) => e.code,
    }
}
