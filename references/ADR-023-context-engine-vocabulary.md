# ADR-023 - Context Engine Vocabulary

Status: Accepted
Date: 2026-08-15
Owner: hermes-nexus-main
Node: EP-016

## Context

EP-016 owns the context plane: hybrid retrieval, context capsules,
memory consolidation, retention, privacy, and graph-aware context
construction (node contract `.agent/node-contracts/EP-016.md`), in the
Rust crate `crates/nexus-context`. SPEC-002 locks the canonical terms
`MemoryRecord`, `MemoryProposal`, `MemoryType`, `Sensitivity`,
`RetentionPolicy`, `Provenance`, `Supersession`, `EmbeddingRef`,
`WorldGraphRepository`, `ContextCandidate`, and `ContextCapsule`; those
names are already implemented and re-exported from `nexus-data`,
`nexus-domain`, and `nexus-fabric` and are NEVER redefined here. SPEC-020
locks purpose limitation; INV-007 locks namespace isolation. EP-005 M1
doctrine requires every new public name to come from an accepted
vocabulary or be added by an ADR and schema update in the same
milestone.

## Decision

Add the following vocabulary-locked names to the EP-016 context plane
(SCREAMING_SNAKE_CASE serde, canonical `as_str`, strict `FromStr` that
rejects unknown classes):

- `ContextPurpose` - purpose limitation classes for context
  construction and memory proposals: `TASK_EXECUTION`, `PLANNING`,
  `SEARCH`, `NOTIFICATION`, `SYSTEM_MAINTENANCE`. A capsule may only
  carry data whose declared purpose permits the current use (SPEC-020).
- `GraphExpansionMode` - bounded graph expansion classes: `DIRECT`,
  `ONE_HOP`, `TWO_HOP`. Graph-aware context construction never expands
  past the declared hop bound (SPEC-002 behavior 7).
- `PrivacyFilterDecision` - per-candidate privacy filter outcome:
  `ALLOW`, `REDACT` (metadata only), `DENY`. Purpose limitation,
  sensitivity ceilings, permission, and namespace isolation are
  enforced by the filter (SPEC-020, INV-007).
- `ConsolidationMode` - semantic consolidation execution mode:
  `MODEL_ASSISTED`, `DETERMINISTIC_FALLBACK`, `SKIPPED`. Model-assisted
  consolidation is preferred; the deterministic fallback satisfies the
  same proposal contract when model evaluation is unavailable (node
  contract fallback). Models can never write canonical memory directly
  (SPEC-002 behavior 5).

New provider-neutral ports owned by EP-016: `ContextEngine`,
`HybridRetriever`, `MemoryConsolidator`, `PrivacyFilter`, and
`GraphExpansionPolicy`, with the request/outcome value types
(`ContextRequest`, `RetrievalSignals`, `ConsolidationRequest`,
`ConsolidationOutcome`, `FilteredCandidate`, `GraphExpansionRequest`,
`GraphExpansion`, `GraphNodeRef`, `GraphEdgeRef`) and the typed
`ContextError` / `ContextErrorCode` (SPEC-006 codes).

Re-exports (never redefined): `MemoryRecord`, `MemoryProposal`,
`MemoryQuery`, `MemoryCandidate`, `MemoryStatus`, `Sensitivity`,
`RetentionPolicy`, `RetentionUnit`, `EmbeddingRef`, `MemoryRepository`,
`WorldGraphRepository`, `VectorRepository`,
`PostgresWorldGraphRepository` (nexus-data), `MemoryType`, `NexusId`,
`TenantId` (nexus-domain), `ContextCapsule`, `CapsuleId`,
`CapsuleReference`, `CapsuleState`, `ContextCapsuleService`
(nexus-fabric), and `RetrievalPolicy`, `RetrievalBlend`,
`ProposalEvaluator`, `ProposalOutcome`, `RetentionEngine`,
`RetentionError`, `LifecycleEngine`, `LifecycleError` (nexus-memory).

## Alternatives

- Redefine `ContextCapsule` / `MemoryProposal` in `nexus-context`
  (rejected: SPEC-002 names are vocabulary locked; the existing
  implementations in nexus-fabric and nexus-data are canonical).
- Skip purpose-limitation vocabulary (rejected: SPEC-020 and the node
  contract acceptance obligation 1 require context to be
  purpose-limited; the enum is the deterministic carrier of that
  boundary).

## Consequences

- EP-016 callers get a single import surface for the full context and
  memory vocabulary.
- Context construction is purpose-limited, permission-filtered,
  bounded, and fail-closed; graph expansion never crosses a security
  boundary.
- Consolidation always emits proposals; canonical memory is only
  written through the policy evaluator.
- Reversal: revert the EP-016 M1 commit.
- Security: no credentials or sensitive content in error messages or
  telemetry; redaction is a first-class filter decision.
- License: no new dependency classes (workspace members only).
- Compatibility: additive crate + vocabulary; no existing surface
  changed.
