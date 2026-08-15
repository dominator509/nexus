# EP-015 M4 reflex router failure and abuse suite

This directory is the EP-015 M4 milestone manifest root. The failure
tests that prove the REAL router and reflex boundary fail safely live
with the router crate:

- `crates/nexus-model-router/tests/ep015_failure_router.rs` - failure and
  abuse tests against the REAL production router and the REAL EP-014
  reflex transport over a controlled provider sandbox (gate selector:
  `cargo test --locked -p nexus-model-router ep015_failure`).

Proven real failure mechanisms (the adapter under proof is never mocked;
the controlled sandbox scripts the failure):

- provider unreachable (closed port) -> UNAVAILABLE fail-closed
- provider read timeout (silent peer) -> TIMEOUT/UNAVAILABLE fail-closed
- malformed provider payload (missing usage) -> VALIDATION fail-closed
- learned adapter failure -> typed error, no fabricated route
- learned out-of-distribution -> escalation, policy route retained
- budget cap exceeded -> BUDGET escalation, never routed over cap
- authority boundary: routing decision carries no authorization fields
- audit redaction: audit records carry metadata only, never
  features/prompts/secrets
- no poisoned state: router remains deterministic after provider failure

Operations diagnostic / bounded recovery command (M4 CONTENT item 6):
`sh benchmarks/router/frozen-benchmark.sh` is the bounded recovery
diagnostic for the router plane - it re-proves the frozen corpus,
security overrides, and replay stability after any policy or provider
change, and exits nonzero on any violation.
