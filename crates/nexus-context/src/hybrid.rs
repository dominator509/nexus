//! HybridRetriever port (EP-016; SPEC-002 behavior 6; ADR-023).
//!
//! Retrieval combines authorization filters, structured lookup,
//! full-text, vector, graph, recency, importance, confidence, and
//! diversity signals. The port is provider-neutral; the canonical
//! `RetrievalPolicy` (nexus-memory) owns the deterministic blend and
//! repository implementations own the signal sources.

use crate::error::ContextError;
use nexus_data::memory::{MemoryCandidate, MemoryQuery};
use serde::{Deserialize, Serialize};

/// Hybrid retrieval signal configuration (SPEC-002 behavior 6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalSignals {
    /// Include exact structured match signal.
    pub exact: bool,
    /// Include full-text match signal.
    pub full_text: bool,
    /// Include vector similarity signal.
    pub vector: bool,
    /// Include graph neighborhood signal.
    pub graph: bool,
    /// Recency signal weight in 0..=1.
    pub recency_weight: f64,
    /// Importance signal weight in 0..=1.
    pub importance_weight: f64,
    /// Confidence signal weight in 0..=1.
    pub confidence_weight: f64,
    /// Diversity signal weight in 0..=1.
    pub diversity_weight: f64,
}

impl RetrievalSignals {
    pub fn all() -> Self {
        Self {
            exact: true,
            full_text: true,
            vector: true,
            graph: true,
            recency_weight: 0.25,
            importance_weight: 0.25,
            confidence_weight: 0.25,
            diversity_weight: 0.25,
        }
    }

    /// Validate signal weights. Fails closed on out-of-range weights.
    pub fn validate(&self) -> Result<(), ContextError> {
        for (name, value) in [
            ("recency_weight", self.recency_weight),
            ("importance_weight", self.importance_weight),
            ("confidence_weight", self.confidence_weight),
            ("diversity_weight", self.diversity_weight),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(ContextError::validation(
                    format!("retrieval signal {name} out of range"),
                    Some("retrieval-signals".into()),
                ));
            }
        }
        Ok(())
    }
}

impl Default for RetrievalSignals {
    fn default() -> Self {
        Self::all()
    }
}

/// Provider-neutral hybrid retriever port.
pub trait HybridRetriever {
    /// Run a hybrid retrieval query combining the configured signals.
    /// Results are always tenant-isolated and authorization-filtered by
    /// the implementation.
    fn retrieve(
        &mut self,
        tenant_id: &str,
        query: &MemoryQuery,
        signals: &RetrievalSignals,
    ) -> Result<Vec<MemoryCandidate>, ContextError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep016_unit_retrieval_signals_default_valid() {
        assert!(RetrievalSignals::all().validate().is_ok());
        assert!(RetrievalSignals::default().validate().is_ok());
    }

    #[test]
    fn ep016_unit_retrieval_signals_reject_out_of_range() {
        let mut signals = RetrievalSignals::all();
        signals.importance_weight = 1.5;
        assert!(signals.validate().is_err());
        let mut signals = RetrievalSignals::all();
        signals.diversity_weight = -0.1;
        assert!(signals.validate().is_err());
    }

    #[test]
    fn ep016_unit_retrieval_signals_serde_round_trip() {
        let signals = RetrievalSignals::all();
        let v = serde_json::to_value(&signals).unwrap();
        let back: RetrievalSignals = serde_json::from_value(v).unwrap();
        assert_eq!(back, signals);
    }
}
