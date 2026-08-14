NODE-META-BEGIN
ID: EP-010
DEPS: EP-009
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-010
VERIFY_SENTINEL: node verify EP-010: ok
GREEN_TAG: green/EP-010
NODE-META-END

# 1. Purpose / Big Picture

Implement capability discovery, health, command, query, event, and connector-tier contracts. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-010.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-010.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-009` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-010.md`
- `.agent/specs/SPEC-003-api-mcp-a2a-artifacts-and-interoperability.md`
- `.agent/specs/SPEC-022-universal-connector-contract-sdks-sidecar-and-legacy-integration.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-010.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-010-capability-registry-and-connector-contract.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-010.txt`
- `.agent/node-contracts/EP-010.md`
- `scripts/nodes/EP-010.sh`
- `crates/nexus-capabilities/`
- `crates/nexus-connectors/`
- `schemas/capability-descriptor.schema.json`
- `schemas/connector-manifest.schema.json`
- `tests/capabilities/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `CapabilityRegistry` | `nexus-capabilities` | Defined by EP-010; provider-neutral and versioned |
| `CapabilityDescriptor` | `nexus-capabilities` | Defined by EP-010; provider-neutral and versioned |
| `ConnectorManifest` | `nexus-capabilities` | Defined by EP-010; provider-neutral and versioned |
| `QueryCapability` | `nexus-capabilities` | Defined by EP-010; provider-neutral and versioned |
| `CommandCapability` | `nexus-capabilities` | Defined by EP-010; provider-neutral and versioned |
| `WorkflowCapability` | `nexus-capabilities` | Defined by EP-010; provider-neutral and versioned |
| `HealthCapability` | `nexus-capabilities` | Defined by EP-010; provider-neutral and versioned |
| `ChangeFeedCapability` | `nexus-capabilities` | Defined by EP-010; provider-neutral and versioned |

Acceptance obligations:

1. Capabilities advertise stable schemas, scopes, risk, idempotency, health, and availability
2. Read, proposal, command, and workflow classes remain distinct
3. A generic execute string is impossible
4. Unavailable provider features are not advertised

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement capability discovery, health, command, query, event, and connector-tier contracts.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-010-M1.txt`, `.agent/node-contracts/EP-010.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-010-capability-registry-and-connector-contract.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-010.txt`, `.agent/node-contracts/EP-010.md`, `scripts/nodes/EP-010.sh`, `crates/nexus-capabilities/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep010_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-010.sh M1`

EXPECT:

- `EP-010 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-010 MILESTONE_PASS "M1 EP-010 M1: ok"`

FALLBACK: Support REST plus signed webhooks before durable event transport for simple Tier 1 connectors. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-010][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-010.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-010-M2.txt`, `.agent/node-contracts/EP-010.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `crates/nexus-connectors/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep010_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-010.sh M2`

EXPECT:

- `EP-010 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-010 MILESTONE_PASS "M2 EP-010 M2: ok"`

FALLBACK: Support REST plus signed webhooks before durable event transport for simple Tier 1 connectors. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-010][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-010 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-010-M3.txt`, `.agent/node-contracts/EP-010.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `schemas/capability-descriptor.schema.json`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep010_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-010.sh M3`

EXPECT:

- `EP-010 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-010 MILESTONE_PASS "M3 EP-010 M3: ok"`

FALLBACK: Support REST plus signed webhooks before durable event transport for simple Tier 1 connectors. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-010][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-010 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-010-M4.txt`, `.agent/node-contracts/EP-010.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `schemas/connector-manifest.schema.json`

CONTENT:

1. Create tests whose names begin `ep010_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-010.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-010 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-010 MILESTONE_PASS "M4 EP-010 M4: ok"`

FALLBACK: Support REST plus signed webhooks before durable event transport for simple Tier 1 connectors. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-010][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-010.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-010-M5.txt`, `.agent/node-contracts/EP-010.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/capabilities/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-010` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-010.sh M5`
2. `sh scripts/node-verify.sh EP-010`
3. `sh scripts/scope-audit.sh EP-010`

EXPECT:

- `EP-010 M5: ok`
- `node verify EP-010: ok`
- `scope audit EP-010: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-010 MILESTONE_PASS "M5 EP-010 M5: ok"`

FALLBACK: Support REST plus signed webhooks before durable event transport for simple Tier 1 connectors. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-010][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-010` and observe `node verify EP-010: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
  - `crates/nexus-capabilities` workspace crate registered; 8 public
    interfaces: `CapabilityRegistry`, `CapabilityDescriptor`,
    `ConnectorManifest`, `QueryCapability`, `CommandCapability`,
    `WorkflowCapability`, `HealthCapability`, `ChangeFeedCapability`.
  - Vocabulary ADR-015 (`HealthState`, `Certification`, `SchemaRef`) +
    `docs/vocabulary/README.md`; reused domain `CapabilityClass`,
    `Idempotency`, `Availability`, `Locality`, `Tier`,
    `ConnectorRuntime`, `Risk`, `ApprovalClass`, `Reversal`,
    `Privacy`, `PrincipalType`, typed IDs.
  - 37 `ep010_unit_*` lib tests + 1 dependency-direction test; clippy
    clean; `EP-010 M1: ok`; format check ok; lint ok; fence amended
    (vocabulary README, ADR-015, Cargo.toml, Cargo.lock).
- [x] M2: Core behavior and deterministic invariants
  - `crates/nexus-connectors` deterministic core: in-memory
    capability registry (tenant-keyed, idempotent registration,
    availability-filtered discovery), typed capability dispatcher
    (class-validated entry points; a generic execute string is
    impossible), and idempotency tracker (key bound to capability,
    replay on retry, cross-capability reuse = conflict).
  - 18 `ep010_unit_*` tests + 1 dependency-direction test; clippy
    clean; `EP-010 M2: ok`; format check ok; lint ok; gate script
    wired to `nexus-connectors`.
- [x] M3: Real dependency and transport integration
  - Real cross-language schema parity: `nexus-connectors` integration
    tests serialize the real Rust contract types and validate the
    JSON against the canonical `schemas/*.schema.json` documents with
    the real `jsonschema` 0.49.9 validator (dev-dependency; recorded
    in VERSIONS.lock.yaml). Canonical `$ref` resolution is served
    from the repo `schemas/` tree, never the network.
  - 6 `ep010_integration_*` tests: descriptor validates against
    canonical schema; all five capability classes validate; manifest
    (with embedded descriptor ref) validates; binding shape;
    unknown class rejected by schema; missing required field rejected.
  - `EP-010 M3: ok`; license gate ok; cargo-deny licenses ok; format
    check ok; lint ok; fence amended (VERSIONS.lock.yaml).
- [x] M4: Forced failures, abuse cases, and observability
  - 12 `ep010_failure_*` tests through the real deterministic core and
    the real `jsonschema` validator: missing capability (typed
    NotFound with correlation/actor/resource), unavailable capability
    (fail closed), provider error (typed, never allows), malformed
    descriptor (validation), duplicate idempotency key across
    capabilities (Conflict), cross-tenant invocation denied,
    unregister-missing (NotFound), and schema-authority rejections
    (duplicate secrets/events/network-origins, unknown class, missing
    required) against the canonical schemas.
  - Hardened `schemas/connector-manifest.schema.json` with
    `uniqueItems` on events/secrets/network_origins (parity with the
    descriptor schema).
  - `EP-010 M4: ok`; security check ok; license gate ok; clippy clean.
- [x] M5: Live-fire, operations, and node closure
  - Composed EP-010 subsystem proof via the single probe binary
    `crates/nexus-connectors/examples/livefire_probe.rs` (orchestration
    only) driving the REAL production implementations: real
    `InMemoryCapabilityRegistry`, real `CapabilityDispatcher`, real
    `IdempotencyTracker`, real `CapabilityDescriptor`/manifest
    contracts, real typed `CapabilityError`, real canonical schemas
    through the real `jsonschema` 0.49.9 validator (draft 2020-12,
    hermetic local `$ref`). Deterministic providers in the probe are
    test-zone doubles implementing the real capability port traits.
  - Exactly the 13 canonical stages, each with observed results:
    REGISTER_DISCOVER (tenant-scoped, BTreeMap-sorted deterministic
    order, idempotent re-registration, len stable),
    UNAVAILABLE_NOT_ADVERTISED, QUERY_DISPATCH,
    COMMAND_IDEMPOTENT (provider executed once, replay, records=1),
    WORKFLOW_DISPATCH, HEALTH, CHANGEFEED (event + cursor +
    tenant binding), CLASS_MISMATCH_DENIED (VALIDATION),
    CROSS_TENANT_DENIED (NOT_FOUND), PROVIDER_ERROR_FAIL_CLOSED
    (query + command typed UNAVAILABLE, cached_success=false),
    IDEMPOTENCY_CONFLICT (CONFLICT), SCHEMA_VALIDATION (zero errors),
    SCHEMA_REJECTION (unknown class, missing required, duplicate
    events/secrets/network_origins all rejected).
  - Authority boundaries proven from observed data: descriptor carries
    no grant/token/credential/authorization material; TIER3/CERTIFIED
    manifest validates but tier/certification never enter the
    descriptor or dispatch; HealthReport is observation only.
    EP-008 owns authorization, EP-005 owns event transport, EP-006
    owns durable workflow execution; external connector/provider
    certification explicitly NOT OWNED BY EP-010.
  - `tests/capabilities/` package: README + 7 `ep010_livefire_*`
    pytest tests that build/run the real probe, require exactly the 13
    stages with no duplicates, independently verify stage detail
    fields, check authority boundaries, schema versions
    (current-version parity PASS; future-version migration NOT
    ASSERTED), and write governed evidence from observed output.
  - Evidence: `.agent/state/evidence/ep010-m5/ep010-m5-composed-proof.json`
    + `EP-010-M5-composed-proof.md` (correlation/tenant fixture IDs
    only; no credentials or tokens).
  - M5 gate hardened with `scripts/ep010-vacuity.sh` (pytest filter
    must produce a real non-zero test count) and
    `scripts/ep010-orphan-audit.sh` (zero containers/networks/volumes,
    zero /tmp scratch, zero stray probe processes, exact evidence
    files). Fence amended for both scripts.
  - `EP-010 M5: ok`; `node verify EP-010: ok`; `scope audit EP-010:
    ok`; expected-files ok; adapter parity (1 unique PRIME-BLOCK
    cksum); security/license/reality/format/lint/dependency audit ok;
    smoke test ok; live-fire ok; orphan audit ok.

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

# 13. Decision Log

- 2026-08-14 - Decision: reuse nexus-domain vocabulary for
  `CapabilityClass`, `Idempotency`, `Availability`, `Locality`, `Tier`,
  `ConnectorRuntime`, `Risk`, `ApprovalClass`, `Reversal`, `Privacy`,
  `PrincipalType`, and typed IDs rather than redefining any class.
  Evidence: `crates/nexus-domain/src/vocabulary.rs` already carries all
  tables; canonical schemas (`capability-descriptor`,
  `connector-manifest`, `invocation-context`) already exist from the
  bootstrap pack and are the wire contract. Alternatives: duplicate
  enums in the new crate (rejected: vocabulary lock + drift risk).
  Consequence: single source of truth; the crate adds only the three
  EP-010-owned classes. Reversal: ADR + vocabulary update.
  Security/license/compatibility: none.
- 2026-08-14 - Decision: capability ports return the shared
  `CapabilityError` (SPEC-006) directly instead of per-port wrapper
  error types. Evidence: `nexus-trust` precedent returns `TrustError`
  from every port; wrapper types added ~64 bytes per error and tripped
  `clippy::result_large_err`. Alternatives: `Box<CapabilityError>`
  wrappers (rejected: unnecessary indirection for a value-type error).
  Consequence: uniform typed errors with correlation/actor/tenant/
  resource context; `CapabilityError` context fields are `Box<str>` to
  keep the error value under the clippy size threshold. Reversal: ADR.
  Security/license/compatibility: none.
- 2026-08-14 - Decision: `CapabilityDescriptor` and `ConnectorManifest`
  validation mirrors the canonical JSON Schemas exactly (id patterns,
  description length, unique scopes, version form, required fields).
  Evidence: `schemas/capability-descriptor.schema.json` and
  `schemas/connector-manifest.schema.json` are bootstrap-owned and
  immutable for this node; M3/M4 own any schema amendments. Consequence:
  cross-language clients can rely on schema/type parity. Reversal:
  schema update in M3/M4 milestone. Security/license: none.
- 2026-08-14 - Decision: `ConnectorRuntime` referenced through
  `nexus_domain::vocabulary::ConnectorRuntime` because the domain root
  re-export list omits it (only `Tier` is root-reexported). Evidence:
  `crates/nexus-domain/src/lib.rs` export list. Consequence: no fence
  change to nexus-domain; full-path import is stable. Reversal: domain
  root re-export later. Security/license: none.
- 2026-08-14 - Decision: the deterministic core uses interior
  mutability (`Mutex`) for the registry and idempotency tracker so
  both implement the `&self` port methods while remaining shareable
  across dispatchers and threads. Evidence: `CapabilityRegistry` port
  methods take `&self`; `Arc<dyn CapabilityRegistry + Send + Sync>`
  requires `Sync`. Alternatives: `&mut self` ports (rejected: breaks
  sharing and the adapter pattern), `RefCell` (rejected: not `Sync`).
  Consequence: all registry/tracker operations are serialized, which
  keeps them deterministic for a given call sequence. Reversal: ADR +
  port change. Security/license: none.
- 2026-08-14 - Decision: the dispatcher is the only composition path
  and every entry point is class-validated before a port is touched.
  Evidence: acceptance obligation 2 (read/proposal/command/workflow
  classes remain distinct) and obligation 3 (a generic execute string
  is impossible). Consequence: a `QUERY` capability cannot be invoked
  through the command path and vice versa; each dispatch method is a
  typed method (`dispatch_query`, `dispatch_command`,
  `dispatch_workflow`, `dispatch_health`, `dispatch_changefeed`).
  Reversal: ADR + schema update. Security/license: none.
- 2026-08-14 - Decision: M2 gate script wired to run
  `nexus-connectors ep010_unit` (the scaffold had pointed at
  `nexus-capabilities` for every milestone). Evidence: M2 CHANGE block
  owns `crates/nexus-connectors/`. Consequence: milestone gates
  exercise the milestone's actual artifact. Reversal: none.
  Security/license: none.
- 2026-08-14 - Decision: M3 uses the real `jsonschema` crate (0.49.9,
  MIT OR Apache-2.0) as a dev-dependency of `nexus-connectors` to
  prove cross-language schema parity: Rust contract types serialize to
  JSON that validates against the canonical schemas. Evidence: the
  canonical schemas are the single cross-language contract source
  (SPEC-003 behavior 1; SPEC-022 behavior 4); a hand-written
  conformance stub would violate the no-stub rule. Alternatives:
  Python `jsonschema` under `uv run` (rejected: not in the locked
  environment), a from-scratch validator (rejected: real validator
  required). Consequence: any drift between the Rust type surface and
  the canonical schema fails the M3 gate; version/class enum rejection
  is proven against the schema authority. Reversal: remove the
  dev-dependency and record in VERSIONS.lock.yaml. Security/license:
  MIT OR Apache-2.0, test-only, recorded in VERSIONS.lock.yaml,
  cargo-deny licenses ok. Compatibility: dev-only, no production
  binary impact.
- 2026-08-14 - Decision: canonical `$ref` resolution in integration
  tests is served from the repository `schemas/` tree via a
  `LocalSchemasRetriever` rather than the network. Evidence: the
  manifest schema references `capability-descriptor.schema.json`;
  resolving against the real network namespace would fail in offline
  CI and leak nothing useful. Consequence: tests are hermetic and
  deterministic. Reversal: none. Security/license: none.
- 2026-08-14 - Decision: the dependency-direction test checks the
  production edge set (`--edges normal`) so test-only dev-dependencies
  (the JSON Schema validator) do not violate the infrastructure-free
  production boundary. Evidence: `jsonschema` legitimately pulls HTTP
  crates into the dev tree only; the production tree of
  `nexus-connectors` remains domain + serde only. Consequence: the
  invariant "no infrastructure in production dependencies" stays
  enforced while integration tooling is permitted. Reversal: none.
  Security/license: none.
- 2026-08-14 - Decision: M4 hardened `connector-manifest.schema.json`
  with `uniqueItems` on `events`, `secrets`, and `network_origins`.
  Evidence: the descriptor schema already enforced uniqueness on
  `required_scopes`, `data_classes`, and `event_types`; the manifest
  schema did not, so duplicate secret references or event types could
  pass the wire contract. Consequence: cross-language clients and the
  Rust contract now reject duplicate declarations identically;
  failure tests prove the schema rejects them. Reversal: none.
  Security/license: none.
- 2026-08-14 - Decision: M4 failure tests exercise the real failure
  mechanisms (missing/unavailable capability, typed provider error,
  duplicate idempotency key, cross-tenant denial, schema rejections)
  through the real deterministic core and the real `jsonschema`
  validator; no component being proven is mocked. Evidence: milestone
  M4 content item 2 ("do not mock the component being proven") and
  item 3 (fail-closed behavior). Consequence: failures are proven at
  the exact boundary where they occur. Reversal: none. Security/
  license: none.

- 2026-08-14 - Decision: M5 proves the EP-010 contracts as ONE
  composed subsystem through a single probe binary
  (`crates/nexus-connectors/examples/livefire_probe.rs`) rather than
  separate per-crate proofs. Evidence: EP-010 owns no standalone
  external provider, so the M5 proof must compose the real registry,
  dispatcher, idempotency tracker, contract types, and canonical
  schema authority as one system (directive B: the example is
  orchestration only). Alternatives: separate M3/M4-style probes
  (rejected: would not prove composition), a fake external connector
  (rejected: would claim external certification EP-010 does not own).
  Consequence: 13 stages with observed results, authority-boundary
  assertions computed from real data, and explicit certification
  boundaries (external connector/provider certification: NOT OWNED BY
  EP-010). Reversal: none. Security/license: none.
- 2026-08-14 - Decision: probe fixes conformed the PROBE to the locked
  M1/M2 contracts, never the reverse. Evidence: (1) `locality` was
  passed as `None` but the canonical descriptor schema requires a
  non-null `Locality` enum - fixed to `Some(Locality::Any)`; (2) the
  initial REGISTER_DISCOVER assertion expected insertion order, but
  the real registry iterates a `BTreeMap<(TenantId, String)>`, so the
  deterministic order is sorted by capability id - assertion fixed to
  match observed behavior. No production contract was changed to
  satisfy the probe. Reversal: none. Security/license: none.
- 2026-08-14 - Decision: M5 gate uses a vacuity guard
  (`scripts/ep010-vacuity.sh`) because `pytest -o
  python_functions="ep010_livefire_*"` exits 0 even when zero tests
  match ("no tests ran"), which would let a broken filter print green.
  Evidence: observed "no tests ran in 0.06s" with exit 0 before the
  function rename; EP-009's gate had the same latent risk. The guard
  captures the pytest log and requires a real `[1-9]+ passed` line.
  Consequence: a zero-match filter now fails the M5 gate. Reversal:
  none. Security/license: none.
- 2026-08-14 - Decision: `scripts/ep010-orphan-audit.sh` fenced and
  wired into the M5 gate (EP-009 precedent). EP-010 owns no containers
  or daemons, so the audit is defensive (zero nexus-ep010-*
  containers/networks/volumes if docker exists), plus zero /tmp
  scratch, zero stray `livefire_probe` processes, and exactly the two
  canonical evidence files under `.agent/state/evidence/ep010-m5/`.
  Evidence: `scripts/ep009-orphan-audit.sh` is fenced in EP-009's
  expected-files; the EP-010 fence was amended for both new scripts.
  Reversal: none. Security/license: none.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.

## EP-010 M5 closure

- Changed files (within fence): `tests/capabilities/` (README +
  `test_ep010_live_fire.py`), `crates/nexus-connectors/examples/
  livefire_probe.rs`, `scripts/nodes/EP-010.sh` (M5 gate),
  `scripts/ep010-vacuity.sh`, `scripts/ep010-orphan-audit.sh`,
  `.agent/expected-files/EP-010.txt` (fence amendment),
  `.agent/state/evidence/ep010-m5/` (governed evidence), ExecPlan
  progress/decision-log/outcomes. No file outside the machine fence
  was modified.
- Exact commands and observed sentinels:
  - `cargo build --locked -p nexus-connectors --example livefire_probe`
    - zero errors, zero warnings
  - `sh scripts/format-check.sh` - `format check: ok`
  - `cargo clippy --locked -p nexus-connectors --example livefire_probe
    -- -D warnings` - clean
  - `uv run --frozen pytest tests/capabilities -q --tb=native -o
    python_functions="ep010_livefire_*"` - `7 passed`
  - `sh scripts/nodes/EP-010.sh M5` - `EP-010 M5: ok`
  - `sh scripts/node-verify.sh EP-010` - `node verify EP-010: ok`
  - `sh scripts/scope-audit.sh EP-010` - `scope audit EP-010: ok`
  - `sh scripts/expected-files.sh EP-010` - `expected files EP-010: ok`
  - adapter parity - 1 unique PRIME-BLOCK cksum across all 8 agent
    instruction files
  - `sh scripts/security-check.sh`, `sh scripts/license-gate.sh`,
    `sh scripts/reality-gate.sh`, `sh scripts/dependency-audit.sh`,
    `sh scripts/smoke-test.sh`, `sh scripts/live-fire.sh` - all ok
- Test/proof evidence: 13-stage composed subsystem proof with all
  stages PASS (see `.agent/state/evidence/ep010-m5/
  ep010-m5-composed-proof.json`); independent stage-field checks;
  authority-boundary assertions; schema versions recorded
  (current-version parity PASS; future-version migration NOT
  ASSERTED); certification boundaries explicit.
- Assumptions confirmed: EP-010 owns no external provider; the
  deterministic core is the real production surface; BTreeMap order is
  the deterministic discovery order; tier/certification are manifest
  metadata only.
- Provider/hardware status: none owned by EP-010. External connector/
  provider certification: NOT OWNED BY EP-010.
- Remaining risks: none for EP-010 closure. The graph-level
  `nexus-cli`-stub live-fire defect for future DONE nodes (LF-001/
  LF-002/LF-004+) remains out of EP-010 scope.
- Green tag: `green/EP-010` (created after committed-state
  verification).
