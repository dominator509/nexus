# EP-044 M5 Live-Fire Evidence - Control Plane Runtime

Date: 2026-08-14
Node: EP-044 (Control Plane Runtime)
Agent: hermes-nexus-main

## Graph amendment (owner resolution of EP-013 GLOBAL_GATE_PREREQUISITE_UNOWNED)

- GRAPH.md now 45 nodes; EP-044 inserted between EP-013 and EP-014
  (EP-014 DEPS rewired EP-013 -> EP-044).
- `scripts/smoke-test.sh` activates the runtime smoke at `at-least EP-044`.
- Before EP-044 is DONE: `runtime smoke: not-applicable-before EP-044`
  (never a false PASS for runtime functionality).
- At/after EP-044 is DONE: the smoke is mandatory and fails closed when
  the runtime is absent or unhealthy.
- Gate-ownership regression: `tests/runtime/smoke-gate-regression.sh`
  proves (1) not-invoked before owner, (2) wiring at EP-044,
  (3) fail-closed when absent. Observed: `smoke gate regression: ok`.
- EP-013 closed green after the amendment: `node verify EP-013: ok` with
  the smoke stage classified as not-applicable-before EP-044; NODE_DONE;
  tag green/EP-013 at 71607cc.

## Runtime implementation (apps/control-plane, nexus-control-plane crate)

- Real runnable binary `nexus-control-plane` (axum 0.8.9, tokio 1.53.1,
  hyper 1.11.0, tower 0.5.3).
- Canonical endpoints with real handlers: GET /healthz, GET /readyz,
  GET /v1/capabilities.
- Composition root: real CapabilityListSource + real RuntimeLifecycle;
  no fabricated data; fail closed.
- Graceful startup/shutdown: bind once, serve, stop on signal, no leak.
- Canonical runtime config: `config/runtime/core.json`.
- Local deterministic bring-up: `infra/compose/core.yaml` +
  `apps/control-plane/Dockerfile` (image nexus-control-plane:local,
  built and tagged 2026-08-14, digest image id 975918c35e4c).
- Canonical base URL/domain resolution: NEXUS_BASE_DOMAIN / NEXUS_SMOKE_URL
  conventions; local profile maps the control-plane to 127.0.0.1:8443
  (host :443 is an unrelated API, so https://nexus.test is not the local
  control-plane).

## Allow path (real live execution)

Commands (all run now, sentinels observed):

- `sh scripts/nodes/EP-044.sh M1` -> `EP-044 M1: ok` (19 unit + 1 dep-direction)
- `sh scripts/nodes/EP-044.sh M2` -> `EP-044 M2: ok` (25 unit)
- `sh scripts/nodes/EP-044.sh M3` -> `EP-044 M3: ok` (4 integration, real binary)
- `sh scripts/nodes/EP-044.sh M4` -> `EP-044 M4: ok` (5 failure, real binary)
- Full crate suite: 34 tests (19 lib + 1 dep + 5 core + 5 failure + 4 integration)
- `docker compose -f infra/compose/core.yaml config --quiet` -> exit 0
- `docker build -f apps/control-plane/Dockerfile -t nexus-control-plane:local .` -> success (image 975918c35e4c, 130MB)
- `NEXUS_SMOKE_URL=http://127.0.0.1:8443 sh scripts/local-start.sh core` -> `local start core: ok`
- `NEXUS_SMOKE_URL=http://127.0.0.1:8443 sh scripts/smoke/runtime.sh` -> `runtime smoke: ok`
- `sh scripts/smoke-test.sh` (EP-044 not DONE at that point) -> `runtime smoke: not-applicable-before EP-044` + `smoke test: ok`

Real endpoint assertions over real HTTP (from ep044_integration_http.rs and
the live container):

- GET /healthz -> HTTP 200, body `{"status":"healthy"}`
- GET /readyz -> HTTP 200, body `{"ready":true}`
- GET /v1/capabilities -> HTTP 200, body `{"capabilities":["health","capabilities"]}`
- GET /v1/nope -> HTTP 404

## Denial/failure paths (real binary, fail closed)

- Port conflict -> process exits nonzero
- Invalid config (empty base domain, malformed bind) -> process exits
  nonzero, stderr redacted
- Runtime absent -> canonical smoke probes fail (never ALLOW)
- Graceful shutdown -> SIGTERM terminates, port closes, no leaked process
- Telemetry -> startup output never contains raw tenant id

## Side gates

`security check: ok`, `license gate: ok`, `reality gate: ok`,
`format check: ok`, `lint: ok`, `dependency audit: ok`,
`blueprint validation: ok`, `scope audit EP-013: ok` (post-amendment),
adapter parity 8x3505091078 1453.

## Persisted artifacts

- Deterministic M5 evidence (this file) under `.agent/state/evidence/ep044-m5/`.
- No credentials, bearer tokens, or private data persisted; evidence carries
  endpoint shapes, exit codes, and image identifiers only.
