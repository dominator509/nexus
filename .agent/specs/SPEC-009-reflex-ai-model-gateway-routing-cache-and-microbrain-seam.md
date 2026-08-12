# SPEC-009 - Reflex AI, Model Gateway, Routing, Cache, and Microbrain Seam

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define ReflexProvider, NexusControlObject, DeepSeek V4 Flash, Bifrost preference, model routing, cache discipline, budgets, privacy, and Microbrain replacement.

## Canonical terms

ReflexProvider, ModelGateway, ModelRoute, NexusControlObject, EffortTier, PromptSegment, CacheHitRatio, ProviderHealth, Escalation, Microbrain. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. DeepSeek V4 Flash is V1 primary ReflexProvider; Bifrost is preferred ModelGateway but the contract supports replacement.
2. Effort tiers are deterministic, non-thinking, high, max, and specialist. Max is never the default for trivial work.
3. Every provider returns the same NexusControlObject schema; deterministic validation rejects extra or invalid fields.
4. Prompt segments are ordered from immutable constitution through schemas, capability taxonomy, risk policy, examples, stable tenant context, session context, and dynamic request.
5. Canonical serialization fixes key ordering, whitespace, schema ordering, tool ordering, and segment versions. Volatile IDs and timestamps stay in the tail.
6. Cache hit ratio is hit prompt tokens divided by total prompt tokens and targets at least 0.97 on cacheable reflex traffic.
7. Router inputs include domain, complexity, privacy, risk, capability, cost, latency, locality, availability, historical success, certification, and budget.
8. Private data egress requires policy. An uncensored content profile may use Venice or xAI only when configured and permitted.
9. Microbrain uses the same ReflexProvider contract, begins in shadow, passes frozen and adversarial evals, then canaries low-risk traffic with DeepSeek fallback.
10. Models cannot grant scopes, approve actions, modify policies, reveal secrets, or bypass output validation.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- One model for every task
- Local GGUF as mandatory
- Max thinking for all requests
- Training on private data without opt-in
- Model confidence as authorization

## Required tests

- Control object schema fuzzing
- Prompt byte stability
- Cache replay at 0.97
- Provider fallback
- Privacy route denial
- Budget cap
- Router policy table
- Microbrain shadow comparison

## Acceptance

Reflex traffic produces valid control objects, cache and cost telemetry, deterministic escalation, safe failover, and no authority side effect.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
