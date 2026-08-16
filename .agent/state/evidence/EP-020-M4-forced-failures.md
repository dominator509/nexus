# EP-020 M4 Evidence — Forced failures, abuse cases, and observability

Node: EP-020 (Home Assistant provider and device control)
Milestone: M4
Date: 2026-08-16
Runner: autonomous GraphLock execution (ledger lease `5dc7425`)

## Contract-crate fail-closed suite (Rust, 13 tests)

`crates/nexus-home/tests/ep020_failure_forced.rs`, run through the REAL
`cargo test --locked -p nexus-home ep020_failure` machinery (vacuity
guarded). All tests exercise the real production types in `nexus-home`:

- verifier missing target state -> UNKNOWN (never Verified)
- unrelated entity change -> UNRELATED_CHANGE (never Verified)
- target mismatch -> MISMATCH (never Verified)
- missing attribute under AttributeEquals -> UNKNOWN
- verification timeout is a terminal distinct from VERIFIED; provider
  ack is SUBMITTED at most
- unknown/malicious HA domain maps total to OTHER (no panic, no leak)
- display names / empty / spaced strings never strong identity
- unknown vocabulary rejected at parse (fail-closed; "LIGHT" is a valid
  DeviceCategory but NOT a CommandState - no cross-class coercion)
- unknown availability never treated as safe/off/closed
- error redaction never leaks payload/secret (Display = code +
  correlation + resource only)
- correlation preserved through typed errors; auth/policy/unavailable/
  conflict codes distinct
- correlation ids are deterministic UUIDv7 (SPEC-006)

## Real-container failure suite (pytest, 9 tests)

`tests/home/test_ep020_failure_home_assistant.py` against the REAL
pinned HA container (digest `sha256:56690a…cb42a5`, version 2026.8.2),
reusing the M3 automated OAuth bootstrap. A throwaway `nexus-abuse`
user is provisioned ONLY for the abuse proof; the admin token is never
affected by lockout.

- bad credential -> 401 denied (typed, never success)
- unknown entity -> 404, no fabricated state
- invalid service/action -> 400, never accepted
- malformed body (wrong-typed entity_id) -> 500, still fail-closed
  (status >= 400) and NO partial side effect (fixture state unchanged)
  - real wire fact: this HA version returns 500 for a malformed service
    body, not 400; the invariant asserted is fail-closed + no effect
- duplicate service submission is idempotent (both accepted, one
  effect, no conflict on replay)
- bounded verification window expiry is TIMEOUT/UNKNOWN, never VERIFIED
- container stopped -> connection failure, never success; restart +
  entity wait restores the suite
- abuse: 6 consecutive failed login flows NEVER mint a token; every
  step is an error (fail-closed under pressure); HA's real throttle
  signal is observed and recorded (denial unconditional, throttle
  claim evidence-based only)
- error surfaces never echo credentials

## Gates

- `EP-020 M4: ok` (node gate, artifact check + vacuity-guarded cargo +
  vacuity-guarded pytest)
- `security check: ok`
- `license gate: ok`
- scope audit, format, lint, dependency, reality: run at node closure

## Operations diagnostic

`tests/home/README.md` — per-signal diagnostics and bounded recovery
commands for the HA provider (container absent/image missing/entities
missing/401/mount-wrong/rate-lockout/offline). Recovery command is the
M4 gate re-run: the fixture recreates the ephemeral container, mints a
fresh token, and re-proves every failure path.

## Certification boundary (unchanged)

HA real server / authentication / API provider / command+readback:
PASS (M3 real container). Controlled template-light entity:
CONTROLLED_TEST_FIXTURE. Physical light hardware: NOT ASSERTED /
DEFERRED to its certification owner. Metrics/traces dashboards owned by
the control-plane observability nodes; this suite proves structured
errors + incident correlation without secrets.
