# EP-011 M4 — Failure Hardening, Abuse Cases, and Observability

Node: EP-011 (connector SDKs and sidecar runtime)
Milestone: M4 — forced failures, abuse cases, and observability
Date: 2026-08-14
Owner: hermes-nexus-main
Evidence location: `.agent/state/evidence/ep011-m4/`

## Scope

The real hardened sidecar boundary (`crates/nexus-sidecar/`) proven over real
loopback HTTP: test client → real `nexus-sidecar` process → real Python
fixture provider (`tests/connectors/fixture_sidecar.py`). No in-process mocks
on the proven path; every failure is induced through the real mechanism
(process death, malformed/oversized/truncated/schema-invalid payloads,
signature failure, replay, path traversal, credential scope denial, signal).

## Suite Results (real runs, this milestone)

| Suite | Binary | Result |
| --- | --- | --- |
| Unit (lib) | `cargo test -p nexus-sidecar` (lib) | 45 passed |
| Dependency direction | `tests/dependency_direction.rs` | 1 passed |
| Failure/abuse | `tests/ep011_failure_abuse.rs` | 36 passed |
| Integration/lifecycle | `tests/ep011_integration_lifecycle.rs` | 21 passed |
| M3 Python transport parity | `pytest tests/connectors -o python_functions=ep011_*` | 58 passed |
| M3 Rust SDK | `cargo test -p nexus-connector-sdk` | unit+integration green (gate) |
| M3 TypeScript SDK | `pnpm --filter @nexus/connector-sdk test:unit` | green (gate) |
| Clippy | `cargo clippy -p nexus-sidecar --all-targets --all-features` | 0 warnings |
| Format | `scripts/format-check.sh` | `format check: ok` |
| Lint | `scripts/lint.sh` | `lint: ok` |
| Security | `scripts/security-check.sh` | `security check: ok` |
| License | `scripts/license-gate.sh` | `license gate: ok` |
| Reality | `scripts/reality-gate.sh` | `reality gate: ok` |
| Expected files | `scripts/expected-files.sh EP-011` | `expected files EP-011: ok` |
| Scope audit | `scripts/scope-audit.sh EP-011` | `scope audit EP-011: ok` |
| Orphan audit | `scripts/ep011-orphan-audit.sh` | `EP-011 orphan audit: ok` |
| M4 gate | `scripts/nodes/EP-011.sh M4` | `EP-011 M4: ok` |
| M3 re-verify | `scripts/nodes/EP-011.sh M3` | `EP-011 M3: ok` (vacuity 58, orphan audit ok) |

## Fail-Closed Boundary Proofs (directive A–AC)

### Malformed / malicious clients (all rejected before provider invocation)
- malformed JSON → 400 VALIDATION; truncated JSON → 400 VALIDATION
- malformed UTF-8 → 400 VALIDATION; binary body → 400 VALIDATION
- deeply nested JSON → 400 VALIDATION (bounded depth)
- oversized request (> 64 KiB) → 413 PAYLOAD_TOO_LARGE before provider
- duplicate `tenant_id` / `protocol_version` (and other security keys) → 400
  VALIDATION (ambiguous JSON fails closed; no parser-dependent retention)
- unknown top-level field → 400 VALIDATION
- wrong Content-Type on POST → 415; wrong method → 405; unknown path → 404;
  debug/admin/metrics/status paths → 404

### Protocol version (no silent downgrade)
- missing protocol header → fail closed; old major → fail closed; future
  major → fail closed; conflicting declarations → fail closed

### Tenant / connector binding (before provider)
- bound tenant A + body tenant B → 400 VALIDATION (`tenant mismatch`)
- capability owned by another connector → 503 UNAVAILABLE
  (`capability not found`, no cross-connector fallback)
- unknown connector → 503 UNAVAILABLE (`unknown connector`)
- class mismatch (query on COMMAND class) → 400 VALIDATION
  (`class mismatch`, provider not invoked)

### Provider failure (real process/transport)
- provider absent before request → 503 UNAVAILABLE
- provider dies before dispatch → 502 EXTERNAL_PROVIDER
- provider malformed response → 502 EXTERNAL_PROVIDER (never relayed)
- provider schema-invalid response → 502 EXTERNAL_PROVIDER (never relayed)
- provider oversized response → 502 VALIDATION (`bounded size`)
- provider slow → 504 TIMEOUT (typed, phase-specific)
- command partial side effect (provider mutates then exits before ack) →
  502 EXTERNAL_PROVIDER; NO fabricated SUCCESS; no success entry is recorded
  (the sidecar holds no success cache; the fixture dedupe is key-based and
  the crashed provider never acked)

### Credential boundary
- permitted reference → 200 with fingerprint only (canary value absent)
- out-of-scope reference → 403 AUTHORIZATION
- unnamespaced reference → 403 AUTHORIZATION
- other-connector reference → 403 AUTHORIZATION
- credential canary (`fixture-secret-value`) searched in: HTTP success body,
  HTTP error body, sidecar stdout (PORT contract), sidecar stderr (redacted
  telemetry) — zero occurrences

### Webhook ingress
- valid signature + fingerprint → 200 normalized
- missing signature → 401 VERIFICATION; invalid signature → 401
  VERIFICATION; wrong fingerprint → 401 VERIFICATION
- replay of same provider_event_id across separate HTTP requests → second
  request 401 VERIFICATION (shared in-process dedupe state; replay defense
  is process-lifetime only — crash-durable replay defense NOT ASSERTED)
- validly signed unknown event (even one named after a real capability) →
  normalized (`executable: false`), never executed; provider shows zero
  command executions for that name

### Legacy poller boundary
- unchanged source → zero fabricated events
- real mutation → expected event only
- corrupt checkpoint → detected, fail closed
- truncated source → fail closed
- `../` traversal / absolute escape → rejected at provisioning (exit 2)
- restart resumes at the owned checkpoint (validated)
- unprovisioned poll → fail closed

### Concurrency / resource pressure
- concurrency bound enforced (saturation returns typed 429/504/502, never
  fabricated success; at least one concurrent request succeeds)

### Shutdown ownership (directive C/D)
- idle SIGTERM → exit 0, SIDECAR_STOPPED emitted
- mid-request SIGTERM (slow provider in-flight, observed via
  REQUEST_ACCEPTED) → bounded exit (< 10 s), in-flight client receives
  termination semantics (never success), listener released (old port
  rebindable immediately), zero orphan processes
- owned resources terminate with the process: listener, in-flight request
  tasks, webhook ingress state, poller state/checkpoint writer, credential
  scope table, telemetry sink, signal/background tasks
- not owned and not terminated: provider process (sidecar is a client;
  test owns provider lifecycle)

### Observability
- lifecycle event stream observed: SIDECAR_STARTED → SIDECAR_READY →
  REQUEST_ACCEPTED → DISPATCH_COMPLETED → SIDECAR_STOPPED
- redaction: raw tenant id never appears in telemetry; credential values
  never appear in stdout/stderr/HTTP bodies

### Authorization boundary
- sidecar validation = acceptance only; responses never claim
  "authorized"; EP-008 remains the authorization authority (NOT ASSERTED
  here — no fabricated EP-008 composition)

## Parallel-Hang Root Cause (directive B)

Determined empirically, not serialized away:

- CASE 1 (harness resource collision): NOT the cause. Harness uses only
  ephemeral ports, per-test tempdirs, per-test processes, no shared
  mutable env/files. The 36-test failure suite passed in parallel in
  3.54–4.13 s and the 21-test lifecycle suite in 2.52–3.23 s with default
  thread parallelism.
- CASE 2 (real sidecar lifecycle race): NOT the cause. The sidecar
  correctly stays alive awaiting requests; no shutdown/ownership defect.
- Actual cause: a leftover dev diagnostic `tests/probe_test.rs`
  (`probe_spawn`) that unconditionally panicked and blocked forever on
  `read_to_string` waiting for stdout EOF from the long-lived sidecar.
  Removed (redundant with `ep011_failure_sidecar_provider_absent_before_request`).
  After removal the full `cargo test -p nexus-sidecar` runs clean under
  default parallelism: 45 + 1 + 36 + 21, zero leaks.

## Gate Bug Found and Fixed

The M4 gate's `pytest tests/connectors -q` step collected ZERO tests: the
M3 Python suite functions are `ep011_integration_*`/`ep011_failure_*`, and
`pyproject.toml`'s `python_functions` list does not include `ep011_*`
(pyproject.toml is outside the EP-011 fence). The M3 gate never ran pytest
(the 58 tests were run manually with the selector). Fixed in
`scripts/nodes/EP-011.sh` (in-fence): both M4 and M5|verify now invoke
`pytest tests/connectors -q -o python_functions=ep011_*` → 58 passed.

## Artifacts

- `crates/nexus-sidecar/` (lib, binary, tests) — the hardened boundary
- `tests/connectors/fixture_sidecar.py` — arming controls (malformed,
  schema-invalid, oversized, slow, crash-after-mutate)
- `python/nexus_connector_sdk/sidecar.py` — no change committed; the M3
  `except json.JSONDecodeError, OSError:` form is valid Python 3.14 (PEP
  758) and ruff-format canonical; the earlier import failure was stale
  pycache (cleared)
- `scripts/nodes/EP-011.sh` — M4 gate wired to `nexus-sidecar` full suite +
  pytest selector + orphan audit
- `scripts/ep011-orphan-audit.sh` — extended with the exact owned
  `nexus-sidecar` process pattern

## Result Codes Observed

VALIDATION, PAYLOAD_TOO_LARGE, NOT_FOUND, METHOD_NOT_ALLOWED,
UNSUPPORTED_MEDIA_TYPE, UNAVAILABLE, TIMEOUT, EXTERNAL_PROVIDER,
AUTHORIZATION, VERIFICATION, CONFLICT, PROTOCOL_VERSION_MISMATCH,
RATE_LIMIT — all canonical cross-language SDK codes (SPEC-006).

No credentials, bearer tokens, private data, or complete sensitive IDs are
persisted in this evidence; telemetry carries fingerprints/correlation IDs
only.
