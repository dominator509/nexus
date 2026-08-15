NODE-META-BEGIN
ID: EP-044
DEPS: EP-013
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-044
VERIFY_SENTINEL: node verify EP-044: ok
GREEN_TAG: green/EP-044
NODE-META-END

# 1. Purpose / Big Picture

Implement the Nexus Control Plane Runtime. This node owns the real runnable control-plane server binary under `apps/control-plane/`, the application composition root, `infra/compose/core.yaml`, canonical runtime configuration, canonical base URL/domain resolution, the three canonical runtime endpoints (`/healthz`, `/readyz`, `/v1/capabilities`), graceful startup/shutdown, runtime smoke ownership, local deterministic runtime bring-up, and runtime observability bootstrap. It is the owner that resolves the `GLOBAL_GATE_PREREQUISITE_UNOWNED` graph defect recorded at EP-013.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-044.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-044.txt`.
- Implement real behavior, tests, telemetry, security, operations, and any owning live-fire proof.
- Own the runtime smoke: `scripts/smoke/runtime.sh` remains the single canonical runtime smoke and must pass against the real server.
- Own local deterministic bring-up: `sh scripts/local-start.sh core` works against `infra/compose/core.yaml`.
- Keep optional providers disabled until certified.

# 3. Non-goals

- No work owned by a later node (web dashboard, setup wizard, compute fabric, deployment, ship).
- No production deployment.
- No mocks, stubs, demonstration modes, or sample success in production paths.
- No fake runtime, placeholder health endpoint, or /etc/hosts trick.
- No weakening of the runtime smoke assertions.
- No weakening of a spec, policy, security boundary, test, or GraphLock gate.

# 4. Context and Orientation

Nexus is logically one brain and physically a distributed control system. This node creates the runnable control plane that composes the real contracts built by EP-000..EP-013: domain, identity, data, events, workflows, auth, policy/action gateway, trust, capabilities, connectors, API fabric, and model gateway. It is inserted into the graph between EP-013 and EP-014 by the owner GraphLock amendment of 2026-08-14.

The runtime smoke gate was previously activated at `at-least EP-012`, which outran its dependency owner: no node owned or created the Nexus runtime. This node owns the runtime; `scripts/smoke-test.sh` now activates the runtime smoke only at `at-least EP-044`. Before EP-044 is DONE the smoke stage reports `runtime smoke: not-applicable-before EP-044`; at/after EP-044 it MUST run and MUST fail if the runtime is absent or unhealthy.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-044.md`
- `.agent/specs/.agent/specs/SPEC-003-api-mcp-a2a-artifacts-and-interoperability.md`
- `.agent/specs/.agent/specs/SPEC-006-errors-reliability-idempotency-verification-and-action-safety.md`
- `.agent/specs/.agent/specs/SPEC-007-observability-incident-correlation-and-operations.md`
- `crates/nexus-capabilities/src/{descriptor,registry,health,vocabulary}.rs`
- `crates/nexus-fabric/src/rest.rs`
- `scripts/smoke/runtime.sh`
- `scripts/smoke-test.sh`
- `scripts/local-start.sh`
- `infra/gateway/` (composition precedent)

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-044.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-044-control-plane-runtime.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-044.txt`
- `.agent/node-contracts/EP-044.md`
- `.agent/milestone-files/EP-044-M1.txt` .. M5
- `scripts/nodes/EP-044.sh`
- `apps/control-plane/`
- `infra/compose/`
- `config/runtime/`
- `tests/runtime/`
- `scripts/smoke-test.sh` (activation amendment)
- `scripts/smoke/runtime.sh` (owned)
- `scripts/local-start.sh` / `scripts/local-stop.sh` (owned)
- `live-fire/REGISTRY.tsv` + `scripts/live-fire/LF-029.sh`
- `.agent/GRAPH.md` (graph amendment)
- `.agent/MANIFEST.md`
- `AGENTS.md`, `README_FIRST.md`, `NEXUS_GRAPHLOCK_INPUTS.md` (node count text)
- `Cargo.toml`, `Cargo.lock`
- `docs/vocabulary/README.md`
- `references/ADR-019-control-plane-runtime-vocabulary.md`
- `references/ADR-020-runtime-smoke-ownership.md`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `ControlPlaneConfig` | `nexus-control-plane` | Defined by EP-044; provider-neutral and versioned |
| `RuntimeHealth` | `nexus-control-plane` | Defined by EP-044; canonical `/healthz` shape |
| `RuntimeReadiness` | `nexus-control-plane` | Defined by EP-044; canonical `/readyz` shape |
| `CapabilityList` | `nexus-control-plane` | Defined by EP-044; canonical `/v1/capabilities` shape |
| `ControlPlaneServer` | `nexus-control-plane` | Defined by EP-044; provider-neutral and versioned |
| `RuntimeLifecycle` | `nexus-control-plane` | Defined by EP-044; graceful start/shutdown |
| `RuntimeSmoke` | `nexus-control-plane` | Defined by EP-044; smoke contract |

Acceptance obligations:

1. A real runnable control-plane server binary exists under `apps/control-plane/` (no placeholder, mock, or demo mode)
2. `GET /healthz` returns `{"status":"healthy"}` HTTP 200 when healthy
3. `GET /readyz` returns `{"ready":true}` HTTP 200 when ready
4. `GET /v1/capabilities` returns `{"capabilities":[...]}` HTTP 200 non-empty
5. `infra/compose/core.yaml` brings up the control plane deterministically for the local profile
6. Canonical base URL/domain resolution through `NEXUS_BASE_DOMAIN` / `NEXUS_SMOKE_URL`
7. Graceful startup/shutdown without leaked processes
8. Runtime smoke ownership: activation only at `at-least EP-044`

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones

### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for the control plane runtime.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-044-M1.txt`, `.agent/node-contracts/EP-044.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-044-control-plane-runtime.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-044.txt`, `.agent/node-contracts/EP-044.md`, `.agent/milestone-files/EP-044-M1.txt`, `scripts/nodes/EP-044.sh`, `apps/control-plane/`, `docs/vocabulary/README.md`, `references/ADR-019-control-plane-runtime-vocabulary.md`, `references/ADR-020-runtime-smoke-ownership.md`, `.agent/GRAPH.md`, `.agent/MANIFEST.md`, `AGENTS.md`, `README_FIRST.md`, `NEXUS_GRAPHLOCK_INPUTS.md`, `Cargo.toml`, `Cargo.lock`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md (Rust app under `apps/control-plane/`).
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep044_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Register the workspace member and update the graph amendment documentation (ADR-020) and runtime vocabulary (ADR-019) in the same milestone.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-044.sh M1`

EXPECT:

- `EP-044 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-044 MILESTONE_PASS "M1 EP-044 M1: ok"`

FALLBACK: None for the contract surface; the real binary is mandatory.

COMMIT: `git add -A && git commit -m "[EP-044][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-044.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-044-M2.txt`, `.agent/node-contracts/EP-044.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-044-control-plane-runtime.md`, `.agent/state/LEDGER.md`, `apps/control-plane/`, `config/runtime/`, `tests/runtime/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep044_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-044.sh M2`

EXPECT:

- `EP-044 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-044 MILESTONE_PASS "M2 EP-044 M2: ok"`

FALLBACK: None; real behavior is mandatory.

COMMIT: `git add -A && git commit -m "[EP-044][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-044 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-044-M3.txt`, `.agent/node-contracts/EP-044.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-044-control-plane-runtime.md`, `.agent/state/LEDGER.md`, `apps/control-plane/`, `infra/compose/`, `config/runtime/`, `tests/runtime/`

CONTENT:

1. Prove the real server over real HTTP: bind, serve `/healthz`, `/readyz`, `/v1/capabilities`, and shut down gracefully.
2. Compose the real capability registry and real fabric REST surface; the capability list must be non-empty and tenant-scoped.
3. Create `infra/compose/core.yaml` and `config/runtime/` canonical configuration.
4. Create integration tests whose names begin `ep044_integration_` proving the real endpoints over a real socket.
5. Register the runtime in `COMPONENT_REGISTRY.yaml` only if the component is not already declared (it is declared as `nexus-control-plane`); do not duplicate.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-044.sh M3`

EXPECT:

- `EP-044 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-044 MILESTONE_PASS "M3 EP-044 M3: ok"`

FALLBACK: Run the server binary directly when compose is unavailable; the binary is the source of truth.

COMMIT: `git add -A && git commit -m "[EP-044][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-044 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-044-M4.txt`, `.agent/node-contracts/EP-044.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-044-control-plane-runtime.md`, `.agent/state/LEDGER.md`, `apps/control-plane/`, `infra/compose/`, `config/runtime/`, `tests/runtime/`

CONTENT:

1. Create failure tests whose names begin `ep044_failure_` proving: port conflict, malformed config, missing base domain, shutdown timeout, unhealthy runtime state, empty capability list (readiness not ready), and telemetry redaction.
2. Prove the runtime smoke fails (never ALLOW/success) when the server is absent or unhealthy.
3. Prove graceful shutdown terminates the listener and does not leak processes.
4. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-044.sh M4`

EXPECT:

- `EP-044 M4: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-044 MILESTONE_PASS "M4 EP-044 M4: ok"`

FALLBACK: None; failure behavior is mandatory.

COMMIT: `git add -A && git commit -m "[EP-044][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Prove the complete runtime over real execution, then close the node.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-044-M5.txt`, `.agent/node-contracts/EP-044.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: Only the active ExecPlan progress, Decision Log, ledger, evidence, and the smoke ownership scripts may change in this milestone.

CONTENT:

1. Run the runtime live-fire proof (`scripts/live-fire/LF-029.sh`): start the real server, assert `/healthz` `{"status":"healthy"}`, `/readyz` `{"ready":true}`, `/v1/capabilities` non-empty, then shut down gracefully; write machine-readable evidence under `.agent/state/evidence/`.
2. Prove the runtime smoke gate classification: before EP-044 DONE -> `runtime smoke: not-applicable-before EP-044`; at/after EP-044 -> the smoke runs and must pass against the real server.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-044` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-044.sh M5`
2. `sh scripts/node-verify.sh EP-044`
3. `sh scripts/scope-audit.sh EP-044`

EXPECT:

- `EP-044 M5: ok`
- `node verify EP-044: ok`
- `scope audit EP-044: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-044 MILESTONE_PASS "M5 EP-044 M5: ok"`

FALLBACK: None; the live-fire proof must pass against the real server.

COMMIT: `git add -A && git commit -m "[EP-044][M5] live-fire, operations, and node closure"`

# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-044` and observe `node verify EP-044: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- LF-029 (runtime-smoke): start the real control-plane server, assert all three canonical endpoints over real HTTP, graceful shutdown, and zero leaked processes.

# 10. Idempotence and Recovery

Milestone gates are idempotent: rerunning `sh scripts/nodes/EP-044.sh Mk` after a partial failure only re-executes the same checks. The runtime smoke is fail-closed: absence or unhealthiness of the server is a failure, never a pass.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary (graph amendment + crate, 2026-08-14)
- [x] M2: Core behavior and deterministic invariants (2026-08-14)
- [x] M3: Real dependency and transport integration (2026-08-14)
- [x] M4: Forced failures, abuse cases, and observability (2026-08-14)
- [ ] M5: Live-fire, operations, and node closure

Graph amendment detail (2026-08-14, commits `37e8908` + `9f018fc`): owner
GraphLock amendment created EP-044 as the Control Plane Runtime node between
EP-013 and EP-014. GRAPH.md now 45 nodes; EP-014 DEPS rewired EP-013 ->
EP-044. `scripts/smoke-test.sh` activates the runtime smoke at
`at-least EP-044`; before EP-044 is DONE it prints
`runtime smoke: not-applicable-before EP-044` (never a false PASS). Gate
regression `tests/runtime/smoke-gate-regression.sh` proves: not-invoked
before owner, wiring at EP-044, fail-closed when absent. LF-029 registered.
EP-013 fence amended for the graph-amendment paths. Validations observed:
`blueprint validation: ok`, `scope audit EP-013: ok`, `smoke gate regression:
ok`, `node verify EP-013: ok` (runtime smoke not-applicable-before EP-044),
`EP-013 M5: ok`, expected-files/security/license/reality/format/lint/
dependency ok, adapter parity 8x3505091078 1453. EP-013 closed green
(NODE_DONE 2026-08-15, tag green/EP-013 at `71607cc`).

M1 crate detail (2026-08-14): built `apps/control-plane/` as the
`nexus-control-plane` crate - the real runnable control-plane server.
Modules: `vocabulary` (RuntimeState + ADR-019 locked names), `config`
(ControlPlaneConfig: base domain, bind address, tenant, capability source;
validates host:port, rejects empties), `error` (typed RuntimeErrorCode),
`health` (RuntimeHealth `{"status":"healthy"}`), `readiness`
(RuntimeReadiness `{"ready":true}`), `capabilities` (CapabilityList +
CapabilityListSource port + ConfiguredCapabilityList real adapter),
`lifecycle` (RuntimeLifecycle state machine Starting->Ready->Stopping->
Stopped, fail-closed transitions), `server` (ControlPlaneServer: real axum
0.8.9 router over /healthz /readyz /v1/capabilities, composed ServerState,
bind-once serve with graceful shutdown), `smoke` (RuntimeSmoke contract:
fail closed on any failed probe). Workspace member registered in root
Cargo.toml; Cargo.lock regenerated offline. Real HTTP chain: axum 0.8.9,
tokio 1.53.1, hyper 1.11.0, tower 0.5.3 (all in the offline registry cache,
green license class). 19 ep044_unit tests + ep044_unit_dependency_direction
(forbidden: reqwest/ureq/openbao/headscale/openfga/opa/jsonschema/rusqlite/
sqlx/nats/tonic/prost/clap/keycloak/temporal/postgres; HTTP server chain
allowed per ARCHITECTURE app layer). Observed sentinels: `EP-044 M1: ok`,
`security check: ok`, `license gate: ok`, `reality gate: ok`,
`format check: ok`, `lint: ok`, `dependency audit: ok`,
`blueprint validation: ok`.

M2 core detail (2026-08-14): added the runnable binary
`apps/control-plane/src/main.rs` (reads `NEXUS_BASE_DOMAIN`,
`NEXUS_SMOKE_URL`, `NEXUS_CONTROL_PLANE_BIND`, `NEXUS_TENANT_ID`,
`NEXUS_CAPABILITY_SOURCE`; composes real capability source + lifecycle;
serves until ctrl-c; prints start/stop with lifecycle state). Canonical
runtime config `config/runtime/core.json` (base_domain, bind_address,
tenant_id, capability_source, capabilities, smoke_base_url). Core-behavior
integration tests `apps/control-plane/tests/ep044_core_behavior.rs` (5
ep044_unit_* tests): composition-root router is readiness-gated, capability
source fail-closed on empty, not-ready serialization, smoke contract
requires all probes, lifecycle cannot return to Ready. Observed sentinels:
`EP-044 M2: ok` (25 unit tests), `security check: ok`, `license gate: ok`,
`reality gate: ok`, `format check: ok`, `lint: ok`, `dependency audit: ok`.

M3 real-dependency detail (2026-08-14): proved the real server over real
HTTP. Integration suite `apps/control-plane/tests/ep044_integration_http.rs`
spawns the ACTUAL production binary (`CARGO_BIN_EXE_nexus-control-plane`) as
a child process on a real loopback socket and drives real HTTP/1.1 over
`TcpStream`: `/healthz` -> 200 `{"status":"healthy"}`, `/readyz` -> 200
`{"ready":true}`, `/v1/capabilities` -> 200 non-empty, unknown route -> 404.
Local deterministic bring-up: `infra/compose/core.yaml` (control-plane
service, build from `apps/control-plane/Dockerfile`, port 8443, canonical
env) + `.dockerignore`. Compose config validated (`docker compose config
--quiet` exit 0). Observed sentinels: `EP-044 M3: ok` (4 integration tests
against the real binary), side gates ok.

M4 failure detail (2026-08-14): failure/abuse suite
`apps/control-plane/tests/ep044_failure_modes.rs` (5 ep044_failure_* tests
against the REAL binary): port conflict -> exit nonzero, invalid config
(empty domain, malformed bind) -> exit nonzero with redacted stderr,
runtime-absent smoke -> curl health probe fails (fail closed), graceful
shutdown -> SIGTERM terminates and port closes (no leak), telemetry
redaction -> startup output never contains raw tenant id. Observed
sentinels: `EP-044 M4: ok`, full suite 34 tests (19 lib + 1 dep + 5 core +
5 failure + 4 integration), side gates ok.

# 12. Surprises & Discoveries

- 2026-08-14: The original graph had 44 nodes and no runtime owner; `scripts/smoke-test.sh` activated the runtime smoke at `at-least EP-012`, which outran its dependency owner. EP-044 is the owner amendment.

# 13. Decision Log

- 2026-08-14 | Decision: Create EP-044 as a dedicated GraphLock node for the Nexus Control Plane Runtime, per the owner GraphLock amendment decision of 2026-08-14 (resolution of `NODE_BLOCKED EP-013 GLOBAL_GATE_PREREQUISITE_UNOWNED`). Evidence: exhaustive graph read showed `apps/control-plane`, `apps/edge`, `apps/cli`, `infra/compose/` in ARCHITECTURE.md + COMPONENT_REGISTRY but in NO node fence/ExecPlan/contract; zero `main.rs`; the apps/README "control-plane node (graph EP-007+)" is a phantom. Alternatives: hard-code the smoke to EP-035/036/042/043 (rejected: those nodes do not build the runtime; the owner directive forbids mapping smoke to a node that does not actually build it); leave EP-013 blocked forever (rejected: owner resolution). Consequence: the graph is now 45 nodes; EP-014's DEPS rewired from EP-013 to EP-044; the runtime smoke activates at `at-least EP-044`; before EP-044 is DONE the stage is `not-applicable-before EP-044`; at/after EP-044 it is mandatory and fail-closed. Reversal: revert the graph amendment commit and restore smoke activation. Security: the runtime is the composition root; secrets travel as references; the smoke carries no credentials. License: axum/tokio additions are MIT/Apache-2.0 (green class); recorded in ADR-019/ADR-020. Compatibility: additive node; prior green tags and ledger history preserved.
- 2026-08-14 | Decision: Build `apps/control-plane/` as the `nexus-control-plane` crate with the real axum 0.8.9 HTTP server chain (tokio 1.53.1, hyper 1.11.0, tower 0.5.3 - all already present in the offline registry cache, all MIT/Apache-2.0 green class; no new license class). The server owns the canonical `/healthz`, `/readyz`, `/v1/capabilities` routes with real handlers; readiness is driven by the real RuntimeLifecycle; capabilities come from a real CapabilityListSource (never fabricated at request time); graceful shutdown binds once and stops on signal (no TOCTOU). Evidence: `EP-044 M1: ok` (19 unit + 1 dep-direction), `format check: ok`, `lint: ok`, `license gate: ok`, `reality gate: ok`. Alternatives: std-only HTTP server (rejected: ARCHITECTURE.md names Axum for apps/control-plane; hand-rolling HTTP is not a selected component); hyper directly (rejected: axum is the declared app framework and composes hyper/tower). Consequence: the real runtime binary exists at the crate boundary; M3 will prove it over real sockets; M5 live-fire will drive the real smoke. Reversal: revert the crate and workspace member. Security: config and responses never carry secrets; errors are typed and redacted. License: axum 0.8.9 MIT; tokio MIT; hyper MIT; tower MIT. Compatibility: additive workspace member; no existing surface changed.
- 2026-08-14 | Decision: Add the runnable binary (`apps/control-plane/src/main.rs`) and canonical runtime config (`config/runtime/core.json`) in M2, plus the core-behavior integration test file. The binary is the source of truth for the runtime smoke: it reads the canonical env surface (`NEXUS_BASE_DOMAIN`, `NEXUS_SMOKE_URL`, `NEXUS_CONTROL_PLANE_BIND`, `NEXUS_TENANT_ID`, `NEXUS_CAPABILITY_SOURCE`), composes real capability source + lifecycle, serves until ctrl-c, and prints lifecycle state on stop. Evidence: `EP-044 M2: ok` (25 unit tests: 19 lib + 5 core-behavior + 1 dep-direction), `format check: ok`, `lint: ok`, `license gate: ok`, `reality gate: ok`, `dependency audit: ok`. Alternatives: config in env only (rejected: node contract requires canonical runtime configuration artifact); keep binary for M3 only (rejected: M2 owns core behavior including the composition root). Consequence: the real runtime is runnable and deterministic; M3 proves it over real sockets; M4 proves failure modes; M5 live-fire drives the real smoke. Reversal: revert M2 commit. Security: config carries no secrets; env surface is documented; responses are typed and redacted. License: none new. Compatibility: additive.
- 2026-08-14 | Decision: Prove the real server over real HTTP by spawning the ACTUAL production binary as a child process in integration tests (not an in-process router double). The test uses `CARGO_BIN_EXE_nexus-control-plane`, real `TcpStream` HTTP/1.1, and asserts the canonical shapes plus 404 for unknown routes. Local deterministic bring-up: `infra/compose/core.yaml` + `apps/control-plane/Dockerfile` (rust:1.97.1-bookworm builder, debian:bookworm-slim runtime, ENTRYPOINT is the real binary). Evidence: `EP-044 M3: ok` (4 integration tests), `docker compose config --quiet` ok. Alternatives: in-process router test (rejected: does not prove the real binary path, env surface, or bind/serve lifecycle); docker-based integration (rejected: slower and less deterministic in CI; the binary is the source of truth per the node contract fallback). Consequence: the runtime is proven over real sockets; compose provides the local profile bring-up for LF-029; M4 proves failure modes; M5 drives the real smoke. Reversal: revert M3 commit. Security: no secrets in the test; child process is the production binary. License: rust/debian base images recorded; binary licenses unchanged. Compatibility: additive.
- 2026-08-14 | Decision: Add the M4 failure/abuse suite (`apps/control-plane/tests/ep044_failure_modes.rs`): 5 ep044_failure_* tests against the REAL binary covering port conflict, invalid config, runtime-absent smoke fail-closed, graceful shutdown/no-leak, and telemetry redaction. Evidence: `EP-044 M4: ok`, full suite 34 tests. Alternatives: in-process failure injection (rejected: M4 CONTENT item 1 requires real failure mechanisms; the binary is the unit under proof); skip observability (rejected: node contract requires observability bootstrap). Consequence: every runtime failure mode is proven fail-closed; M5 drives the real smoke live-fire. Reversal: revert M4 commit. Security: redaction proven; stderr asserted redacted. License: none new. Compatibility: additive tests only.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.

## EP-044 outcomes (pending)

Changed files versus the machine fence: exactly the fence entries. Exact commands and observed sentinels, test and proof evidence, assumptions, provider and hardware status, remaining risks, and green tag to be recorded at node closure.
