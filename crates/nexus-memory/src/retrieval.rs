//! Hybrid retrieval policy (SPEC-002 behavior 6).
//!
//! Retrieval combines authorization filters, structured lookup, full-text,
//! vector, graph, recency, importance, confidence, and diversity. This
//! module owns the deterministic *policy* part: ranking candidates by a
//! configurable blend and enforcing diversity so a single source cannot
//! crowd out the result set. Providers execute the actual lookups; this
//! engine decides the final ranking.

use nexus_data::{DataError, DataErrorCode, MemoryCandidate};

/// Retrieval policy error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetrievalPolicyError {
    /// The blend weights do not sum to 1.
    InvalidWeights,
}

impl std::fmt::Display for RetrievalPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("retrieval blend weights must sum to 1")
    }
}

impl std::error::Error for RetrievalPolicyError {}

/// Blend weights for hybrid retrieval ranking.
///
/// Weights are normalized to sum to 1; the engine applies them to the
/// candidate's component scores (structured match, full-text, vector,
/// recency, confidence).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetrievalBlend {
    /// Structured filter match weight.
    pub structured: f64,
    /// Full-text match weight.
    pub full_text: f64,
    /// Vector similarity weight.
    pub vector: f64,
    /// Recency weight (newer = higher).
    pub recency: f64,
    /// Confidence weight.
    pub confidence: f64,
}

impl Default for RetrievalBlend {
    fn default() -> Self {
        Self {
            structured: 0.3,
            full_text: 0.2,
            vector: 0.2,
            recency: 0.15,
            confidence: 0.15,
        }
    }
}

impl RetrievalBlend {
    /// Validate that weights are non-negative and sum to 1 (within f64
    /// tolerance).
    pub fn validate(&self) -> Result<(), DataError> {
        let sum = self.structured + self.full_text + self.vector + self.recency + self.confidence;
        if self.structured < 0.0
            || self.full_text < 0.0
            || self.vector < 0.0
            || self.recency < 0.0
            || self.confidence < 0.0
        {
            return Err(DataError::new(
                DataErrorCode::Validation,
                "blend weights must be non-negative",
            ));
        }
        if (sum - 1.0).abs() > 1e-9 {
            return Err(DataError::new(
                DataErrorCode::Validation,
                "blend weights must sum to 1",
            ));
        }
        Ok(())
    }
}

/// Retrieval policy engine (SPEC-002 behavior 6).
#[derive(Debug, Clone, Copy)]
pub struct RetrievalPolicy {
    /// Blend used for ranking.
    pub blend: RetrievalBlend,
    /// Maximum number of candidates from a single source namespace.
    pub max_per_namespace: usize,
}

impl Default for RetrievalPolicy {
    fn default() -> Self {
        Self {
            blend: RetrievalBlend::default(),
            max_per_namespace: 5,
        }
    }
}

impl RetrievalPolicy {
    /// Rank candidates by the configured blend and enforce per-namespace
    /// diversity. Candidates arrive with provider component scores in
    /// `scores`; the engine computes the weighted total and keeps the top
    /// `limit`.
    ///
    /// `scores` must be parallel to `candidates`; each entry holds
    /// (structured, full_text, vector, recency, confidence) in [0, 1].
    pub fn rank(
        &self,
        candidates: Vec<MemoryCandidate>,
        scores: Vec<[f64; 5]>,
        limit: usize,
    ) -> Result<Vec<MemoryCandidate>, DataError> {
        self.blend.validate()?;
        if candidates.len() != scores.len() {
            return Err(DataError::new(
                DataErrorCode::Invariant,
                "candidates and scores must be parallel",
            ));
        }
        // Diversity: first pass counts per namespace, second pass enforces
        // the cap by dropping excess from the same namespace after ranking.
        let mut ranked: Vec<(f64, MemoryCandidate)> = candidates
            .into_iter()
            .zip(scores)
            .map(|(candidate, s)| {
                let total = self.blend.structured * s[0]
                    + self.blend.full_text * s[1]
                    + self.blend.vector * s[2]
                    + self.blend.recency * s[3]
                    + self.blend.confidence * s[4];
                (total, candidate)
            })
            .collect();
        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut out = Vec::new();
        for (_, candidate) in ranked {
            let ns = candidate.record.namespace.clone();
            let count = seen.entry(ns).or_insert(0);
            if *count >= self.max_per_namespace {
                continue;
            }
            *count += 1;
            out.push(candidate);
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_data::{MemoryStatus, RetentionPolicy, RetentionUnit, Sensitivity};
    use nexus_domain::MemoryType;
    use nexus_domain::{NexusId, TenantId};

    fn candidate(ns: &str, id_byte: u8, confidence: f64) -> MemoryCandidate {
        let record = nexus_data::MemoryRecord {
            memory_id: NexusId::new(&format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6e{id_byte:02x}"))
                .unwrap(),
            tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6e02").unwrap(),
            namespace: ns.to_string(),
            memory_type: MemoryType::Semantic,
            content: serde_json::json!({ "fact": true }),
            content_hash: "e".repeat(64),
            source: "test".to_string(),
            actor: "principal".to_string(),
            created_at: "2026-08-12T00:00:00Z".to_string(),
            observed_at: "2026-08-12T00:00:00Z".to_string(),
            confidence,
            sensitivity: Sensitivity::Household,
            purpose: "remember".to_string(),
            retention: RetentionPolicy::for_duration(RetentionUnit::Days, 30),
            status: MemoryStatus::Active,
            derived_from: vec![],
            supersedes: None,
            embedding_ref: None,
        };
        MemoryCandidate {
            record,
            score: confidence,
        }
    }

    #[test]
    fn ep004_unit_retrieval_ranks_by_blend() {
        let policy = RetrievalPolicy::default();
        let cands = vec![candidate("a", 0x01, 0.9), candidate("b", 0x02, 0.5)];
        // All component scores equal, but recency favors the second.
        let scores = vec![[0.8, 0.8, 0.8, 0.2, 0.8], [0.8, 0.8, 0.8, 0.9, 0.8]];
        let ranked = policy.rank(cands, scores, 10).unwrap();
        assert_eq!(ranked.len(), 2);
        // Second has higher recency -> ranks first.
        assert!(ranked[0].record.namespace == "b");
    }

    #[test]
    fn ep004_unit_retrieval_enforces_namespace_diversity() {
        let policy = RetrievalPolicy {
            max_per_namespace: 2,
            ..RetrievalPolicy::default()
        };
        let cands = vec![
            candidate("a", 0x01, 0.9),
            candidate("a", 0x02, 0.9),
            candidate("a", 0x03, 0.9),
            candidate("b", 0x04, 0.9),
        ];
        let scores = vec![[0.9; 5]; 4];
        let ranked = policy.rank(cands, scores, 10).unwrap();
        assert_eq!(ranked.len(), 3);
        let ns_a = ranked.iter().filter(|c| c.record.namespace == "a").count();
        assert_eq!(ns_a, 2);
    }

    #[test]
    fn ep004_unit_retrieval_rejects_bad_weights() {
        let policy = RetrievalPolicy {
            blend: RetrievalBlend {
                structured: 1.0,
                ..RetrievalBlend::default()
            },
            ..RetrievalPolicy::default()
        };
        let err = policy
            .rank(vec![candidate("a", 0x01, 0.9)], vec![[0.5; 5]], 10)
            .unwrap_err();
        assert_eq!(err.code(), DataErrorCode::Validation);
    }

    #[test]
    fn ep004_unit_retrieval_rejects_mismatched_scores() {
        let policy = RetrievalPolicy::default();
        let err = policy
            .rank(vec![candidate("a", 0x01, 0.9)], vec![], 10)
            .unwrap_err();
        assert_eq!(err.code(), DataErrorCode::Invariant);
    }
}
