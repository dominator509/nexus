//! Memory consolidation workers (SPEC-002 behaviors 4-5; EP-016 M2).
//!
//! Candidate observations -> `MemoryProposal` -> proposal evaluation ->
//! canonical memory only if accepted. A model or consolidator can NEVER
//! write canonical memory directly. When semantic/model-assisted
//! consolidation is unavailable the worker disables it and uses
//! deterministic rules only; it never simulates a successful model
//! consolidation result.

use crate::util::sensitivity_rank;
use nexus_context::{
    ConsolidationMode, ConsolidationOutcome, ConsolidationRequest, ContextError, ContextErrorCode,
    MemoryConsolidator,
};
use nexus_data::memory::{MemoryProposal, MemoryRecord, MemoryStatus, Sensitivity};
use nexus_domain::{NexusId, TenantId};
use nexus_memory::{ProposalEvaluator, ProposalOutcome};
use std::collections::HashSet;

/// Source memory provider port (injected I/O): fetches the source
/// records named by a consolidation request.
pub trait SourceProvider {
    fn fetch(
        &mut self,
        tenant_id: &str,
        memory_ids: &[String],
    ) -> Result<Vec<MemoryRecord>, ContextError>;
}

/// Semantic (model-assisted) consolidator port (injected I/O). The
/// adapter calls a model outside the worker; `None` disables semantic
/// consolidation cleanly.
pub trait SemanticConsolidator {
    /// Produce a semantic consolidation result from source records.
    /// Returns `None` when semantic consolidation is unavailable.
    fn consolidate(
        &mut self,
        sources: &[MemoryRecord],
    ) -> Result<Option<serde_json::Value>, ContextError>;
}

/// Deterministic memory consolidator.
///
/// Policy (EP-016 Decision Log):
/// - The pipeline is proposal-before-canonical: this worker only emits
///   `MemoryProposal`s; promotion to `ACTIVE` happens in a repository
///   after `ProposalEvaluator` approval (never here).
/// - When a `SemanticConsolidator` is injected AND returns a result,
///   `ConsolidationMode::ModelAssisted` is reported and the semantic
///   content is used.
/// - When semantic consolidation is unavailable (no adapter, or the
///   adapter returned `None`), `ConsolidationMode::DeterministicFallback`
///   is used: deterministic merging of source records. No model result
///   is simulated.
/// - Sensitivity of the proposal never exceeds the source maximum and
///   never exceeds the request sensitivity.
/// - Duplicate/conflicting proposals follow canonical proposal policy:
///   the evaluator decides; the worker never mutates canonical memory.
pub struct DeterministicMemoryConsolidator<S> {
    pub sources: S,
    pub semantic: Option<Box<dyn SemanticConsolidator>>,
    pub evaluator: ProposalEvaluator,
    /// Dedupe set of already-emitted proposal content hashes, so a
    /// duplicate consolidation request never re-emits an identical
    /// proposal (idempotency).
    emitted_hashes: HashSet<String>,
}

impl<S: SourceProvider> DeterministicMemoryConsolidator<S> {
    pub fn new(sources: S, semantic: Option<Box<dyn SemanticConsolidator>>) -> Self {
        Self {
            sources,
            semantic,
            evaluator: ProposalEvaluator::new(),
            emitted_hashes: HashSet::new(),
        }
    }

    /// Deterministic fallback: merge source records into one proposal.
    fn deterministic_proposal(
        &self,
        request: &ConsolidationRequest,
        records: &[MemoryRecord],
    ) -> Result<MemoryProposal, ContextError> {
        if records.is_empty() {
            return Err(ContextError::new(
                ContextErrorCode::NotFound,
                "consolidation source records not found",
                Some(request.correlation_id.clone()),
                Some(request.principal_id.clone()),
                Some(request.tenant_id.clone()),
                Some("memory-consolidator".into()),
            ));
        }
        let tenant = TenantId::new(request.tenant_id.clone()).map_err(|_| {
            ContextError::validation("invalid tenant id in consolidation request", None)
        })?;
        let memory_id = deterministic_proposal_id(request);
        // Merge: collect all content keys; confidence is the minimum
        // source confidence (conservative); derived_from preserves the
        // full provenance chain.
        let mut merged = serde_json::Map::new();
        let mut confidence = 1.0_f64;
        let mut derived: Vec<NexusId> = Vec::new();
        let mut observed_at = String::new();
        let mut source_max_rank = 0u8;
        for record in records {
            if let serde_json::Value::Object(map) = &record.content {
                for (k, v) in map {
                    merged.insert(k.clone(), v.clone());
                }
            }
            confidence = confidence.min(record.confidence);
            derived.push(record.memory_id.clone());
            if record.observed_at > observed_at {
                observed_at = record.observed_at.clone();
            }
            source_max_rank = source_max_rank.max(sensitivity_rank(record.sensitivity));
        }
        // Sensitivity: never above the request ceiling nor the source
        // maximum.
        let request_rank = sensitivity_rank(request.sensitivity);
        let effective_rank = request_rank.min(source_max_rank);
        let sensitivity = sensitivity_from_rank(effective_rank);
        let content = serde_json::Value::Object(merged);
        let content_hash = crate::util::fnv1a64(&content.to_string());
        let record = MemoryRecord {
            memory_id,
            tenant_id: tenant,
            namespace: "consolidated".into(),
            memory_type: request.target_type,
            content,
            content_hash: format!("{content_hash:064x}"),
            source: "memory-consolidator".into(),
            actor: request.principal_id.clone(),
            created_at: observed_at.clone(),
            observed_at,
            confidence,
            sensitivity,
            purpose: request.purpose.as_str().into(),
            retention: request.retention,
            status: MemoryStatus::Proposed,
            derived_from: derived,
            supersedes: None,
            embedding_ref: None,
        };
        Ok(MemoryProposal { record })
    }

    /// Run the consolidation pipeline. Never mutates canonical memory.
    pub fn consolidate_evaluated(
        &mut self,
        request: &ConsolidationRequest,
    ) -> Result<ConsolidationOutcome, ContextError> {
        request.validate()?;
        let records = self
            .sources
            .fetch(&request.tenant_id, &request.source_memory_ids)?;

        // Semantic consolidation: only when an adapter exists AND returns
        // a real result. Otherwise deterministic fallback.
        let mut mode = ConsolidationMode::DeterministicFallback;
        let mut proposal = self.deterministic_proposal(request, &records)?;

        if let Some(semantic) = self.semantic.as_deref_mut() {
            let semantic_content = semantic.consolidate(&records)?;
            if let Some(semantic_content) = semantic_content {
                mode = ConsolidationMode::ModelAssisted;
                proposal.record.content = semantic_content;
                proposal.record.content_hash = format!(
                    "{:064x}",
                    crate::util::fnv1a64(&proposal.record.content.to_string())
                );
                proposal.record.confidence =
                    records.iter().map(|r| r.confidence).fold(1.0_f64, f64::min);
            }
        }
        // Idempotency: identical proposal content is emitted once per
        // consolidator instance.
        if !self
            .emitted_hashes
            .insert(proposal.record.content_hash.clone())
        {
            return Ok(ConsolidationOutcome {
                proposals: vec![],
                mode,
            });
        }

        Ok(ConsolidationOutcome {
            proposals: vec![proposal],
            mode,
        })
    }

    /// Expose the evaluation of a proposal without promoting it. This is
    /// the canonical proposal-before-canonical gate.
    pub fn evaluate_proposal(&self, proposal: &MemoryProposal) -> ProposalOutcome {
        self.evaluator
            .evaluate(&proposal.record)
            .unwrap_or(ProposalOutcome::Rejected)
    }
}

impl<S: SourceProvider> MemoryConsolidator for DeterministicMemoryConsolidator<S> {
    fn consolidate(
        &mut self,
        request: &ConsolidationRequest,
    ) -> Result<ConsolidationOutcome, ContextError> {
        self.consolidate_evaluated(request)
    }
}

/// Deterministic proposal id derived from request identity so repeated
/// consolidation of the same request is idempotent.
fn deterministic_proposal_id(request: &ConsolidationRequest) -> NexusId {
    let mut source = request.request_id.clone();
    for id in &request.source_memory_ids {
        source.push(':');
        source.push_str(id);
    }
    // Deterministic UUIDv7-shaped id from the stable digest.
    let digest = crate::util::fnv1a64(&source);
    let hex = format!("0190e1c4-5c8a-7f40-8a1b-{:012x}", digest & 0xffff_ffff_ffff);
    NexusId::new(hex)
        .unwrap_or_else(|_| NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01").unwrap())
}

fn sensitivity_from_rank(rank: u8) -> Sensitivity {
    match rank {
        0 => Sensitivity::Public,
        1 => Sensitivity::Household,
        2 => Sensitivity::Personal,
        3 => Sensitivity::Sensitive,
        4 => Sensitivity::BusinessConfidential,
        5 => Sensitivity::Security,
        _ => Sensitivity::Secret,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_data::memory::{
        MemoryRecord, MemoryStatus, RetentionPolicy, RetentionUnit, Sensitivity,
    };
    use nexus_domain::{MemoryType, NexusId};

    fn source_record(id_byte: u8, confidence: f64, sensitivity: Sensitivity) -> MemoryRecord {
        MemoryRecord {
            memory_id: NexusId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a{id_byte:02x}"))
                .unwrap(),
            tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a80").unwrap(),
            namespace: "household".into(),
            memory_type: MemoryType::Episodic,
            content: serde_json::json!({ "fact": id_byte }),
            content_hash: format!("{:064x}", id_byte),
            source: "test".into(),
            actor: "p-1".into(),
            created_at: "2025-12-01T00:00:00Z".into(),
            observed_at: "2025-12-01T00:00:00Z".into(),
            confidence,
            sensitivity,
            purpose: "remember".into(),
            retention: RetentionPolicy::for_duration(RetentionUnit::Days, 30),
            status: MemoryStatus::Active,
            derived_from: vec![],
            supersedes: None,
            embedding_ref: None,
        }
    }

    struct StubSources(Vec<MemoryRecord>);

    impl SourceProvider for StubSources {
        fn fetch(
            &mut self,
            _tenant_id: &str,
            memory_ids: &[String],
        ) -> Result<Vec<MemoryRecord>, ContextError> {
            Ok(self
                .0
                .iter()
                .filter(|r| memory_ids.iter().any(|id| id == r.memory_id.as_str()))
                .cloned()
                .collect())
        }
    }

    fn request() -> ConsolidationRequest {
        request_with_sources(&[
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a01".to_string(),
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a02".to_string(),
        ])
    }

    fn request_with_sources(source_ids: &[String]) -> ConsolidationRequest {
        ConsolidationRequest {
            request_id: "req-1".into(),
            correlation_id: "corr-1".into(),
            tenant_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a80".into(),
            principal_id: "p-1".into(),
            source_memory_ids: source_ids.to_vec(),
            target_type: MemoryType::Semantic,
            sensitivity: Sensitivity::Sensitive,
            purpose: nexus_context::ContextPurpose::TaskExecution,
            retention: RetentionPolicy::for_duration(RetentionUnit::Days, 30),
        }
    }

    #[test]
    fn ep016_unit_consolidation_emits_proposal_never_canonical() {
        let sources = StubSources(vec![
            source_record(0x01, 0.9, Sensitivity::Household),
            source_record(0x02, 0.8, Sensitivity::Household),
        ]);
        let mut c = DeterministicMemoryConsolidator::new(sources, None);
        let outcome = c.consolidate(&request()).unwrap();
        assert_eq!(outcome.mode, ConsolidationMode::DeterministicFallback);
        assert_eq!(outcome.proposals.len(), 1);
        let proposal = &outcome.proposals[0];
        assert_eq!(proposal.record.status, MemoryStatus::Proposed);
        // Conservative confidence: min of sources.
        assert_eq!(proposal.record.confidence, 0.8);
        // Provenance preserved.
        assert_eq!(proposal.record.derived_from.len(), 2);
    }

    #[test]
    fn ep016_unit_consolidation_rejected_proposal_not_canonicalized() {
        let sources = StubSources(vec![
            source_record(0x03, 0.9, Sensitivity::Secret),
            source_record(0x04, 0.9, Sensitivity::Secret),
        ]);
        let req = request_with_sources(&[
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a03".to_string(),
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a04".to_string(),
        ]);
        let mut c = DeterministicMemoryConsolidator::new(sources, None);
        let outcome = c.consolidate(&req).unwrap();
        let proposal = &outcome.proposals[0];
        // Proposal sensitivity is capped below the request ceiling by the
        // source maximum; a Secret proposal fails auto-approval.
        let decision = c.evaluate_proposal(proposal);
        assert_eq!(decision, ProposalOutcome::Rejected);
        // The record remains PROPOSED; nothing was promoted.
        assert_eq!(proposal.record.status, MemoryStatus::Proposed);
    }

    #[test]
    fn ep016_unit_consolidation_duplicate_request_idempotent() {
        let sources = StubSources(vec![
            source_record(0x05, 0.9, Sensitivity::Household),
            source_record(0x06, 0.9, Sensitivity::Household),
        ]);
        let req = request_with_sources(&[
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a05".to_string(),
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a06".to_string(),
        ]);
        let mut c = DeterministicMemoryConsolidator::new(sources, None);
        let first = c.consolidate(&req).unwrap();
        assert_eq!(first.proposals.len(), 1);
        // Identical request: no duplicate proposal re-emitted.
        let second = c.consolidate(&req).unwrap();
        assert!(second.proposals.is_empty());
    }

    struct NoSemantic;

    impl SemanticConsolidator for NoSemantic {
        fn consolidate(
            &mut self,
            _sources: &[MemoryRecord],
        ) -> Result<Option<serde_json::Value>, ContextError> {
            Ok(None)
        }
    }

    #[test]
    fn ep016_unit_semantic_unavailable_disables_cleanly() {
        let sources = StubSources(vec![
            source_record(0x07, 0.9, Sensitivity::Household),
            source_record(0x08, 0.9, Sensitivity::Household),
        ]);
        let req = request_with_sources(&[
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a07".to_string(),
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a08".to_string(),
        ]);
        let mut c = DeterministicMemoryConsolidator::new(sources, Some(Box::new(NoSemantic)));
        let outcome = c.consolidate(&req).unwrap();
        // Semantic adapter exists but returned None: deterministic
        // fallback, never a simulated model result.
        assert_eq!(outcome.mode, ConsolidationMode::DeterministicFallback);
        assert_eq!(outcome.proposals.len(), 1);
        assert!(outcome.proposals[0].record.content.is_object());
    }

    struct YesSemantic;

    impl SemanticConsolidator for YesSemantic {
        fn consolidate(
            &mut self,
            _sources: &[MemoryRecord],
        ) -> Result<Option<serde_json::Value>, ContextError> {
            Ok(Some(serde_json::json!({ "semantic": true })))
        }
    }

    #[test]
    fn ep016_unit_model_assisted_only_when_actually_exercised() {
        let sources = StubSources(vec![
            source_record(0x09, 0.9, Sensitivity::Household),
            source_record(0x0a, 0.9, Sensitivity::Household),
        ]);
        let req = request_with_sources(&[
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a09".to_string(),
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7a0a".to_string(),
        ]);
        let mut c = DeterministicMemoryConsolidator::new(sources, Some(Box::new(YesSemantic)));
        let outcome = c.consolidate(&req).unwrap();
        assert_eq!(outcome.mode, ConsolidationMode::ModelAssisted);
        assert_eq!(
            outcome.proposals[0].record.content,
            serde_json::json!({ "semantic": true })
        );
    }
}
