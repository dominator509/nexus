# EP-002 M5 Live-Fire Evidence

Generated: 2026-08-12
Node: EP-002 (domain contracts and vocabulary)
Agent: hermes-nexus-main

## Acceptance obligations and proof

1. All IDs are typed and non-interchangeable in Rust
   - crates/nexus-domain/src/id.rs: 12 typed UUIDv7 newtypes
   - crates/nexus-domain/tests/dependency_direction.rs
   - `cargo test --locked -p nexus-domain`: 16 passed
   - Compile-time non-interchangeability proven in validated.rs test
     (TenantId vs CapabilityId comparison rejected by the compiler)

2. JSON Schema validation and generated Rust, TypeScript, Python, and Dart
   models agree
   - tests/unit/test_ep002_unit_agreement.py (7 agreement tests: field names,
     const, enums, UUID formats, additionalProperties, Python class_ alias
     round-trip)
   - crates/nexus-contracts/tests/contracts.rs (round-trips, generated match)
   - packages/contracts/src/__tests__/generated.test.ts (3 tests)
   - `dart analyze packages/contracts/src/generated.dart`: No issues found
   - pnpm --filter @nexus/contracts test:unit: 4 passed (incl. real postgres)
   - Canonical wire names = schema property names verbatim (ADR-006)

3. Vocabulary tables reject unknown risk, privacy, route, principal, and
   capability classes
   - crates/nexus-domain/src/vocabulary.rs
   - validated.rs rejects unknown class strings (ValidationError::Vocabulary)
   - `cargo test --locked -p nexus-contracts ep002_unit`: 8 passed

4. No provider brand leaks into canonical domain names
   - Vocabulary and ID tests assert no vendor names
   - COMPONENT_REGISTRY.yaml keeps provider selection at the registry layer

## Real dependency integration (M3)

- postgres:18.4 ephemeral containers, dynamic host ports
- crates/nexus-contracts/tests/integration_postgres.rs: 4 tests
  - ep002_integration_contracts_roundtrip_real_postgres
  - ep002_integration_idempotency_key_is_unique_in_postgres
  - ep002_integration_slow_query_cancel_and_recovery
  - ep002_integration_ephemeral_container_isolation_and_cleanup
- `cargo test --locked -p nexus-contracts ep002_integration`: 4 passed

## Forced failures (M4)

- crates/nexus-contracts/tests/failure_postgres.rs: 7 tests
  - ep002_failure_unavailable_dependency_fails_closed
  - ep002_failure_timeout_cancels_slow_statement
  - ep002_failure_malformed_input_rejected
  - ep002_failure_duplicate_idempotency_key_rejected
  - ep002_failure_denied_permission_fails_closed
  - ep002_failure_cancelled_work_rolls_back
  - ep002_failure_errors_are_structured_and_redacted
- `cargo test --locked -p nexus-contracts ep002_failure`: 7 passed

## Security and license

- sh scripts/security-check.sh: security check: ok
- sh scripts/license-gate.sh: license gate: ok
- cargo-deny 0.20.2 + cargo-audit 0.22.2 pinned (VERSIONS.lock.yaml,
  SOURCE_VERIFICATION.json, toolchain-check.sh, install.sh,
  dependency-audit.sh, CI) - ADR-006
- cargo audit: No known vulnerabilities found (91 crate dependencies)

## Sentinels observed

- EP-002 M1: ok
- EP-002 M2: ok
- EP-002 M3: ok
- EP-002 M4: ok (security check: ok; license gate: ok)
- EP-002 M5: ok
- node verify EP-002: (see node-verify output)
- scope audit EP-002: ok
