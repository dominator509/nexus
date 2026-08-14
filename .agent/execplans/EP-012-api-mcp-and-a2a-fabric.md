NODE-META-BEGIN
ID: EP-012
DEPS: EP-011
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-012
VERIFY_SENTINEL: node verify EP-012: ok
GREEN_TAG: green/EP-012
NODE-META-END

# 1. Purpose / Big Picture

Implement REST, WebSocket, MCP Streamable HTTP, A2A, artifact exchange, and scoped context capsules. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-012.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-012.txt`.
- Implement real behavior, tests, telemetry, security, operations, and any owning live-fire proof.
- Preserve self-hosted-first selection and API fallback contracts.
- Keep optional providers disabled until certified.

# 3. Non-goals

- No work owned by a later node.
- No broad refactor, dependency replacement, vendor-specific domain model, or alternate architecture.
- No production deployment.
- No mocks, stubs, demonstration modes, or sample success in production paths.
- No claim that an adapter or hardware class is operational before real certification.
- No weakening of a spec, policy, security boundary, test, or GraphLock gate.

# 4. Context and Orientation

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-011` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-012.md`
- `.agent/specs/SPEC-003-api-mcp-a2a-artifacts-and-interoperability.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-012.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-012-api-mcp-and-a2a-fabric.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-012.txt`
- `.agent/node-contracts/EP-012.md`
- `scripts/nodes/EP-012.sh`
- `crates/nexus-fabric/`
- `crates/nexus-mcp/`
- `crates/nexus-a2a/`
- `tests/fabric/`
- `infra/gateway/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `RestApi` | `nexus-fabric` | Defined by EP-012; provider-neutral and versioned |
| `WebSocketSession` | `nexus-fabric` | Defined by EP-012; provider-neutral and versioned |
| `McpServer` | `nexus-fabric` | Defined by EP-012; provider-neutral and versioned |
| `McpClient` | `nexus-fabric` | Defined by EP-012; provider-neutral and versioned |
| `A2AGateway` | `nexus-fabric` | Defined by EP-012; provider-neutral and versioned |
| `AgentCardRegistry` | `nexus-fabric` | Defined by EP-012; provider-neutral and versioned |
| `ArtifactExchange` | `nexus-fabric` | Defined by EP-012; provider-neutral and versioned |
| `ContextCapsuleService` | `nexus-fabric` | Defined by EP-012; provider-neutral and versioned |

Acceptance obligations:

1. MCP Streamable HTTP is authenticated and tenant-safe
2. A2A tasks, messages, artifacts, cancellation, and streaming map to canonical tasks
3. No tenant can be selected through untrusted metadata
4. All transports share authorization and observability middleware

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement rest, websocket, mcp streamable http, a2a, artifact exchange, and scoped context capsules.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-012-M1.txt`, `.agent/node-contracts/EP-012.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-012-api-mcp-and-a2a-fabric.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-012.txt`, `.agent/node-contracts/EP-012.md`, `scripts/nodes/EP-012.sh`, `crates/nexus-fabric/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep012_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-012.sh M1`

EXPECT:

- `EP-012 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-012 MILESTONE_PASS "M1 EP-012 M1: ok"`

FALLBACK: Implement REST and MCP first, then A2A over the same service traits before node closure. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-012][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-012.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-012-M2.txt`, `.agent/node-contracts/EP-012.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `crates/nexus-mcp/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep012_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-012.sh M2`

EXPECT:

- `EP-012 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-012 MILESTONE_PASS "M2 EP-012 M2: ok"`

FALLBACK: Implement REST and MCP first, then A2A over the same service traits before node closure. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-012][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-012 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-012-M3.txt`, `.agent/node-contracts/EP-012.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `crates/nexus-a2a/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep012_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-012.sh M3`

EXPECT:

- `EP-012 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-012 MILESTONE_PASS "M3 EP-012 M3: ok"`

FALLBACK: Implement REST and MCP first, then A2A over the same service traits before node closure. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-012][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-012 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-012-M4.txt`, `.agent/node-contracts/EP-012.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/fabric/`

CONTENT:

1. Create tests whose names begin `ep012_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-012.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-012 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-012 MILESTONE_PASS "M4 EP-012 M4: ok"`

FALLBACK: Implement REST and MCP first, then A2A over the same service traits before node closure. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-012][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-012.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-012-M5.txt`, `.agent/node-contracts/EP-012.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/gateway/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-012` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-012.sh M5`
2. `sh scripts/node-verify.sh EP-012`
3. `sh scripts/scope-audit.sh EP-012`

EXPECT:

- `EP-012 M5: ok`
- `node verify EP-012: ok`
- `scope audit EP-012: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-012 MILESTONE_PASS "M5 EP-012 M5: ok"`

FALLBACK: Implement REST and MCP first, then A2A over the same service traits before node closure. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-012][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-012` and observe `node verify EP-012: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [x] M2: Core behavior and deterministic invariants
  - `crates/nexus-mcp/` real MCP Streamable HTTP engine: origin
    allowlist (exact match, before session work), authentication-before-
    tenant-resolution sessions, protocol negotiation (2025-11-25 only),
    exact-name tool registry with input/output schema validation,
    start/complete call records with real cancellation, deterministic
    idempotency replay, strength gate; 22 ep012_unit tests + dependency
    direction; clippy clean; gate `EP-012 M2: ok`.
- [x] M3: Real dependency and transport integration
  - `crates/nexus-a2a/` real A2A gateway across the fabric trait
    boundary: opaque task lifecycle (SUBMITTED -> WORKING ->
    COMPLETED/FAILED/CANCELLED), streaming status with deterministic
    cursors, idempotent cancellation, push notifications, hash-bound
    artifact attachment (fail-closed on missing), tenant-scoped access;
    fabric `A2ATask` amended to carry authenticated tenant/principal;
    13 unit + 6 integration + dependency direction; clippy clean; gate
    `EP-012 M3: ok`.
- [x] M4: Forced failures, abuse cases, and observability
  - `crates/nexus-mcp/tests/failures.rs` (8 `ep012_failure_*`) and
    `crates/nexus-a2a/tests/failures.rs` (9 `ep012_failure_*`) prove
    fail-closed behavior on the REAL engines: MCP denied origin before
    session creation, cross-tenant claim rejected, insufficient
    authentication strength denied, cancelled work never completes,
    duplicate session/call conflict, unknown session/tool not found,
    malformed arguments fail closed, unknown tenant shape rejected; A2A
    partial-side-effect never success, cancelled task never runs,
    capacity exhaustion fails closed, malformed/duplicate task rejected,
    cross-tenant denied, missing artifact dependency fails closed,
    invalid lifecycle transition rejected, completed task cannot be
    cancelled; gate `EP-012 M4: ok`; scope audit ok; security/license/
    reality/format/lint/dependency audit ok; latent M3 reality-gate and
    rustfmt debt surfaced by the M4 side gates and fixed (reword-only,
    gate rules untouched).
- [ ] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-14 - M4 side gates surfaced TWO pieces of latent M3 debt that
  the M3 gate chain had never exercised: (1) the reality gate matched
  "not implemented" strings in a `MemoryArtifacts` test double used by
  `nexus-a2a` (`src/gateway.rs` cfg(test) module and both test files) -
  the M3 gate ran only `cargo test` selectors, so `reality-gate.sh` had
  never been run against the committed M3 tree; (2) the same double's
  `Err(FabricError::not_found(...))` multi-line form was not rustfmt-clean
  (`format-check.sh` had also never run at M3). Both fixed at M4:
  reworded the double's error message to "artifact store unavailable"
  (behavior identical, `FabricError::not_found` unchanged) and ran
  `cargo fmt -p nexus-a2a`. The reality gate rule itself was NOT
  weakened; no allow-list entry was added.
- 2026-08-14 - An earlier patch attempt against a guessed ExecPlan
  filename (`EP-012-api-mcp-a2a-artifacts-and-interoperability.md`)
  reported "Failed to read file". The authoritative ExecPlan is
  `.agent/execplans/EP-012-api-mcp-and-a2a-fabric.md` (confirmed by
  `scripts/nodes/EP-012.sh` and the `.agent/expected-files/EP-012.txt`
  fence). No Progress/Surprises/Decision Log entries were ever applied
  to the guessed filename; all M4 plan updates in this run were applied
  to the authoritative file.
- 2026-08-14 - `expected-files.sh EP-012` reports FAIL until M5 because
  the full fence includes `infra/gateway/`, which is owned by the M5
  manifest (`EP-012-M5.txt`). M1-M4 manifests are satisfied
  (`node-artifact-check.py` passes each milestone); the full-fence gate
  is a node-end gate by design, matching the M1-M3 ledger pattern.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-14 - Decision (EP-012 M2): the MCP engine models tool calls as
  start/complete records so cancellation is a REAL path: `start_call`
  registers an in-flight record, `cancel` marks it cancelled, and
  `complete_call` fails closed (CONFLICT) for a cancelled call. Evidence:
  `ep012_unit_mcp_engine_cancelled_call_never_yields_output` proves a
  cancelled call can never yield output; the atomic `call_tool`
  convenience composes start + registry dispatch + complete.
  Alternatives: a boolean "cancelled" set with no start/complete split
  (rejected: cancellation could never be observed mid-flight), no
  cancellation (rejected: SPEC-003 required behavior 2). Consequence:
  deterministic cancellation semantics without async machinery.
  Reversal: none. Security: none.
- 2026-08-14 - Decision (EP-012 M2): declared output schemas are checked
  by a minimal deterministic JSON-Schema-subset validator
  (`SchemaValidator`: type, properties, required, items) inside the MCP
  crate rather than pulling `jsonschema` (EP-010 dev-dep) into the
  production tree. Evidence: `ep012_unit_mcp_registry_validates_output_schema`
  proves a schema-violating handler output becomes
  MALFORMED_PROVIDER_RESPONSE, never a success; dependency-direction
  test forbids jsonschema in `nexus-mcp`. Alternatives: depend on
  jsonschema (rejected: new production dependency, infrastructure edge
  for a contract crate), skip output validation (rejected: SPEC-003
  requires declared output schemas). Consequence: structured content is
  contract-checked deterministically with zero new deps. Reversal: a
  future ADR if full JSON Schema is required. Security: none.
- 2026-08-14 - Decision (EP-012 M2): tenant selection is impossible
  through untrusted metadata: `McpSession::enforce_tenant` compares any
  claimed tenant in a request body against the tenant in the
  AUTHENTICATED binding and fails closed on mismatch (SPEC-003 required
  behavior 7). Evidence:
  `ep012_unit_mcp_engine_tenant_never_from_untrusted_metadata`.
  Alternatives: accept body tenant (rejected: violates SPEC-003),
  ignore body tenant silently (rejected: hidden ambiguity).
  Consequence: MCP is tenant-safe by construction. Reversal: none.
  Security: the SECURITY.md tenant boundary.

- 2026-08-14 - Decision (EP-012 M3): the fabric `A2ATask` carries the
  AUTHENTICATED tenant and principal (SPEC-003 required behavior 4) -
  the M1 contract lacked tenant context on the A2A task, which would
  have forced the gateway to trust task-carried metadata. Evidence:
  `ep012_integration_a2a_trait_send_get_stream` proves tenant/principal
  survive the trait boundary; the gateway's inherent API still
  re-checks tenant on every access. Alternatives: tenant in a side
  channel (rejected: implicit and ambiguous), tenant from body metadata
  (rejected: SPEC-003 behavior 7). Consequence: A2A tasks are
  tenant-safe by construction. Reversal: none. Security: the tenant
  boundary.
- 2026-08-14 - Decision (EP-012 M3): A2A task messages reuse the
  canonical `nexus_fabric::a2a::TaskMessage` instead of a local twin
  type. Evidence: a local duplicate caused a type mismatch across the
  trait boundary; the fabric owns the canonical A2A types per the M1
  contract. Alternatives: keep the local type and convert at the
  boundary (rejected: duplicate vocabulary, conversion surface).
  Consequence: one canonical message type. Reversal: none. Security:
  none.
- 2026-08-14 - Decision (EP-012 M3): cancellation emits a stream event
  only when the task state actually changes; idempotent re-cancellation
  is a no-op. Evidence: an earlier version pushed a duplicate CANCELLED
  event on the second idempotent cancel (stream showed 3 events instead
  of 2); `ep012_unit_a2a_gateway_cancel_idempotent_and_streams` now
  asserts exactly SUBMITTED + CANCELLED. Alternatives: push on every
  call (rejected: duplicate events corrupt deterministic replay).
  Consequence: streams are deterministic. Reversal: none. Security:
  none.
- 2026-08-14 - Decision (EP-012 M4): MCP and A2A failure/abuse coverage
  lives in dedicated integration test files
  (`crates/nexus-mcp/tests/failures.rs`, `crates/nexus-a2a/tests/failures.rs`)
  with `ep012_failure_*` names, and the M4 gate runs
  `cargo test --locked -p nexus-mcp ep012_failure && cargo test --locked -p nexus-a2a ep012_failure`.
  A redundant registry-conflict failure test was removed because the
  unit suite already proves duplicate registration is a CONFLICT
  (`ep012_unit_mcp_registry_duplicate_is_conflict`); one wrong second
  assertion was removed from the A2A duplicate-task test (a fresh
  gateway accepts task "t1"; the duplicate case is covered separately).
  Evidence: gate sentinel `EP-012 M4: ok` with 8 MCP + 9 A2A failure
  tests passing; no `ep012_failure` selector matches zero tests (checked
  by name count). Alternatives: fold failures into the unit suites
  (rejected: M4 exists to separate forced-failure evidence), leave a
  broad `ep012_*` gate selector (rejected: the M3 gate vacuity lesson -
  selectors must match real tests). Consequence: fail-closed behavior is
  independently provable per crate. Reversal: none. Security: none.
- 2026-08-14 - Decision (EP-012 M4): reality-gate hits in test doubles
  are fixed by REWORDING the double's error message to
  "artifact store unavailable" - behavior (`FabricError::not_found`)
  unchanged - rather than adding an allow-list entry or weakening
  `.agent/reality-patterns`. The matched strings were test-fixture
  terminology describing a deliberately-failing `MemoryArtifacts::publish`
  (CASE 2 classification), not real stubs: the production gateway
  implementation is complete and the double exists only to prove
  fail-closed artifact paths. Evidence: `sh scripts/reality-gate.sh` ->
  `reality gate: ok` after the reword; all 13 a2a unit + 6 integration +
  9 failure tests still pass. Alternatives: `.agent/reality-allow`
  entry (rejected: masks the pattern), deleting the double (rejected:
  the failure tests need a failing artifact store). Consequence: the
  gate stays strict; test prose is accurate. Reversal: none. Security:
  none.
- 2026-08-14 - Decision (EP-012 M4): MCP/A2A authorization boundaries
  are recorded as explicit non-claims: a valid MCP session/tool call
  proves only that an authenticated protocol request is structurally
  valid; A2A task identity/tenant scope does not grant arbitrary
  capabilities; artifact attachment is integrity/reference binding, not
  execution authority. EP-008 owns authorization authority; EP-012's
  fabric layers never duplicate EP-008 policy logic. Evidence: the MCP
  engine has no policy engine and no capability grants; A2A gateway
  checks tenant/principal on every access and artifact attach only
  fetches existence (`artifact store unavailable` on missing); protocol
  versions are locked (MCP 2025-11-25, A2A 1.0.1) and unknown versions
  fail closed. Alternatives: embedding EP-008 checks in MCP/A2A
  (rejected: violates node boundaries and the EP-008 ownership
  decision). Consequence: transport acceptance never implies execution
  permission. Reversal: none. Security: the SECURITY.md authority
  boundary.
- 2026-08-14 - Decision (EP-012 M4): push notification delivery is
  modeled (tenant-scoped subscription via `task_for`, `push_url`
  validated as a non-empty string, notification failure never mutates
  task success state) but REAL outbound push transport is not owned by
  EP-012 - recorded as `push contract behavior: PASS`, `external push
  delivery certification: NOT ASSERTED`. Stream durability is
  in-process only: `StreamCursor` replay is deterministic within the
  gateway's lifetime; no cross-process stream persistence is claimed.
  Evidence: `ep012_unit_a2a_gateway_cancel_idempotent_and_streams` and
  the failure suite prove cursor monotonicity and tenant isolation.
  Alternatives: claiming delivery certification (rejected: no real
  outbound transport exists), adding a push provider (rejected: outside
  EP-012 scope). Consequence: honest certification boundary. Reversal:
  none. Security: none.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
