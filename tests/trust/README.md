# EP-009 trust tests

This directory hosts the EP-009 test suites for the trust domain
(`crates/nexus-trust`) and its infrastructure adapters
(`infra/openbao`, `infra/pki`, `infra/headscale`, `config/sops`).

## Naming

- `ep009_unit_*` - pure contract and vocabulary tests (no I/O). Run
  via `cargo test --locked -p nexus-trust ep009_unit` and the M1/M2
  gate.
- `ep009_integration_*` - real provider tests against the pinned
  OpenBao container (M3 gate).
- `ep009_failure_*` - fail-closed provider failure tests (M4 gate).

## Rules

- Secrets never appear in test output, logs, or evidence. Values are
  opaque wrappers that redact `Debug`/serialization.
- Provider versions are pinned in `VERSIONS.lock.yaml` and registered
  in `COMPONENT_REGISTRY.yaml`; tests read image references from the
  lock file, never from unversioned tags.
- Test containers are created and removed by the test itself; orphaned
  containers are swept by `scripts/ep009-orphan-audit.sh` (M5 teardown).
