# TESTING

## Principle

Software that appears to work is a failure state. Only software proven through the real implementation and appropriate real dependencies counts. Tests are traceability evidence, not a substitute for live-fire.

## Test layers

1. Domain unit tests: pure invariants, value objects, parsers, policy tables, deterministic routing, schema validators, and state machines.
2. Property tests: idempotency, serialization, policy monotonicity, tenant isolation, action lifecycle, prompt stability, and memory supersession.
3. Contract tests: JSON Schema, OpenAPI, AsyncAPI, MCP, A2A, connector SDK, provider, artifact, skill, and event compatibility.
4. Integration tests: real PostgreSQL, NATS JetStream, Temporal, Keycloak, OpenFGA, OPA, OpenBao, object storage, and sidecars through containers or controlled appliances.
5. E2E tests: browser, Tauri, Flutter, CLI, setup wizard, edge, and control-plane entry points.
6. Live-fire proofs: named outcomes from LIVE_FIRE_PROOFS.md against real services, providers, and hardware required by the active release profile.
7. Provider certification: external account and observable external effect, recorded separately from core tests.
8. Hardware certification: physical model and firmware evidence, not vendor-family inference.
9. Performance, chaos, security, accessibility, privacy, backup, restore, update, and rollback drills.

## Test double zone

Mocks, fakes, fixtures, emulators, generated services, and protocol simulators are legal only under:

- `tests/fixtures/`
- `tests/doubles/`
- `services/*/tests/`
- `crates/*/tests/`
- `apps/*/tests/`
- `provider-certification/fixtures/`

They may test error handling and contract edges. They cannot certify a provider or satisfy a final live-fire proof. Production code cannot branch on test mode, demo mode, fixture mode, or hard-coded sample identities.

## Real dependency rules

- PostgreSQL integration tests use PostgreSQL 18.4, never SQLite or an in-memory substitute.
- NATS tests use JetStream persistence and publish acknowledgements.
- Temporal tests include the official test environment and at least one real server E2E.
- Keycloak, OpenFGA, OPA, OpenBao, Home Assistant, Frigate, Asterisk, Postiz, and other sidecars use pinned images and real APIs.
- Provider certification uses actual test accounts and hardware listed in PREFLIGHT.md.

## Data

Synthetic test people, businesses, messages, calls, camera footage, network flows, and documents are generated from clearly fictional domains. Production data is forbidden. Each test owns a tenant and cleanup token. Artifacts are hash checked and destroyed unless a failed test preserves an encrypted support bundle.

## Flaky policy

A flaky test is a defect. It is isolated, root-caused, and fixed in the same node. Retrying until green, increasing sleeps without evidence, ignoring, or using `continue-on-error` for required tests is prohibited. Deleting a test requires an ADR proving the behavior is no longer required.

## Validation matrix

| Specification | Primary test roots | Live-fire or certification |
| --- | --- | --- |
| SPEC-000 | `tests/product/` | all LF proofs |
| SPEC-001 | `crates/domain/tests/` | LF-004, LF-005 |
| SPEC-002 | `crates/memory/tests/`, `tests/data/` | LF-002, LF-020 |
| SPEC-003 | `tests/contracts/`, `tests/mcp/`, `tests/a2a/` | LF-016, LF-023 |
| SPEC-004 | `apps/web/tests/`, `apps/setup/tests/` | LF-001, LF-005 |
| SPEC-005 | `tests/security/identity/`, `tests/security/policy/` | LF-003, LF-022, LF-028 |
| SPEC-006 | `tests/reliability/` | LF-017, LF-019, LF-021 |
| SPEC-007 | `tests/observability/` | LF-019 |
| SPEC-008 | `scripts/production-readiness-check.sh` | all active proofs |
| SPEC-009 | `crates/model-router/tests/`, `tests/model/` | LF-021 |
| SPEC-010 | `tests/agents/`, `tests/skills/` | LF-016, LF-018 |
| SPEC-011 | `tests/home/` | LF-006, LF-007, LF-024 |
| SPEC-012 | `tests/voice/`, `hardware/tests/voice/` | LF-026, LF-028 |
| SPEC-013 | `tests/sentinel/` | LF-009, LF-010 |
| SPEC-014 | `tests/communications/` | LF-011, LF-012, LF-013 |
| SPEC-015 | `tests/business/`, `tests/social/` | LF-014, LF-015, LF-025, LF-027 |
| SPEC-016 | `tests/deployment/` | LF-001 |
| SPEC-017 | `apps/mobile/integration_test/` | LF-004, LF-022 |
| SPEC-018 | `tests/self-healing/` | LF-019 |
| SPEC-019 | `tests/supply-chain/` | ship gate |
| SPEC-020 | `tests/privacy/` | LF-002, LF-028 |
| SPEC-021 | `tests/cameras/`, `hardware/tests/cameras/` | LF-008 |
| SPEC-022 | `tests/connectors/` | LF-023 |
| SPEC-023 | `tests/events/`, `services/workflows/tests/` | LF-007, LF-017 |
| SPEC-024 | `tests/storage/`, `tests/recovery/` | LF-002, LF-020 |
| SPEC-025 | `services/microbrain/tests/` | shadow and canary gates |

## Definition of test-done

The requirement has unit or property coverage where applicable, real integration coverage, E2E entry-point coverage, failure coverage, observability assertions, cleanup, and a live-fire or certification path. All required commands pass once in a fresh run and their sentinels are recorded.
