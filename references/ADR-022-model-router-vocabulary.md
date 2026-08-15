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

---

## EP-015 M5: Provider failover plane (LF-021)

### Decision

Add a production provider-failover surface to `crates/nexus-model-router`
so the LF-021 model-provider-failover live-fire proof exercises the real
router instead of a proof harness that calls providers directly:

- `ProviderFailoverPolicy` (`crates/nexus-model-router/src/failover.rs`),
  config-driven from the canonical `config/models/router/policy.json`
  `failover` section (`max_provider_attempts`, `attempt_cost`,
  `attempt_latency_ms`). Only `UNAVAILABLE` and `TIMEOUT` typed failures
  are failover-eligible; contract, rate, policy, budget, and security
  failures never cause provider hopping.
- `DeterministicModelRouter::route_with_failover`: the deterministic
  router selects the primary route, the primary `ReflexProvider` is
  attempted through its real transport, a typed failover-eligible
  failure selects the configured secondary provider, which is attempted
  with the remaining budgets (never a fresh cap), the same Nexus
  trace/correlation id, and the same canonical `NexusControlObject`
  validation contract. Security policy dominates availability (the
  secondary tier must pass the same `RoutePolicy::override_security` and
  `EscalationPolicy` surfaces); every path fails closed with bounded
  attempts; no provider cycling, no fabricated control object.
- Vocabulary: `ProviderFailureClass`, `FailoverStage` (spec-006 audit
  chain). `RouteAuditRecord` gains `stage` and `failure_class` fields
  (additive; plain routing decisions emit `None`).
- `scripts/live-fire/LF-021.sh` was rewritten from a stub that delegated
  to a nonexistent `nexus-cli` proof runner (the workspace has no CLI;
  EP-006/EP-008 precedent: LF-017.sh/LF-003.sh) to directly run the
  committed live-fire suite `crates/nexus-model-router/tests/lf021.rs`
  (8 tests) with a vacuity guard and governed evidence.

### Ownership resolution (LF-021)

EP-015 owns NO CLI. `scripts/proof-runner.sh` delegates to
`nexusctl`/`nexus-cli proof run`, but no `nexus-cli` crate exists in the
workspace and `apps/` contains only the control plane. The established
precedent for stubbed live-fire scripts (LF-003 by EP-008, LF-017 by
EP-006) is a direct invocation of an EP-015-owned real proof harness.
No global proof-runner change was made.

### Configured providers

Canonical registry `config/models/providers/providers.json`:
`deepseek-v4-flash` (DEEPSEEK; ReflexProvider primary) and `bifrost`
(BIFROST preferred gateway, not implemented). The primary proof uses
the production `DeepSeekFlashProvider` + `DeepSeekReflexTransport`; the
secondary is a production `DeepSeekFlashProvider` adapter instance at a
real isolated HTTP endpoint (instance label `deepseek-v4-flash-secondary`).
External DeepSeek/secondary vendor certification: NOT ASSERTED.

### Consequences

- Failover is deterministic, config-driven, bounded, and auditable; a
  failed primary attempt consumes the configured per-attempt cost and
  latency budgets.
- Security policy outranks availability; a prohibited secondary is never
  used.
- Budgets carry forward; failover never resets the request caps.
- Reversal: revert the EP-015 M5 commit.
- Security: audit records remain redacted (metadata only); no
  credential, prompt, or feature domain is emitted.
- License: no new dependency classes (workspace members only).
- Compatibility: additive module + fields; existing routing and audit
  behavior unchanged (decision records emit `stage: None`).
