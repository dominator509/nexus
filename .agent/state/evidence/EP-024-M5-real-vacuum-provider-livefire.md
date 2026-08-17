# EP-024 M5 Live-Fire: Real Vacuum Provider Through the EP-020-Certified Home Assistant Boundary

- Node: EP-024 (SPEC-011 home devices; milestone M5 live-fire and node closure)
- Date: 2026-08-17
- Gate: `sh scripts/ep024-m5-tests.sh` -> `EP-024 M5: ok` (exit 0)
- Container: `nexus-ep024-vac` (127.0.0.1:8126) running the SAME pinned HA digest certified by EP-020 (`ghcr.io/home-assistant/home-assistant:stable@sha256:56690a89...cb42a5`)

## Proven chain (strongest real owned chain)

REAL pinned Home Assistant -> real template vacuum entities
(`vacuum.nexus_vacuum_a` / `vacuum.nexus_vacuum_b` backed by real
`input_select` helpers + real auto-dock automation) -> production
`nexus-vacuum` adapter (composed through EP-020
`RestTransport::with_timeout(10s)`, no second HA client) -> real
capability discovery -> real `vacuum.start` -> exact-target CLEANING
verification -> real `vacuum.pause` -> exact-target PAUSED verification
-> real `vacuum.return_to_base` -> RETURNING (distinct) -> real
auto-dock transition -> DOCKED verification -> restart -> stable
identity -> offline/revoke failure behavior -> clean teardown.

## Observed sentinels

- `EP-024 M5: ok`
- 11 lib unit + 10 integration unit + 10 probe + 1 live journey tests green
- `observed vacuum A supported_features=12308` = START(4096) | STATE(8192) | PAUSE(4) | RETURN_HOME(16) - capability mapping bound to the observed provider value
- `vacuum-diag: ok` with `availability=AVAILABLE capabilities=START_CLEAN,PAUSE,RETURN_HOME,DOCK state=DOCKED`
- `auth: FAIL (UNAVAILABLE)` while the provider is stopped (diag correctly reports unavailable)
- restore poll requires genuine fresh-readback AVAILABLE before healthy
- zero orphans after teardown; vacuity guards pass (non-zero tests, non-zero passes, journey + probe names present)

## Real state transitions (fresh exact-target readback, never /api/states mutation)

- StartClean A -> SUBMITTED -> readback `cleaning` -> VERIFIED CLEANING; B untouched
- Pause A -> SUBMITTED -> readback `paused` -> VERIFIED PAUSED
- Resume A -> cleaning; ReturnHome A -> SUBMITTED -> readback `returning` (DISTINCT from docked) -> VERIFIED RETURNING -> auto-dock automation -> readback `docked` -> VERIFIED DOCKED
- Dock B -> SAME provider action (`vacuum.return_to_base`) -> RETURNING -> DOCKED (explicit mapping, not two fabricated behaviors)
- Wrong-target: B's CLEANING transition never verifies A (adapter Verification + verifier UnrelatedChange)
- Retry after completion is NOT a Conflict (in-flight released; crash-durable NOT ASSERTED - process-local)
- Docker restart -> rediscovery -> same canonical identity -> StartClean works -> exact readback works
- Provider stop -> UNAVAILABLE (never DOCKED/SAFE); recover() clears 0; provider start -> AVAILABLE only via fresh readback
- Correlation `vacuum-<nanos>-<seq>` preserved on every error path; zero token leakage in audit/counters/diag

## Forced-failure matrix (real mechanisms)

1. HA provider unavailable -> UNAVAILABLE (docker stop)
2. bad credential -> auth_check false (real 401 -> External per EP-020 contract)
3. unknown vacuum -> NotFound (selector + registry membership)
4. unsupported command (MapReadback without map surface) -> Policy BEFORE provider mutation
5. provider timeout -> TIMEOUT (silent TCP peer, bounded 10s client)
6. malformed provider response -> fail closed (External, never benign)
7. wrong-target state transition -> never verifies
8. ambiguous post-command disconnect -> no blind retry (SUBMITTED/UNKNOWN preserved; verify-first)
9. map requested without provider map support -> Policy (fail closed, NOT success)
10. revoked/unavailable target -> no command execution (UNAVAILABLE)
11. redaction canary -> zero credential leakage
12. restart/reconnect -> stable canonical identity

## Certification

- nexus-vacuum adapter: INTERNAL_CERTIFIED
- Home Assistant vacuum provider path: PROVIDER_CERTIFIED (EP-020 composition + real vacuum proof)
- controlled template vacuum fixtures: CONTROLLED_TEST_FIXTURE
- vacuum map path: NOT CERTIFIED (no real map exercised; MapReadback implementation IMPLEMENTED, fails closed Policy; no fabricated map data)
- physical robot vacuum / physical SLAM map: NOT ASSERTED / DEFERRED
- RobotProvider hardware: NOT ASSERTED (M1 RobotSafetyDeclaration regression green - vacuum support never widens robot authority)
