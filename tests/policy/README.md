# EP-008 policy test zone

Owned by EP-008 (authorization policy and action gateway). This
directory holds the real-dependency test suites for the policy stack:

- `ep008_integration_*` - real OpenFGA + OPA containers (M3).
- `ep008_failure_*` - forced provider failures, fail-closed proofs (M4).

The Rust provider-neutral contracts under test live in
`crates/nexus-policy` (interfaces) and `crates/nexus-action-gateway`
(deterministic gateway). Test names follow the GraphLock convention;
the M3/M4 gates select them with
`-o python_functions="ep008_integration_*"` /
`-o python_functions="ep008_failure_*"`.

Reality rule: no in-memory substitute for the pinned components; each
integration test runs the real pinned container (OpenFGA, OPA) with
zero persisted credentials and zero-orphan teardown (EP-007 precedent).
