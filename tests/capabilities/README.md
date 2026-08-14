# tests/capabilities - EP-010 composed capability subsystem proofs

Owned by milestone `EP-010 M5` (manifest `.agent/milestone-files/EP-010-M5.txt`).

This package proves the REAL EP-010 contracts as ONE composed
subsystem:

- real `InMemoryCapabilityRegistry` (tenant-keyed, idempotent
  registration, availability-filtered discovery, no global fallback)
- real `CapabilityDispatcher` (typed class-specific entry points; no
  generic execute-string surface; provider error never coerced to allow)
- real `IdempotencyTracker` (key bound to capability; replay; conflict)
- real `CapabilityDescriptor` / `ConnectorManifest` contracts
- real typed `CapabilityError` (SPEC-006, correlation-preserving)
- real canonical JSON Schemas validated by the real `jsonschema`
  0.49.9 validator (draft 2020-12, hermetic local `$ref` resolution)

The single probe binary
`crates/nexus-connectors/examples/livefire_probe.rs` is orchestration
only. Every observed behavior comes from the production
implementations; deterministic providers in the probe are test-zone
doubles implementing the real capability port traits.

## Certification boundary (directive T)

EP-010 owns NO standalone external provider. The evidence therefore
states:

- Capability contract certification: PASS
- Deterministic registry/dispatcher certification: PASS
- Canonical schema parity: PASS
- Forced-failure behavior: PASS
- Composed EP-010 subsystem proof: PASS
- External connector/provider certification: NOT OWNED BY EP-010

Later connector nodes certify real providers through the same
contracts.

## Authority boundaries (directives N/O/K)

- A `CapabilityDescriptor` is metadata only; it carries no grant,
  token, credential, or authorization material.
- A `ConnectorManifest` tier / certification is metadata only; TIER3 /
  CERTIFIED never bypasses tenant isolation, availability,
  capability-class checking, idempotency, or downstream authorization.
- `HealthState::Healthy` is provider/runtime observation, never
  authorization, grant, or approval.
- EP-008 owns authorization to invoke. EP-005 owns the event transport
  substrate. EP-006 owns durable workflow execution. EP-010 owns
  capability description / discovery / dispatch mechanics.

## Test naming

`ep010_livefire_*` - selected by the M5 gate with
`-o python_functions="ep010_livefire_*"` plus a vacuity guard so a
zero-match filter can never print green.

## Running

```sh
cd /root/nexus
uv run --frozen pytest tests/capabilities -q --tb=native -o python_functions="ep010_livefire_*"
```

Evidence is accumulated by the suite from the real probe output and
written to `.agent/state/evidence/ep010-m5/` by the M5 gate.
