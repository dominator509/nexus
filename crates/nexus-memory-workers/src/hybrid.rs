//! Hybrid retrieval workers (SPEC-002 behavior 6; EP-016 M2).
//!
//! Retrieval combines authorization filters, structured lookup,
//! full-text, vector, graph, recency, importance, confidence, and
//! diversity signals with deterministic score composition. Exact
//! structured/entity matches dominate vague semantic similarity (exact
//! precedence tier). When vector retrieval is unavailable the worker
//! does NOT fabricate semantic candidates: deterministic retrieval
//! (structured + full-text + graph + recency/importance/confidence)
//! continues and reports `semantic_available = false`.

use crate::lifecycle::{ActiveMemoryLifecycleFilter, LifecycleContext};
use crate::permission::{AccessProfile, PermissionFilter};
use crate::purpose::{PurposeLimiter, PurposePolicy};
use crate::util::clamp01;
use nexus_context::{ContextError, HybridRetriever, RetrievalSignals};
use nexus_data::memory::{MemoryCandidate, MemoryQuery};
use std::cmp::Ordering;

/// Provider-supplied per-candidate signals. This is the injected I/O
/// boundary: the provider (repository adapter) computes raw similarity,
/// graph proximity, recency, and importance; the worker blends them
/// deterministically.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateSignals {
    /// Exact structured/entity/identifier match (direct name match).
    pub exact: bool,
    /// Full-text relevance in [0, 1].
    pub full_text: f64,
    /// Vector similarity in [0, 1]; `None` when semantic retrieval is
    /// unavailable for this candidate.
    pub vector: Option<f64>,
    /// Graph proximity in [0, 1]; `None` when no graph signal.
    pub graph: Option<f64>,
    /// Recency score in [0, 1] (newer = higher), computed by the
    /// caller/outer layer from the injected clock.
    pub recency: f64,
    /// Importance score in [0, 1].
    pub importance: f64,
    /// Diversity cluster key: canonical entity, supersession chain,
    /// source event, or semantic cluster id. Empty means singleton.
    pub diversity_key: String,
}

impl CandidateSignals {
    /// Validate component ranges. Fails closed on out-of-range values.
    pub fn validate(&self) -> Result<(), ContextError> {
        for (name, value) in [
            ("full_text", self.full_text),
            ("recency", self.recency),
            ("importance", self.importance),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(ContextError::validation(
                    format!("candidate signal {name} out of range"),
                    Some("candidate-signals".into()),
                ));
            }
        }
        match self.vector {
            Some(v) if !(0.0..=1.0).contains(&v) => {
                return Err(ContextError::validation(
                    "candidate signal vector out of range",
                    Some("candidate-signals".into()),
                ));
            }
            _ => {}
        }
        match self.graph {
            Some(g) if !(0.0..=1.0).contains(&g) => {
                return Err(ContextError::validation(
                    "candidate signal graph out of range",
                    Some("candidate-signals".into()),
                ));
            }
            _ => {}
        }
        Ok(())
    }
}

/// Auditable component scores for one candidate (SPEC-002 behavior 6;
/// ranking transparency).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreComponents {
    /// Exact structured match (0 or 1).
    pub exact: f64,
    /// Full-text relevance.
    pub full_text: f64,
    /// Vector similarity, when available.
    pub vector: Option<f64>,
    /// Graph proximity, when available.
    pub graph: Option<f64>,
    /// Recency.
    pub recency: f64,
    /// Importance.
    pub importance: f64,
    /// Confidence (from the record).
    pub confidence: f64,
    /// Whether semantic retrieval was available for this candidate.
    pub semantic_available: bool,
}

/// A scored candidate with its auditable components.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredCandidate {
    pub candidate: MemoryCandidate,
    /// Deterministic blended total in [0, 1].
    pub total: f64,
    pub components: ScoreComponents,
}

/// Deterministic hybrid scorer. Pure function of inputs.
///
/// Score composition policy (EP-016 Decision Log):
/// - Enabled signals contribute a normalized blend. Boolean signals
///   (exact, full-text, vector, graph) contribute their component
///   values when enabled; weighted signals (recency, importance,
///   confidence) contribute `weight * component`.
/// - Exact precedence: an exact structured/entity match forms a
///   separate ranking tier above all non-exact candidates, so a direct
///   entity match never loses to vaguely similar embedding similarity.
/// - Vector unavailable: the vector component is omitted and
///   `semantic_available = false`; the blend renormalizes over the
///   remaining signals. No synthetic embedding score is returned.
#[derive(Debug, Clone, Copy, Default)]
pub struct HybridScorer;

impl HybridScorer {
    /// Score one candidate. `signals` configures which signals
    /// participate and the weighted blend.
    pub fn score(
        &self,
        signals: &RetrievalSignals,
        candidate: MemoryCandidate,
        provider: CandidateSignals,
    ) -> Result<ScoredCandidate, ContextError> {
        signals.validate()?;
        provider.validate()?;
        let confidence = clamp01(candidate.record.confidence);
        let vector = if signals.vector {
            provider.vector
        } else {
            None
        };
        let graph = if signals.graph { provider.graph } else { None };
        let exact = if signals.exact && provider.exact {
            1.0
        } else {
            0.0
        };
        let full_text = if signals.full_text {
            provider.full_text
        } else {
            0.0
        };

        let mut terms: Vec<f64> = Vec::new();
        if signals.exact {
            terms.push(exact);
        }
        if signals.full_text {
            terms.push(full_text);
        }
        if let Some(v) = vector {
            terms.push(v);
        }
        if let Some(g) = graph {
            terms.push(g);
        }
        // Weighted signals always participate (weight 0 disables).
        terms.push(signals.recency_weight * provider.recency);
        terms.push(signals.importance_weight * provider.importance);
        terms.push(signals.confidence_weight * confidence);

        let total = if terms.is_empty() {
            0.0
        } else {
            terms.iter().sum::<f64>() / terms.len() as f64
        };

        Ok(ScoredCandidate {
            candidate,
            total: clamp01(total),
            components: ScoreComponents {
                exact,
                full_text,
                vector,
                graph,
                recency: provider.recency,
                importance: provider.importance,
                confidence,
                semantic_available: vector.is_some(),
            },
        })
    }

    /// Exact tier for ordering: 1 for exact matches, 0 otherwise.
    pub fn exact_tier(scored: &ScoredCandidate) -> u8 {
        if scored.components.exact > 0.0 { 1 } else { 0 }
    }

    /// Deterministic ordering: exact tier desc, total desc, observed
    /// time desc, canonical memory id asc.
    pub fn order(a: &ScoredCandidate, b: &ScoredCandidate) -> Ordering {
        let tier = Self::exact_tier(b).cmp(&Self::exact_tier(a));
        if tier != Ordering::Equal {
            return tier;
        }
        let total = b.total.partial_cmp(&a.total).unwrap_or(Ordering::Equal);
        if total != Ordering::Equal {
            return total;
        }
        let time = crate::util::rfc3339_utc_millis(&b.candidate.record.observed_at).cmp(
            &crate::util::rfc3339_utc_millis(&a.candidate.record.observed_at),
        );
        if time != Ordering::Equal {
            return time;
        }
        a.candidate
            .record
            .memory_id
            .cmp(&b.candidate.record.memory_id)
    }
}

/// Injected candidate provider port. The adapter fetches raw candidates
/// plus provider signals; the worker never performs the I/O itself.
pub trait CandidateProvider {
    /// Fetch candidates for a tenant and query, with provider signals.
    /// The adapter applies its own tenant isolation and returns only
    /// candidates within the tenant; the worker enforces permission,
    /// purpose, lifecycle, scoring, and diversity on top.
    fn fetch(
        &mut self,
        tenant_id: &str,
        query: &MemoryQuery,
    ) -> Result<Vec<(MemoryCandidate, CandidateSignals)>, ContextError>;
}

/// Deterministic hybrid retriever implementing the provider-neutral
/// `HybridRetriever` port. All I/O is injected through
/// `CandidateProvider`; the worker applies the canonical pipeline
/// (permission -> purpose -> lifecycle -> score -> diversity -> order).
#[derive(Debug, Clone)]
pub struct DeterministicHybridRetriever<P> {
    /// Candidate source (injected adapter).
    pub provider: P,
    /// Principal access profile (injected authorization input).
    pub access: AccessProfile,
    /// Purpose policy for this request.
    pub purpose: PurposePolicy,
    /// Lifecycle context (injected clock).
    pub lifecycle: LifecycleContext,
    /// Maximum candidates retained per diversity cluster.
    pub max_per_cluster: usize,
}

impl<P: CandidateProvider> DeterministicHybridRetriever<P> {
    pub fn new(
        provider: P,
        access: AccessProfile,
        purpose: PurposePolicy,
        lifecycle: LifecycleContext,
        max_per_cluster: usize,
    ) -> Self {
        Self {
            provider,
            access,
            purpose,
            lifecycle,
            max_per_cluster: max_per_cluster.max(1),
        }
    }

    /// Run the full deterministic retrieval pipeline and return scored
    /// candidates in ranked order with auditable components.
    pub fn retrieve_scored(
        &mut self,
        query: &MemoryQuery,
        signals: &RetrievalSignals,
    ) -> Result<(Vec<ScoredCandidate>, bool), ContextError> {
        let raw = self.provider.fetch(&self.access.tenant_id, query)?;
        let permission =
            PermissionFilter.filter(&self.access, raw.iter().map(|(c, _)| c.clone()).collect())?;
        let purpose = PurposeLimiter.filter(&self.purpose, permission)?;
        let lifecycle = ActiveMemoryLifecycleFilter.filter(&self.lifecycle, purpose)?;

        let mut scored: Vec<ScoredCandidate> = Vec::new();
        let mut semantic_available = true;
        for (candidate, provider_signals) in raw {
            if !lifecycle
                .iter()
                .any(|c| c.record.memory_id == candidate.record.memory_id)
            {
                continue;
            }
            let item = HybridScorer.score(signals, candidate, provider_signals)?;
            if !item.components.semantic_available {
                semantic_available = false;
            }
            scored.push(item);
        }

        scored.sort_by(HybridScorer::order);
        let mut deduped: Vec<ScoredCandidate> = Vec::new();
        let mut cluster_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for item in scored {
            let key = item
                .candidate
                .record
                .supersedes
                .as_ref()
                .map(|id| format!("chain:{}", id.as_str()))
                .unwrap_or_else(|| {
                    let cluster = item
                        .candidate
                        .record
                        .derived_from
                        .first()
                        .map(|id| id.as_str().to_string());
                    cluster.unwrap_or_else(|| item.candidate.record.memory_id.as_str().to_string())
                });
            // Diversity: cluster by supersession chain / source event /
            // derivation root. Prefer the representative highest-quality
            // candidate; cap the rest.
            let count = cluster_counts.entry(key).or_insert(0);
            if *count >= self.max_per_cluster {
                continue;
            }
            *count += 1;
            deduped.push(item);
        }

        Ok((deduped, semantic_available))
    }
}

impl<P: CandidateProvider> HybridRetriever for DeterministicHybridRetriever<P> {
    fn retrieve(
        &mut self,
        tenant_id: &str,
        query: &MemoryQuery,
        signals: &RetrievalSignals,
    ) -> Result<Vec<MemoryCandidate>, ContextError> {
        // The port contract: results are tenant-isolated and
        // authorization-filtered. The worker's pipeline already enforces
        // tenant/permission; verify the caller's tenant matches the
        // profile (fail closed on mismatch).
        if tenant_id != self.access.tenant_id {
            return Err(ContextError::authorization(
                "retrieve tenant does not match access profile",
                Some("hybrid-retriever".into()),
            ));
        }
        let (scored, _) = self.retrieve_scored(query, signals)?;
        Ok(scored.into_iter().map(|s| s.candidate).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::rfc3339_utc_millis;
    use nexus_data::memory::{
        MemoryRecord, MemoryStatus, RetentionPolicy, RetentionUnit, Sensitivity,
    };
    use nexus_domain::{MemoryType, NexusId, TenantId};

    fn record(id_byte: u8, observed_at: &str, confidence: f64) -> MemoryCandidate {
        MemoryCandidate {
            record: MemoryRecord {
                memory_id: NexusId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6f{id_byte:02x}"))
                    .unwrap(),
                tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6f80").unwrap(),
                namespace: "household".into(),
                memory_type: MemoryType::Semantic,
                content: serde_json::json!({ "fact": true }),
                content_hash: "f".repeat(64),
                source: "test".into(),
                actor: "p-1".into(),
                created_at: observed_at.into(),
                observed_at: observed_at.into(),
                confidence,
                sensitivity: Sensitivity::Household,
                purpose: "SEARCH".into(),
                retention: RetentionPolicy::for_duration(RetentionUnit::Days, 90),
                status: MemoryStatus::Active,
                derived_from: vec![],
                supersedes: None,
                embedding_ref: None,
            },
            score: 0.5,
        }
    }

    struct StubProvider(Vec<(MemoryCandidate, CandidateSignals)>);

    impl CandidateProvider for StubProvider {
        fn fetch(
            &mut self,
            _tenant_id: &str,
            _query: &MemoryQuery,
        ) -> Result<Vec<(MemoryCandidate, CandidateSignals)>, ContextError> {
            Ok(self.0.clone())
        }
    }

    fn profile() -> AccessProfile {
        AccessProfile {
            tenant_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6f80".into(),
            principal_id: "p-1".into(),
            allowed_namespaces: vec!["household".into()],
            max_sensitivity: Sensitivity::BusinessConfidential,
            private_allowed: true,
        }
    }

    fn retriever(provider: StubProvider) -> DeterministicHybridRetriever<StubProvider> {
        DeterministicHybridRetriever::new(
            provider,
            profile(),
            PurposeLimiter::policy_for(nexus_context::ContextPurpose::Search),
            LifecycleContext {
                now_epoch_ms: rfc3339_utc_millis("2026-01-01T00:00:00Z").unwrap(),
                include_historical: false,
            },
            2,
        )
    }

    #[test]
    fn ep016_unit_hybrid_blends_exact_fts_vector_graph_recency_importance_confidence() {
        let mut c = record(0x01, "2025-12-01T00:00:00Z", 0.9);
        let signals = CandidateSignals {
            exact: true,
            full_text: 0.8,
            vector: Some(0.6),
            graph: Some(0.7),
            recency: 0.4,
            importance: 0.9,
            diversity_key: String::new(),
        };
        let scored = HybridScorer
            .score(&RetrievalSignals::all(), c.clone(), signals)
            .unwrap();
        assert!(scored.total > 0.0);
        assert_eq!(scored.components.exact, 1.0);
        assert_eq!(scored.components.confidence, 0.9);
        assert!(scored.components.semantic_available);
        c.record.content_hash = "f".repeat(64); // keep clippy quiet about mut
    }

    #[test]
    fn ep016_unit_exact_entity_beats_unrelated_semantic_similarity() {
        // "front door lock" exact entity vs "garage door lock" with high
        // embedding similarity but no exact match: exact must win.
        let exact = HybridScorer
            .score(
                &RetrievalSignals::all(),
                record(0x01, "2025-12-01T00:00:00Z", 0.8),
                CandidateSignals {
                    exact: true,
                    full_text: 0.5,
                    vector: Some(0.3),
                    graph: Some(0.4),
                    recency: 0.5,
                    importance: 0.5,
                    diversity_key: String::new(),
                },
            )
            .unwrap();
        let vague = HybridScorer
            .score(
                &RetrievalSignals::all(),
                record(0x02, "2025-12-01T00:00:00Z", 0.8),
                CandidateSignals {
                    exact: false,
                    full_text: 0.9,
                    vector: Some(0.95),
                    graph: Some(0.5),
                    recency: 0.6,
                    importance: 0.6,
                    diversity_key: String::new(),
                },
            )
            .unwrap();
        assert_eq!(HybridScorer::exact_tier(&exact), 1);
        assert_eq!(HybridScorer::exact_tier(&vague), 0);
        assert!(HybridScorer::order(&exact, &vague) == Ordering::Less);
    }

    #[test]
    fn ep016_unit_semantic_unavailable_falls_back_deterministically() {
        let signals = RetrievalSignals::all();
        let scored = HybridScorer
            .score(
                &signals,
                record(0x03, "2025-12-01T00:00:00Z", 0.9),
                CandidateSignals {
                    exact: true,
                    full_text: 0.7,
                    vector: None,
                    graph: Some(0.5),
                    recency: 0.4,
                    importance: 0.8,
                    diversity_key: String::new(),
                },
            )
            .unwrap();
        assert!(!scored.components.semantic_available);
        assert!(scored.total > 0.0);
    }

    #[test]
    fn ep016_unit_retriever_filters_and_orders_deterministically() {
        let provider = StubProvider(vec![
            (
                record(0x0a, "2025-12-01T00:00:00Z", 0.9),
                CandidateSignals {
                    exact: false,
                    full_text: 0.5,
                    vector: Some(0.4),
                    graph: None,
                    recency: 0.3,
                    importance: 0.5,
                    diversity_key: String::new(),
                },
            ),
            (
                record(0x0b, "2025-12-02T00:00:00Z", 0.9),
                CandidateSignals {
                    exact: true,
                    full_text: 0.6,
                    vector: Some(0.2),
                    graph: None,
                    recency: 0.5,
                    importance: 0.5,
                    diversity_key: String::new(),
                },
            ),
        ]);
        let mut r = retriever(provider);
        let (scored, semantic) = r
            .retrieve_scored(&MemoryQuery::default(), &RetrievalSignals::all())
            .unwrap();
        assert!(semantic);
        assert_eq!(scored.len(), 2);
        // Exact tier first.
        assert_eq!(HybridScorer::exact_tier(&scored[0]), 1);
    }

    #[test]
    fn ep016_unit_retriever_tenant_mismatch_fails_closed() {
        let mut r = retriever(StubProvider(vec![]));
        let err = r
            .retrieve(
                "other-tenant",
                &MemoryQuery::default(),
                &RetrievalSignals::all(),
            )
            .unwrap_err();
        assert_eq!(err.code, nexus_context::ContextErrorCode::Authorization);
    }
}
