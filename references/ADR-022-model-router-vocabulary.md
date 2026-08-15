# ADR-022 - Model Router Vocabulary

Status: Accepted
Date: 2026-08-15
Owner: hermes-nexus-main
Node: EP-015

## Context

EP-015 owns the model router plane: the Nexus Model Router contract,
policy routing, RouteLLM-compatible scoring, escalation, and the
Microbrain interface (node contract `.agent/node-contracts/EP-015.md`),
in the Rust crate `crates/nexus-model-router`. SPEC-009 already locks the
canonical terms `ModelRoute`, `Escalation`, `Microbrain` and requires
router inputs to include domain, complexity, privacy, risk, capability,
cost, latency, locality, availability, historical success, certification,
and budget. SPEC-025 locks `Microbrain`, `ShadowDecision`,
`PromotionGate`, and `OutOfDistribution`, and requires the Microbrain to
use the same ReflexProvider contract, begin in shadow, and be able to
remain disabled. EP-005 M1 doctrine requires every new public name to
come from an accepted vocabulary or be added by an ADR and a schema
update in the same milestone.

## Decision

Add the following vocabulary-locked classes, owned by
`crates/nexus-model-router` and documented in `docs/vocabulary/README.md`:

- `RoutingDecisionClass` (SPEC-009 canonical term ModelRoute): `ROUTED`,
  `FALLBACK`, `ESCALATED`, `REJECTED`, `SHADOW` - how a routing decision
  was produced. `REJECTED` never routes to a model.
- `RouterStrategyClass` (SPEC-009: RouteLLM/LLMRouter replaceable):
  `POLICY`, `ROUTE_LLM`, `LLM_ROUTER`, `MICROBRAIN` - the strategy that
  produced a routing decision. The policy engine can override learned
  routing for security.
- `EscalationReason` (SPEC-009 canonical term Escalation): `AMBIGUITY`,
  `RISK`, `PRIVACY`, `BUDGET`, `UNAVAILABLE`, `COST`, `LATENCY`,
  `SECURITY`, `CERTIFICATION`, `OUT_OF_DISTRIBUTION` - deterministic
  escalation causes.
- `MicrobrainState` (SPEC-025): `DISABLED`, `SHADOW`, `CANARY`,
  `ACTIVE`, `PROMOTION_GATED` - the Microbrain promotion lifecycle. The
  safe default is `DISABLED`.
- `ShadowDecisionClass` (SPEC-025 canonical term ShadowDecision):
  `MATCH`, `DIVERGE`, `FAILED` - a shadow comparison outcome. A failed
  shadow is never trusted.

The following public names are provider-neutral ports and types owned by
`crates/nexus-model-router` (not vocabulary enums; they carry behavior):
`NexusModelRouter`, `RoutingFeatures`, `RoutingDecision`, `RoutePolicy`,
`LearnedRouterAdapter`, `LearnedScores`, `MicrobrainProvider`,
`DisabledMicrobrain`, `EscalationPolicy`, `EscalationOutcome`,
`RouterError`, `RouterErrorCode`.

The canonical `Route`, `Risk`, and `Privacy` classes are re-exported from
`nexus-domain` (never redefined); `EffortTier`, `ProviderHealth`,
`ProviderHealthState`, and `CacheHitRatio` are re-exported from
`nexus-model-gateway`; the `ReflexProvider` and `ReflexRequest` contracts
are re-exported from `nexus-reflex` so the Microbrain seam uses the SAME
ReflexProvider contract (SPEC-009 behavior 9).

## Consequences

- Deterministic policy routing is the V1 default; a learned router must
  beat the frozen benchmark before it can replace policy (node contract
  fallback).
- Learned scorers and the Microbrain are advisory only; the policy
  engine overrides them for security (acceptance obligation 3).
- The Microbrain begins `DISABLED`; promotion is gated by later SPEC-025
  nodes (training factory, frozen evals, shadow, canary).
- Reversal: revert the EP-015 M1 commit.
- Security: routing is a deterministic control-plane decision; no model
  output can mint a route or override policy.
- License: no new dependency classes (serde/serde_json already
  workspace-pinned; nexus-domain/nexus-model-gateway/nexus-reflex are
  workspace members).
- Compatibility: additive workspace member; no existing surface changed.
