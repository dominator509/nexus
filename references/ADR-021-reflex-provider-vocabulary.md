# ADR-021 - Reflex Provider Vocabulary

Status: Accepted
Date: 2026-08-15
Owner: hermes-nexus-main
Node: EP-014

## Context

EP-014 owns the reflex plane: the DeepSeek V4 Flash ReflexProvider,
effort tiers, deterministic prompt segments, cache accounting, and
schema validation (node contract `.agent/node-contracts/EP-014.md`), in
the Rust crate `crates/nexus-reflex`. SPEC-009 already locks the
canonical terms `ReflexProvider`, `ModelGateway`, `ModelRoute`,
`NexusControlObject`, `EffortTier`, `PromptSegment`, `CacheHitRatio`,
`ProviderHealth`, `Escalation`, and `Microbrain` (ADR-018, owned by
`crates/nexus-model-gateway`). SPEC-009 requires DeepSeek V4 Flash as
the V1 primary ReflexProvider with deterministic effort tiers, stable
canonical prompt segments, a cache hit ratio target of at least 0.97 on
cacheable reflex traffic, and only validated `NexusControlObject`
output continuing. EP-005 M1 doctrine requires every new public name to
come from an accepted vocabulary or be added by an ADR and a schema
update in the same milestone.

## Decision

Add the following vocabulary-locked classes, owned by
`crates/nexus-reflex` and documented in `docs/vocabulary/README.md`:

- `ReflexDecisionClass` (SPEC-009): `DETERMINISTIC`, `MODEL` - how a
  reflex decision was produced. `DETERMINISTIC` means the model was
  bypassed; `MODEL` means the decision came from a real provider and
  passed validation.
- `EffortSelectionClass` (SPEC-009 required behavior 2): `POLICY_SELECTED`,
  `EXPLICIT` - how an effort tier was chosen. MAX is never the default
  for trivial work.

The following public names are provider-neutral ports and types owned by
`crates/nexus-reflex` (not vocabulary enums; they carry behavior):
`ReflexProvider`, `ReflexTransport`, `ReflexRequest`, `ReflexDecision`,
`DeepSeekFlashProvider`, `EffortPolicy`, `EffortInput`, `CacheLedger`,
`CacheRecord`, `PromptSegmentCatalog`, `PromptSegmentVersion`,
`StablePrefix`, and `NexusControlObjectValidator`. The canonical model
plane vocabulary (`EffortTier`, `PromptSegment`, `CacheHitRatio`,
`NexusControlObject`, `ProviderHealth`, `ProviderHealthState`,
`UsageReport`, `PromptSegmentPart`) is RE-EXPORTED from
`nexus-model-gateway`; it is not redefined.

Behavioral invariants recorded with this ADR:

1. Deterministic tasks (`EffortTier::Deterministic`) bypass the model.
2. Non-thinking, high, and max effort are policy selected; MAX is never
   the default for trivial work.
3. Stable prefix segments are canonical and versioned; volatile IDs and
   timestamps stay in the tail.
4. Rolling token cache-hit ratio is measured and targets at least 0.97
   on the cacheable corpus.
5. Only validated NexusControlObject output continues.

## Consequences

- The reflex plane has a single canonical vocabulary; DeepSeek V4 Flash
  is the V1 primary ReflexProvider but the port is replaceable.
- Deterministic tasks never consume model tokens; cache accounting is
  rolling and bounded.
- Model output is advisory only; authority remains with the canonical
  Nexus authorization path (EP-008).

## Compatibility

- New classes only; no existing vocabulary or schema changes.
- `nexus-reflex` depends only on `nexus-domain` and
  `nexus-model-gateway`; the transport is injected behind a port and no
  vendor SDK enters the production tree (dependency-direction guard).
