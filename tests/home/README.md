# EP-020 M4 -- Home Assistant forced-failure suite

Real failure mechanisms against the REAL pinned Home Assistant container
(`ghcr.io/home-assistant/home-assistant:stable@sha256:56690a...cb42a5`,
Apache-2.0, running version 2026.8.2). No mocks of the component under
proof (TESTING.md). The M3 real-container fixture is reused; a throwaway
`nexus-abuse` user is provisioned ONLY for the rate-limit proof so the
admin credential is never affected by lockout.

## What is proven here

- `ep020_failure_bad_credential_denied` -- denied permission: bad token
  -> 401, typed, never success.
- `ep020_failure_unknown_entity_typed` -- NotFound: unknown entity ->
  404, no fabricated state.
- `ep020_failure_invalid_service_rejected` -- invalid service/action ->
  400, never accepted as success.
- `ep020_failure_malformed_body_rejected` -- malformed input -> 400 AND
  no partial side effect (fixture state unchanged).
- `ep020_failure_duplicate_request_idempotent` -- duplicate submission
  is idempotent (both accepted, one effect).
- `ep020_failure_verification_window_expiry_not_success` -- bounded
  window expiry is TIMEOUT/UNKNOWN, never VERIFIED.
- `ep020_failure_ha_offline_fail_closed` -- unavailable dependency:
  stopped container -> connection failure, never success.
- `ep020_failure_abuse_rate_limit_fail_closed` -- repeated failed
  authentication never mints a token; HA's real throttle signal is
  observed and recorded (denial is unconditional, throttle claim is
  evidence-based only).
- `ep020_failure_errors_never_leak_secrets` -- observability: error
  surfaces never echo credentials.

Contract-crate fail-closed tests (Rust, `ep020_failure_*` in
`crates/nexus-home/tests/ep020_failure_forced.rs`): verifier missing
target -> UNKNOWN, unrelated change -> UNRELATED_CHANGE (never VERIFIED),
mismatch -> MISMATCH, missing attribute -> UNKNOWN, verification-timeout
terminal distinct from VERIFIED, unknown-domain mapping total (-> OTHER,
no panic), display-name identity rejected, unknown vocabulary rejected
at parse (fail-closed, no cross-class coercion), unknown availability
never treated as safe, error redaction never leaks payload, correlation
preserved, authorization/policy/unavailable/conflict typed, correlation
ids are deterministic UUIDv7.

## Operations diagnostic (every new service/provider)

| Signal                                                  | Diagnostic                                                                                                     | Bounded recovery                                                                                      |
| ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| HA container absent                                     | `docker ps -a --filter name=nexus-ep020-ha`                                                                    | `docker rm -f nexus-ep020-ha; re-run the suite` (fixture recreates)                                   |
| HA image missing                                        | `docker image inspect ghcr.io/home-assistant/home-assistant:stable@sha256:56690a...cb42a5`                     | Pull the pinned digest; the fixture fails fast with a clear error if absent                           |
| HTTP ready but entities missing                         | `GET /api/states` (needs token) or the container LogPath (`docker inspect <name> --format 'LogPath-template'`) | The fixture's pre-auth boot wait + post-restart entity wait fail the suite closed (never false-ready) |
| 401 on every request                                    | Token stale (30 min expiry)                                                                                    | Re-run the suite; the fixture mints a fresh OAuth token per run (never reuse debug tokens)            |
| Container boots with defaults (fixture entities absent) | `docker inspect <name> --format 'Mounts-template'` (prints each Source -> Destination)                         | Fix the config mount; the mount assertion fails the suite closed (infra/infra regression test)        |
| Rate-limit lockout of admin user                        | Only `nexus-abuse` is hammered; admin token unaffected                                                         | If admin is ever locked, remove `.storage/auth` and re-run (fresh auth store)                         |
| Offline/stopped container                               | `docker ps --filter name=nexus-ep020-ha`                                                                       | `docker start nexus-ep020-ha`; wait ready + entities                                                  |

Recovery command (bounded): re-running the M4 gate
`sh scripts/nodes/EP-020.sh M4` recreates the ephemeral container,
mints a fresh token, and re-proves every failure path. The suite owns
its container lifecycle; teardown removes the container and all
generated config state, so a re-run starts from the pinned image only.

## Certification boundary (unchanged from M3)

- HA real server / authentication / API provider / command+readback:
  PASS (M3, real container).
- controlled template-light entity: CONTROLLED_TEST_FIXTURE.
- physical light hardware: NOT ASSERTED / DEFERRED to its certification
  owner (no NODE_BLOCK).
- metrics/traces dashboards: owned by the control-plane observability
  nodes (EP-044 class); this suite proves structured errors + incident
  correlation without secrets.
