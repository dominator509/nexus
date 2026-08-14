# EP-013 M4 model gateway failure and abuse suite

This directory is the EP-013 M4 milestone manifest root. The Rust
failure/abuse tests that prove EP-013 fails safely under dependency,
policy, security, and resource faults live with the real adapters:

- `config/models/tests/ep013_failure_transport.rs` - failure tests for
  the REAL `nexus-model-transport` HTTP adapter and the REAL
  `nexus-bifrost` gateway composition (gate selector:
  `cargo test --locked -p nexus-model-transport ep013_failure`).

Test names begin `ep013_failure_`. The suite exercises real failure
mechanisms: sandbox token revocation, malformed provider responses,
budget exhaustion, rate limiting, unavailable providers, timeouts,
duplicate requests, and denied routes. The components being proven are
the production adapters; the controlled provider sandbox is a protocol
simulator under TESTING.md's integration layer.

No provider is certified by this suite; live provider certification
remains a later gate.
