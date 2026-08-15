//! Redacted context telemetry (SPEC-006; EP-016 M2).
//!
//! The worker emits safe metadata only: purpose, candidate/selected
//! counts, retrieval signal classes, graph depth, namespace
//! fingerprints, privacy decision, consolidation mode, and correlation
//! id. Never raw private memory text, personal secrets, complete
//! sensitive entity values, embeddings, or the full context capsule.

use crate::util::namespace_fingerprint;
use nexus_context::{ConsolidationMode, ContextPurpose, PrivacyFilterDecision};
use serde::{Deserialize, Serialize};

/// Redacted telemetry snapshot for a context construction pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextTelemetry {
    pub correlation_id: String,
    pub purpose: String,
    /// Number of candidates entering the pipeline (pre-filter).
    pub candidate_count: usize,
    /// Number of items selected for the capsule.
    pub selected_count: usize,
    /// Retrieval signal classes actually available.
    pub signal_classes: Vec<String>,
    /// Graph expansion depth used (0 when unused).
    pub graph_depth: usize,
    /// Stable fingerprints of the namespaces involved (never the
    /// namespace names themselves).
    pub namespace_fingerprints: Vec<String>,
    /// Privacy decisions observed (Allow/Redact/Deny), aggregated.
    pub privacy_decisions: Vec<String>,
    /// Consolidation mode used (when consolidation ran).
    pub consolidation_mode: Option<String>,
}

impl ContextTelemetry {
    pub fn new(correlation_id: impl Into<String>, purpose: ContextPurpose) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            purpose: purpose.as_str().into(),
            candidate_count: 0,
            selected_count: 0,
            signal_classes: vec![],
            graph_depth: 0,
            namespace_fingerprints: vec![],
            privacy_decisions: vec![],
            consolidation_mode: None,
        }
    }

    /// Record namespace fingerprints deterministically (deduplicated,
    /// sorted for stable output).
    pub fn with_namespaces(&mut self, namespaces: &[String]) {
        let mut fingerprints: Vec<String> = namespaces
            .iter()
            .map(|ns| namespace_fingerprint(ns))
            .collect();
        fingerprints.sort();
        fingerprints.dedup();
        self.namespace_fingerprints = fingerprints;
    }

    pub fn with_signal_classes(&mut self, classes: &[&str]) {
        let mut classes: Vec<String> = classes.iter().map(|s| s.to_string()).collect();
        classes.sort();
        classes.dedup();
        self.signal_classes = classes;
    }

    pub fn record_privacy(&mut self, decision: PrivacyFilterDecision) {
        let label = match decision {
            PrivacyFilterDecision::Allow => "ALLOW",
            PrivacyFilterDecision::Redact => "REDACT",
            PrivacyFilterDecision::Deny => "DENY",
        };
        if !self.privacy_decisions.iter().any(|d| d == label) {
            self.privacy_decisions.push(label.into());
        }
        self.privacy_decisions.sort();
    }

    pub fn record_consolidation(&mut self, mode: ConsolidationMode) {
        self.consolidation_mode = Some(match mode {
            ConsolidationMode::ModelAssisted => "MODEL_ASSISTED".into(),
            ConsolidationMode::DeterministicFallback => "DETERMINISTIC_FALLBACK".into(),
            ConsolidationMode::Skipped => "SKIPPED".into(),
        });
    }

    /// Never allow telemetry to carry content: sanitize any serialized
    /// form by construction (fields above are metadata only).
    pub fn redacted_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep016_unit_telemetry_never_contains_content() {
        let mut t = ContextTelemetry::new("c-1", ContextPurpose::TaskExecution);
        t.candidate_count = 12;
        t.selected_count = 4;
        t.with_namespaces(&["household".into(), "household".into(), "business".into()]);
        t.with_signal_classes(&["exact", "full_text", "vector", "exact"]);
        t.record_privacy(PrivacyFilterDecision::Allow);
        t.record_privacy(PrivacyFilterDecision::Deny);
        t.record_consolidation(ConsolidationMode::DeterministicFallback);
        let json = t.redacted_json();
        let s = json.to_string();
        assert!(!s.contains("household")); // namespaces are fingerprinted
        assert!(!s.contains("secret"));
        assert!(!s.contains("note"));
        assert_eq!(t.namespace_fingerprints.len(), 2);
    }

    #[test]
    fn ep016_unit_telemetry_deterministic() {
        let mut a = ContextTelemetry::new("c-1", ContextPurpose::Search);
        a.with_namespaces(&["household".into()]);
        let mut b = ContextTelemetry::new("c-1", ContextPurpose::Search);
        b.with_namespaces(&["household".into()]);
        assert_eq!(a.redacted_json(), b.redacted_json());
    }
}
