# EP-012 fabric tests

Test zone for the `nexus-fabric` contract crate (SPEC-003, EP-012).

## Layout

- `crates/nexus-fabric/src/**` - in-module `ep012_unit_*` tests for
  construction, validation, serialization, and vocabulary rejection.
- `crates/nexus-fabric/tests/dependency_direction.rs` - `ep012_unit_*`
  dependency-direction guard (SPEC-001): no infrastructure, network,
  HTTP, or vendor crate in the fabric production tree.
- Later milestones add `tests/fabric/` integration and failure suites
  against real adapters (REST/WebSocket/MCP/A2A), always over real
  transports, never in-process mocks on the proven path.

## Vocabulary discipline

All new public names come from accepted vocabularies or are added by an
ADR (ADR-017) and a `docs/vocabulary/README.md` update in the same
milestone. Unknown vocabulary values fail closed at the boundary.

## Test naming

- `ep012_unit_*` - unit tests
- `ep012_integration_*` - integration tests (later milestones)
- `ep012_failure_*` - failure/abuse tests (later milestones)

Gate selection: `cargo test --locked -p nexus-fabric ep012_unit` (M1/M2),
`ep012_integration` (M3), `ep012_failure` (M4).
