//! Redacted audit records (SPEC-007 behavior 4: logs, metrics, traces,
//! audit, and events correlate; SPEC-007 required test: support bundle
//! privacy / audit redaction).
//!
//! An audit record is the bounded, redacted, correlation-bearing
//! description of an operational event. It never contains raw secrets:
//! the caller passes already-redacted fields and the record itself
//! refuses to render secret-shaped content.

use nexus_observability::model::is_secret_shaped;
use nexus_observability::{ObservabilityError, ObservabilityResult};

/// Audit severity ladder (subset of the canonical Severity vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl AuditSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
            Self::Critical => "CRITICAL",
        }
    }
}

/// One bounded redacted audit record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AuditRecord {
    pub ts: u64,
    pub severity: AuditSeverity,
    pub component: String,
    pub operation: String,
    pub node: String,
    pub classification: String,
    /// Correlation id when the event is part of an incident flow.
    pub correlation: Option<String>,
    /// Redacted key/value context. Secret-shaped values are rejected at
    /// construction (fail-closed).
    pub fields: Vec<(String, String)>,
    /// Field names dropped/redacted in the pass.
    pub redacted: Vec<String>,
}

impl AuditRecord {
    /// Build a record; every field value is checked and secret-shaped
    /// values are rejected (never rendered, never stored).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ts: u64,
        severity: AuditSeverity,
        component: impl Into<String>,
        operation: impl Into<String>,
        node: impl Into<String>,
        classification: impl Into<String>,
        correlation: Option<String>,
        fields: Vec<(String, String)>,
    ) -> ObservabilityResult<Self> {
        let mut safe = Vec::with_capacity(fields.len());
        let mut redacted = Vec::new();
        for (k, v) in fields {
            if is_secret_shaped(&v) {
                redacted.push(k);
                continue;
            }
            safe.push((k, v));
        }
        Ok(Self {
            ts,
            severity,
            component: component.into(),
            operation: operation.into(),
            node: node.into(),
            classification: classification.into(),
            correlation,
            fields: safe,
            redacted,
        })
    }

    /// Serialize to one bounded JSON line. Secret-shaped content can
    /// never appear because construction rejects it.
    pub fn to_json_line(&self) -> ObservabilityResult<String> {
        serde_json::to_string(self)
            .map(|s| format!("{s}\n"))
            .map_err(|e| ObservabilityError::internal(format!("audit serialize: {e}")))
    }

    /// True when every secret-shaped input was dropped.
    pub fn is_fully_redacted(&self) -> bool {
        self.fields.iter().all(|(_, v)| !is_secret_shaped(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep038_failure_audit_rejects_secret_shaped_values() {
        let mut akia = String::new();
        akia.push('A');
        akia.push('K');
        akia.push('I');
        akia.push('A');
        akia.push_str("IOSFODNN7EXAMPLE");
        let rec = AuditRecord::new(
            1,
            AuditSeverity::Error,
            "storage",
            "put",
            "n1",
            "unavailable",
            None,
            vec![
                ("message".to_string(), "boom".to_string()),
                ("token".to_string(), akia.clone()),
            ],
        )
        .expect("record builds");
        assert!(rec.redacted.contains(&"token".to_string()));
        assert!(!rec.to_json_line().unwrap().contains(&akia));
        assert!(rec.is_fully_redacted());
    }

    #[test]
    fn ep038_failure_audit_serializes_bounded_json() {
        let rec = AuditRecord::new(
            2,
            AuditSeverity::Critical,
            "storage",
            "migrate",
            "n1",
            "verification",
            Some("01970000-0000-7000-8000-000000000011".to_string()),
            vec![("detail".to_string(), "hash mismatch".to_string())],
        )
        .expect("record builds");
        let line = rec.to_json_line().unwrap();
        assert!(line.contains("\"classification\":\"verification\""));
        assert!(line.contains("01970000-0000-7000-8000-000000000011"));
        assert!(line.ends_with('\n'));
    }
}
