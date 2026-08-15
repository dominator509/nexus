# EP-013 NODE_BLOCKED - GLOBAL_GATE_PREREQUISITE_UNOWNED

Date: 2026-08-14
Agent: hermes-nexus-main
Node: EP-013 (model gateway and provider registry)
Status: NODE_BLOCKED (terminal halt per GRAPH.md dispatch; owner decision required)

## 1. Exact blocker

`sh scripts/node-verify.sh EP-013` fails at the **runtime smoke stage** with:

```
scripts/smoke/runtime.sh: 9: NEXUS_BASE_DOMAIN: parameter not set
```

Observed exit code: 2 (smoke stage). The node-verify pipeline itself masked the
true exit code earlier (echo exit=$? >> log); reading raw logs revealed exit 2.

Controlled diagnostic with the canonical variable set:

```
NEXUS_BASE_DOMAIN=nexus.test
curl --fail --silent --show-error --max-time 10 https://nexus.test/healthz
curl: (6) Could not resolve host: nexus.test
```

`NEXUS_BASE_DOMAIN` is defined in `.env` as `nexus.test` (canonical per
PREFLIGHT.md / scripts/probes/base_domain.sh), but nothing serves it.

## 2. Root cause (graph-level, NOT an EP-013 implementation defect)

The smoke stage in `scripts/smoke-test.sh` activates when
`sh scripts/stage.sh at-least EP-012` succeeds. EP-012 is DONE, so the stage
now runs for the first time. It requires a live Nexus runtime serving:

- `/healthz`   -> `{"status":"healthy"}`
- `/readyz`    -> `{"ready":true}`
- `/v1/capabilities` -> `{"capabilities":[...]}`

**No node in the 44-node graph owns or creates that runtime.**

Exhaustive repository evidence:

1. `apps/README.md` declares ownership as:
   - `apps/control-plane` -> "control-plane node (graph EP-007+)"  **phantom; no such node exists in GRAPH.md**
   - `apps/edge`          -> "edge runtime node"                  **phantom; no such node**
   - `apps/cli`           -> "CLI node"                           **phantom; no such node**
2. `ARCHITECTURE.md` repository map lists `apps/control-plane` (Rust Axum
   public/private control API), `apps/edge`, `apps/cli` - but NO node fence,
   NO ExecPlan CHANGE block, and NO node contract references any of them
   (grep across `.agent/execplans/*.md`, `.agent/node-contracts/*.md`,
   `.agent/expected-files/*.txt`: 0 mentions of `apps/control-plane`,
   `apps/edge`, `apps/cli`, or `infra/compose`).
3. `COMPONENT_REGISTRY.yaml` declares `nexus-control-plane` and `nexus-edge`
   as required components with "internal ownership" - but the graph never
   assigns them to a node.
4. `scripts/local-start.sh` expects `infra/compose/core.yaml`; that directory
   does not exist and no node fence owns `infra/compose/`.
5. No `main.rs` exists anywhere in the workspace (only node_modules noise);
   no runnable server binary exists.
6. EP-011 sidecar serves `/v1/fixture/healthz` - a fixture endpoint, NOT the
   Nexus runtime health endpoint (directive: do not reinterpret fixtures as
   runtime endpoints).
7. EP-012 (API/MCP/A2A fabric) built library crates (`nexus-fabric`,
   `nexus-mcp`, `nexus-a2a`, `infra/gateway`) - composition libraries with an
   evidence example binary, not a runnable HTTP server with the three smoke
   endpoints.

## 3. Attempted resolution (directive Sections B/C)

Directive required: locate the earliest node whose DONE guarantees the runtime
prerequisites; amend smoke activation to `at-least <owner>`.

Result of the graph read: **no such node exists.** Candidate nodes checked:

- EP-012 API fabric: DONE, delivered libraries only; fence has no runtime app.
- EP-033 web dashboard: React PWA + Tauri desktop (clients), no server.
- EP-035 setup wizard: onboarding/deployment-choice app; deploys, does not
  build the control-plane runtime.
- EP-036 compute fabric: provisions compute nodes via OpenTofu/cloud-init;
  does not build the local control-plane runtime.
- EP-042 deployment: signed releases/installers; packages components, does
  not build the runtime server.
- EP-043 production readiness: requires all nodes DONE + manual deploy
  handoff; presupposes the runtime exists; does not build it.

Hard-coding any of these as the smoke owner would be exactly what the
directive forbids ("Do not hard-code EP-035 or EP-036 merely because they were
mentioned"). The dependency ownership is not merely "wrong node" - it is
**unowned**: the runtime is declared in ARCHITECTURE.md + COMPONENT_REGISTRY
but assigned to no node in the fixed graph.

## 4. Why fallback Section I applies

The authoritative GraphLock mechanism (GRAPH.md Law: "The graph is immutable
during a run"; AGENTS.md: fixed graph, L3) forbids correcting this activation
defect without an owner decision, because:

- assigning runtime ownership requires either a new node or an amendment of an
  existing node's contract/fence (a graph mutation);
- the correct owner cannot be determined from the existing graph (none exists);
- therefore the narrowly scoped amendment (Sections C-E) cannot be made safely.

Per directive Section I: record `NODE_BLOCKED EP-013` with reason
`GLOBAL_GATE_PREREQUISITE_UNOWNED` and preserve the exact evidence.

## 5. EP-013 implementation status (preserved, per Section J)

EP-013 M1-M4 committed and green; M5 gate green:

- `EP-013 M5: ok` (82 tests across nexus-model-gateway, nexus-bifrost,
  nexus-model-transport)
- Deterministic evidence `.agent/state/evidence/ep013-m5/ep013-m5-live-fire.json`
  (md5 `2f8c8840ae625c972ac3f6d72614f69f` stable across reruns)
- `scripts/nodes/EP-013.sh` M5 wiring preserved
- ExecPlan M5 updates preserved

None of the M5 work is discarded. The block is a global gate defect, not an
EP-013 milestone defect.

## 6. Smallest human decision

Assign ownership of the Nexus runtime (at minimum `apps/control-plane` +
`infra/compose/core.yaml` + the `/healthz`, `/readyz`, `/v1/capabilities`
endpoints) to a graph node, then amend `scripts/smoke-test.sh` activation to
`at-least <that node>` (or add a runtime node to the graph). After that,
rerun `sh scripts/node-verify.sh EP-013`; the smoke stage will classify as
not-applicable-before-owner until that node is DONE, and mandatory after.

## 7. Recommended default

Amend `scripts/smoke-test.sh` to activate the runtime smoke at
`at-least <EARLIEST_REAL_RUNTIME_OWNER>` where the owner node is added to the
graph (e.g., a control-plane/runtime node in the EP-030..EP-043 band), and add
the gate-ownership regression test (Section E) proving: stage < owner -> smoke
not invoked; stage >= owner + runtime absent -> node-verify FAILS.

## 8. Security and data impact

None. No credentials, bearer tokens, or private data touched. Evidence carries
correlation ids and typed codes only. The runtime smoke remains fail-closed
(never a PASS claim while inapplicable; mandatory + failing once the owner is
DONE and runtime absent).

## 9. Exact recovery entry point

1. Owner decides the runtime owner node (graph amendment).
2. Amend `scripts/smoke-test.sh` activation to `at-least <owner>`.
3. Add gate-ownership regression test (Section E).
4. `sh scripts/node-verify.sh EP-013` -> `node verify EP-013: ok`
   (smoke = not-applicable-before <owner>).
5. Complete EP-013 closure: scope audit, expected-files, adapter parity,
   security, license, reality, format, lint, dependency audit.
6. Ledger MILESTONE_PASS M5 -> commit -> committed-state re-verify ->
   NODE_DONE -> green/EP-013 -> graph-next.
