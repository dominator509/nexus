# NEXUS CERTIFICATION REGISTRY

Machine-readable certification registry (OWNER ARCHITECTURE DIRECTIVE
section 7). One component per block, `key: value` lines, ASCII only.

## Status vocabulary

- NOT_IMPLEMENTED
- IMPLEMENTED
- INTERNAL_CERTIFIED
- PROVIDER_CERTIFIED
- HARDWARE_CERTIFIED
- PRODUCTION_CERTIFIED
- DEFERRED

Implementation, integration, and certification are tracked separately.
A component may be IMPLEMENTED before its external certification exists;
that is not simulation. DEFERRED rows must name a certification_owner.
At the ship gate (SPEC-008; EP-040/EP-043) every required capability
must reach its required level: PROVIDER_CERTIFIED for required external
providers, HARDWARE_CERTIFIED for required physical hardware,
PRODUCTION_CERTIFIED for the core runtime. IMPLEMENTED or DEFERRED rows
that are blocking_for_ship=true fail the ship gate.

Update rule: append or edit rows only with ledger evidence. This file
lives under .agent/state/evidence so every node may maintain its own
rows without fence churn.

## Components

## Component: nexus-context
component_id: nexus-context
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: none (provider-neutral ports by design)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-context; EP-016 M1 gate (24 ep016_unit tests + 1 dependency-direction); scope audit EP-016: ok

## Component: nexus-memory-workers
component_id: nexus-memory-workers
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: none (candidate/source/graph/semantic I/O injected through ports by design)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-memory-workers; EP-016 M2 gate (58 unit tests + 1 dependency-direction, 3 suites); clippy -D warnings clean; lint: ok

## Component: memory-workflow-contracts
component_id: memory-workflow-contracts
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: none (Temporal workflow contracts; real engine integration owned by the Temporal runtime node EP-006)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: packages/workflows/src/memory/; EP-016 M3 gate (14 ep016_integration tests via real vitest + vacuity guard + tsc --noEmit clean)

## Component: memory-plane-real-composition
component_id: memory-plane-real-composition
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: postgresql, pgvector, temporal (open-source infrastructure)
provider_certification: INTERNAL_CERTIFIED (real containers/services proved by owning nodes EP-004/EP-006)
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: EP-004 (PostgreSQL, pgvector, repositories, memory records, world graph); EP-006 (Temporal durable workflows); EP-016 worker ports consume these at the composition boundary
graph_gap_note: No node contract explicitly names composing the EP-016 context workers with the real EP-004 repositories and EP-006 Temporal runtime; the earliest consuming node per graph direction (agent orchestration and downstream) is the natural owner. EP-040/EP-043 must confirm an explicit integration owner at ship-gate review or add a certification/integration node (directive section 6).

## Component: agent-workflow-contracts
component_id: agent-workflow-contracts
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: none (Temporal workflow contracts; real engine integration owned by the Temporal runtime node EP-006)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship); real Temporal execution owned by EP-006 substrate, explicit agent-workflow composition owner to be confirmed at ship-gate review (EP-040/EP-043)
blocking_for_ship: false
evidence_reference: packages/workflows/src/agents/; EP-017 M3 gate (10 ep017_integration tests via real vitest + vacuity guard + tsc --noEmit clean); TypeScript workflow/state logic executed under real Vitest, NOT against a real Temporal server (no fake Temporal client; real engine integration deferred per EP-006 ownership)

## Component: control-plane-runtime
component_id: control-plane-runtime
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: none (self-hosted runtime)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: apps/control-plane; infra/compose/core.yaml; Dockerfile; /healthz /readyz /v1/capabilities real handlers; EP-044-M5-live-fire.md (real container, HTTP 200 bodies, local stop, no orphan, LF-029 regression ok)

## Component: deepseek-reflex
component_id: deepseek-reflex
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: deepseek (deepseek-v4-flash)
provider_certification: PROVIDER_CERTIFIED
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-reflex; EP-014-M5-live-fire.md (real provider route deepseek-v4-flash, 8 canonical requests, mandatory runtime smoke real container PASS)

## Component: model-gateway-provider-registry
component_id: model-gateway-provider-registry
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: bifrost (internal gateway 127.0.0.1:8000), deepseek fallback
provider_certification: INTERNAL_CERTIFIED for bifrost (internal infrastructure); deepseek-v4-flash fallback PROVIDER_CERTIFIED via EP-014
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-model-gateway, nexus-model-transport, nexus-bifrost; ep013-m5-live-fire.json (allow/deny paths, budget, rate limit, usage accounting, real transport)

## Component: model-router-microbrain-seam
component_id: model-router-microbrain-seam
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: reflex providers (DeepSeek, Microbrain via ReflexProvider contract)
provider_certification: INTERNAL_CERTIFIED (real transport attempt, connection-refused -> UNAVAILABLE, failover typed lock; real provider certification deferred to provider owner nodes)
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-router seam; LF-021-ep015-m5.md (real transport attempt; only UNAVAILABLE/TIMEOUT failover-eligible; typed lock)
