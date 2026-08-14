# ADR-018 - Model Gateway and Provider Registry Vocabulary

Status: Accepted
Date: 2026-08-14
Owner: hermes-nexus-main

## Context

EP-013 owns the model plane: the model provider registry, Bifrost-
preferred gateway adapter, budgets, fallbacks, and provider health
(node contract `.agent/node-contracts/EP-013.md`), in the Rust crate
`crates/nexus-model-gateway`. SPEC-009 locks the canonical terms
`ReflexProvider`, `ModelGateway`, `ModelRoute`, `NexusControlObject`,
`EffortTier`, `PromptSegment`, `CacheHitRatio`, `ProviderHealth`,
`Escalation`, and `Microbrain`. SPEC-009 requires DeepSeek V4 Flash as
the V1 primary ReflexProvider with Bifrost preferred as the ModelGateway
but replaceable, deterministic effort tiers, the same
`NexusControlObject` schema from every provider, ordered prompt
segments, a cache hit ratio target of at least 0.97 on cacheable reflex
traffic, deterministic escalation, and models that never grant
authority. EP-005 M1 doctrine requires every new public name to come
from an accepted vocabulary or be added by an ADR and a schema update
in the same milestone.

## Decision

Add the following vocabulary-locked classes, owned by
`crates/nexus-model-gateway` and documented in
`docs/vocabulary/README.md`:

- `EffortTier` (SPEC-009 required behavior 2): `DETERMINISTIC`,
  `NON_THINKING`, `HIGH`, `MAX`, `SPECIALIST` - ordered; MAX is never
  the default for trivial work.
- `ProviderKind` (SPEC-009; EP-013): `BIFROST`, `DEEPSEEK`,
  `OPENAI_COMPATIBLE`, `VENICE`, `XAI` - the adapter families; Bifrost
  is preferred but replaceable.
- `ProviderHealthState` (SPEC-009 canonical term `ProviderHealth`):
  `HEALTHY`, `DEGRADED`, `UNHEALTHY`, `UNKNOWN` - observed provider
  health; unknown fails closed.
- `Escalation` (SPEC-009 canonical term `Escalation`): `NONE`,
  `RETRY`, `FAILOVER`, `HUMAN`, `DISABLE` - deterministic escalation on
  provider failure or policy denial.
- `Microbrain` (SPEC-009 canonical term `Microbrain`): `SHADOW`,
  `FROZEN`, `CANARY`, `ACTIVE` - the microbrain lifecycle; it begins in
  shadow, passes frozen and adversarial evals, then canaries low-risk
  traffic with DeepSeek fallback.
- `ModelRouteClass` (SPEC-009 canonical term `ModelRoute`): `DIRECT`,
  `CACHED`, `FALLBACK`, `ESCALATED` - the resolved route decision
  class.
- `ModelGatewayClass` (SPEC-009 canonical term `ModelGateway`):
  `REFLEX`, `BIFROST`, `DIRECT` - gateway implementation class.
- `ReflexProviderClass` (SPEC-009 canonical term `ReflexProvider`):
  `DEEPSEEK_V4_FLASH`, `BIFROST`, `CUSTOM` - primary reflex provider
  class; DeepSeek V4 Flash is V1 primary.
- `CacheHitRatio` (SPEC-009 canonical term `CacheHitRatio`): hit prompt
  tokens divided by total prompt tokens; the cacheable reflex traffic
  target is at least 0.97.
- `PromptSegment` (SPEC-009 required behavior 4): `CONSTITUTION`,
  `SCHEMAS`, `CAPABILITY_TAXONOMY`, `RISK_POLICY`, `EXAMPLES`,
  `TENANT_CONTEXT`, `SESSION_CONTEXT`, `DYNAMIC_REQUEST` - ordered from
  immutable constitution through dynamic request; volatile IDs and
  timestamps stay in the tail.

Every enum parses from its canonical SCREAMING_SNAKE_CASE wire string
and rejects unknown values (fail closed). Ports (`ModelProvider`,
`ModelGateway`, `ProviderRegistry`, `ProviderHealth`, `ModelBudget`,
`ModelRequest`, `ModelResponse`, `ToolCallEnvelope`) are
provider-neutral; provider credentials never leave the gateway; models
never grant scopes, approve actions, modify policies, reveal secrets,
or bypass output validation (SPEC-009 required behavior 10).

## Consequences

- The model plane has a single canonical vocabulary; providers are
  replaceable behind the gateway contract.
- Budgets, retries, rate limits, fallbacks, and usage accounting are
  consistent across every route.
- Authority remains with the canonical Nexus authorization path
  (EP-008); model output is advisory only.

## Compatibility

- New classes only; no existing vocabulary or schema changes.
- No dependency changes; the contract crate depends only on
  `nexus-domain` and `nexus-identity`.
