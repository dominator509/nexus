# ADR-019 - Control Plane Runtime Vocabulary

Status: Accepted
Date: 2026-08-14
Owner: hermes-nexus-main
Node: EP-044

## Context

EP-044 introduces the Nexus Control Plane Runtime. The canonical vocabulary
(SPEC-003, SPEC-005, SPEC-006, SPEC-007, SPEC-022) does not yet lock the
runtime-level public names: the config surface, the health/readiness
response shapes, the capability list response, and the runtime lifecycle and
smoke contracts. Per SPEC-005, new public names require an ADR and a
vocabulary/schema update in the same milestone.

## Decision

Add the following canonical runtime terms to `docs/vocabulary/README.md` and
enforce them in the `nexus-control-plane` crate vocabulary module:

- `ControlPlaneConfig`: canonical runtime configuration (base domain/URL,
  bind address, tenant, capability list source). Provider-neutral.
- `RuntimeHealth`: canonical `/healthz` response shape; must serialize as
  `{"status":"healthy"}` when healthy (SPEC-006 health contract).
- `RuntimeReadiness`: canonical `/readyz` response shape; must serialize as
  `{"ready":true}` when ready.
- `CapabilityList`: canonical `/v1/capabilities` response shape; must
  serialize as `{"capabilities":[...]}` with a non-empty list when the
  runtime is ready.
- `ControlPlaneServer`: the runnable server boundary (bind, routes, serve).
- `RuntimeLifecycle`: graceful startup/shutdown contract.
- `RuntimeSmoke`: the canonical runtime smoke contract, owned by EP-044.

These names are vocabulary locked. A new synonym requires an ADR and a schema
update in the same milestone.

## Consequences

The runtime endpoints and config surface have stable, vocabulary-locked
names. The smoke gate (`scripts/smoke/runtime.sh`) and the server crate share
the same canonical response shapes, preventing drift between the gate and the
implementation.

## Compatibility

Additive. No existing public surface changes. Prior green tags and ledger
history preserved.
