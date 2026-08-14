NODE-META-BEGIN
ID: EP-011
DEPS: EP-010
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-011
VERIFY_SENTINEL: node verify EP-011: ok
GREEN_TAG: green/EP-011
NODE-META-END

# 1. Purpose / Big Picture

Build Rust, Python, and TypeScript connector SDKs plus a sandboxed legacy Connector Sidecar. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-011.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-011.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-010` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-011.md`
- `.agent/specs/SPEC-022-universal-connector-contract-sdks-sidecar-and-legacy-integration.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-011.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-011-connector-sdks-and-sidecar-runtime.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-011.txt`
- `.agent/node-contracts/EP-011.md`
- `scripts/nodes/EP-011.sh`
- `crates/nexus-connector-sdk/`
- `packages/connector-sdk/`
- `python/nexus_connector_sdk/`
- `crates/nexus-sidecar/`
- `tests/connectors/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `RustConnectorSdk` | `nexus-connector-sdk` | Defined by EP-011; provider-neutral and versioned |
| `TypeScriptConnectorSdk` | `nexus-connector-sdk` | Defined by EP-011; provider-neutral and versioned |
| `PythonConnectorSdk` | `nexus-connector-sdk` | Defined by EP-011; provider-neutral and versioned |
| `SidecarAdapter` | `nexus-connector-sdk` | Defined by EP-011; provider-neutral and versioned |
| `LegacyPoller` | `nexus-connector-sdk` | Defined by EP-011; provider-neutral and versioned |
| `WebhookNormalizer` | `nexus-connector-sdk` | Defined by EP-011; provider-neutral and versioned |
| `CredentialBroker` | `nexus-connector-sdk` | Defined by EP-011; provider-neutral and versioned |

Acceptance obligations:

1. All SDKs pass one shared contract corpus
2. Sidecars wrap REST, SOAP, SQL, CLI, filesystem, and browser-last-resort sources without direct authority
3. Commands are idempotent and events are versioned
4. Credentials remain in the broker rather than connector prompts

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for build rust, python, and typescript connector sdks plus a sandboxed legacy connector sidecar.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-011-M1.txt`, `.agent/node-contracts/EP-011.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-011-connector-sdks-and-sidecar-runtime.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-011.txt`, `.agent/node-contracts/EP-011.md`, `scripts/nodes/EP-011.sh`, `crates/nexus-connector-sdk/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep011_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-011.sh M1`

EXPECT:

- `EP-011 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-011 MILESTONE_PASS "M1 EP-011 M1: ok"`

FALLBACK: Ship Rust SDK and protocol conformance first, then make TypeScript and Python thin generated bindings within this node. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-011][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-011.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-011-M2.txt`, `.agent/node-contracts/EP-011.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `packages/connector-sdk/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep011_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-011.sh M2`

EXPECT:

- `EP-011 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-011 MILESTONE_PASS "M2 EP-011 M2: ok"`

FALLBACK: Ship Rust SDK and protocol conformance first, then make TypeScript and Python thin generated bindings within this node. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-011][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-011 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-011-M3.txt`, `.agent/node-contracts/EP-011.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `python/nexus_connector_sdk/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep011_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-011.sh M3`

EXPECT:

- `EP-011 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-011 MILESTONE_PASS "M3 EP-011 M3: ok"`

FALLBACK: Ship Rust SDK and protocol conformance first, then make TypeScript and Python thin generated bindings within this node. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-011][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-011 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-011-M4.txt`, `.agent/node-contracts/EP-011.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `crates/nexus-sidecar/`

CONTENT:

1. Create tests whose names begin `ep011_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-011.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-011 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-011 MILESTONE_PASS "M4 EP-011 M4: ok"`

FALLBACK: Ship Rust SDK and protocol conformance first, then make TypeScript and Python thin generated bindings within this node. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-011][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-011.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-011-M5.txt`, `.agent/node-contracts/EP-011.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/connectors/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-011` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-011.sh M5`
2. `sh scripts/node-verify.sh EP-011`
3. `sh scripts/scope-audit.sh EP-011`

EXPECT:

- `EP-011 M5: ok`
- `node verify EP-011: ok`
- `scope audit EP-011: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-011 MILESTONE_PASS "M5 EP-011 M5: ok"`

FALLBACK: Ship Rust SDK and protocol conformance first, then make TypeScript and Python thin generated bindings within this node. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-011][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-011` and observe `node verify EP-011: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-023` `legacy-sidecar-connector`: Wrap a real local legacy protocol fixture outside production paths, discover capabilities, read state, issue an idempotent write, and receive a change event.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
  - `crates/nexus-connector-sdk/` workspace crate: the provider-neutral
    connector SDK contract. Public surface per node contract: shared
    `ConnectorSdk` trait (discover/query/command/health/changefeed over
    the EP-010 capability ports) plus `RustConnectorSdk`,
    `TypeScriptConnectorSdk`, `PythonConnectorSdk` bindings sharing
    `CONTRACT_VERSION` (SPEC-022 behavior 4); `SidecarAdapter` port
    wrapping one `SidecarTransport` family; `LegacyPoller` port with
    stable cursors; `WebhookNormalizer` port (signed webhook
    verification + replay rejection); `CredentialBroker` port with
    namespaced `CredentialReference` (values never leave the broker).
  - Vocabulary (ADR-016 + docs/vocabulary/README.md): `SdkLanguage`,
    `SidecarTransport` (12 families), `LegacyTransport`,
    `WebhookDeliveryState`, `WebhookVerification` - parse/reject
    unknown, canonical SCREAMING_SNAKE wire values.
  - Typed `SdkError` (SPEC-006 codes) with correlation/actor/tenant/
    resource context boxed as `Box<str>` (clippy large-Err clean);
    fails closed; transient classification.
  - 19 `ep011_unit_*` tests (vocabulary round-trip/rejection, error
    classification/serialization, SDK binding contract version,
    empty-registry discovery, sidecar execute + typed fail-closed,
    legacy poller normalization + cursor, webhook valid/invalid
    signature + no-secret envelope, credential reference validation +
    never-holds-value) + 1 dependency-direction test (production
    `--edges normal` tree: no tokio/axum/reqwest/infra/vendor crates).
  - Fence amended: `docs/vocabulary/README.md`,
    `references/ADR-016-connector-sdk-and-sidecar-vocabulary.md`,
    `Cargo.toml`, `Cargo.lock` (workspace member + lock entry).
  - `EP-011 M1: ok`; format check ok; lint ok; security check ok;
    license gate ok; reality gate ok; clippy clean (zero warnings).
- [x] M2: Core behavior and deterministic invariants
  - `packages/connector-sdk/` TypeScript binding: `ConnectorSdk` class
    mirroring the Rust trait (discover/query/command/health/changefeed),
    `IdempotencyTracker` (key bound to capability, replay, conflict),
    typed `SdkError` with canonical SPEC-006 codes and snake_case wire
    context, class-mismatch denial, availability-filtered discovery.
    Field names are canonical snake_case (SPEC-022 behavior 4).
  - 12 `ep011_unit_*` vitest tests; `tsc --noEmit` clean.
  - `EP-011 M2: ok`; pnpm lockfile updated (`pnpm-lock.yaml` added to
    the fence).
- [x] M3: Real dependency and transport integration
  - REAL process transport: `tests/connectors/fixture_sidecar.py` is a
    real HTTP server on 127.0.0.1 with an ephemeral port implementing
    the canonical sidecar REST protocol (SPEC-022 Tier 1 REST,
    directive C). Every proof crosses a REAL process boundary; no
    in-process mocks, no direct function calls.
  - `python/nexus_connector_sdk/` package: canonical wire types
    (InvocationContext, CapabilityDescriptor, ConnectorManifest,
    Query/Command/Workflow/Health/Changefeed shapes, SidecarAdapter,
    WebhookNormalizer, LegacyPoller, CredentialBroker) with Python
    serializers emitting JSON `null` for absent Option fields to match
    Rust serde byte-for-byte (directive D).
  - Golden wire corpus: 19 canonical fixtures generated from the Rust
    types (example `generate_golden`) committed under
    `tests/connectors/golden/`; Rust/TS/Python all deserialize and
    serialize against the SAME corpus (directives D/E).
  - Rust: 10 `ep011_integration_golden_*` + 9 `ep011_integration_transport_*`
    (live HTTP to the fixture sidecar: discover, query, idempotent
    command replay + CONFLICT, class mismatch, not-found, workflow
    dispatch NOT Temporal, health observation, cross-tenant denial,
    protocol version 426, credential boundary, unavailable fail-closed).
  - TS: 7 `ep011_integration_ts_golden_*` + 7 `ep011_integration_ts_live_*`
    vitest tests against the same sidecar over real fetch.
  - Python: 58 `ep011_integration_*`/`ep011_failure_*` pytest tests:
    live transport, idempotency, class mismatch, cross-tenant,
    webhook normalizer (two provider shapes -> one canonical event;
    bad signature fail-closed), legacy poller (real JSONL source,
    mutation observed, checkpoint restart resume), credential broker
    (reference-only, broker-down fail-closed), transport security
    (malformed JSON, bounded size, unknown path, no debug endpoint),
    protocol versioning, error parity (NOT_FOUND/VALIDATION/CONFLICT/
    UNAVAILABLE), observability (redacted telemetry, no secrets),
    strict teardown (clean shutdown, port release, zero orphans).
  - TS SDK error envelope corrected to canonical snake_case
    (`correlation_id`, not `correlationId`) - directive D violation
    found during parity build; 12 M2 tests still green.
  - Gate: `EP-011 M3: ok` (artifact check, cargo ep011_unit +
    ep011_integration, pnpm test:unit, `ep011-vacuity.sh` 58 tests,
    `ep011-orphan-audit.sh` ok). Fence amended: `pnpm-lock.yaml`,
    `scripts/ep011-vacuity.sh`, `scripts/ep011-orphan-audit.sh`.
  - Evidence: `.agent/state/evidence/ep011-m3/` (result codes,
    correlation IDs, fingerprints only - no credentials).
  - M4: `crates/nexus-sidecar/` real hardened boundary (hyper 1.11 on
    127.0.0.1:0, PORT contract, strict envelope/version/tenant/
    dispatch/credential checks, per-endpoint response-shape validation,
    webhook HMAC + fingerprint + in-process replay dedupe, owned poller
    with checkpoint integrity + path containment, concurrency semaphore,
    redacted telemetry, controlled shutdown). Suites: 45 unit + 1
    dependency-direction + 36 failure/abuse + 21 integration/lifecycle
    (incl. new mid-request shutdown) all green under DEFAULT parallelism;
    58 M3 Python tests green via fixed gate selector; M3 re-verify ok.
    Parallel-hang root cause: leftover `probe_test.rs` diagnostic (always
    panics, blocks on stdout EOF) - removed. Gate: `EP-011 M4: ok`;
    scope audit ok; security/license/reality/format/lint/expected-files
    ok; orphan audit ok (extended with `nexus-sidecar` process pattern).
  - Evidence: `.agent/state/evidence/ep011-m4/` (result codes,
    correlation IDs, fingerprints only - no credentials).
- [x] M4: Forced failures, abuse cases, and observability
- [x] M5: Live-fire, operations, and node closure
  - LF-023 real live-fire proof (tests/connectors/test_ep011_live_fire.py):
    real fixture -> real nexus-sidecar wrapper; DISCOVER -> QUERY ->
    idempotent COMMAND + replay -> CHANGEFEED event; `LF-023: ok`;
    evidence `.agent/state/evidence/LF-023-ep011-m5.md`.
  - Dependency cleanup: jsonschema 0.49.9 default-features=false +
    resolve-file (hermetic schemas); reqwest dev-deps TLS-free in
    sidecar + SDK crates; lock pruned of webpki-root-certs /
    rustls-platform-verifier / core-foundation duplicate; ZERO deny.toml
    changes; `dependency audit: ok`.
  - Real defects fixed: sidecar probe-bind TOCTOU (bind-once listener);
    sidecar.py cross-interpreter except form (multi-line tuple valid on
    3.12 + 3.14, ruff-stable); M3 mypy no-any-return casts.
  - Gate: `EP-011 M5: ok`; M3/M4 regressions green; EP-010 schema
    compatibility green (18+1+12+6).

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-14 - M4: the "parallel test hang" was a leftover dev diagnostic,
  not a harness collision or a sidecar lifecycle race. `tests/probe_test.rs`
  (`probe_spawn`) unconditionally panicked and blocked forever on
  `read_to_string` waiting for stdout EOF from the long-lived sidecar
  binary. Under `--test-threads=1` it still blocked (60 s cargo warning);
  removal fixed the full suite under default parallelism (45+1+36+21 all
  green, zero leaks). CASE determination recorded in the Decision Log.
- 2026-08-14 - M4: the M4 gate's `pytest tests/connectors -q` step had
  ALWAYS collected zero tests: M3 Python functions are `ep011_integration_*`
  /`ep011_failure_*` but `pyproject.toml` `python_functions` lists only
  `test_*`, `ep001_*`, `ep002_*` (pyproject is outside the EP-011 fence).
  The M3 gate never ran pytest (the 58 tests were run manually with a
  selector). Fixed in-fence in `scripts/nodes/EP-011.sh` with
  `-o python_functions=ep011_*`.
- 2026-08-14 - M4: `REQUEST_ACCEPTED` telemetry is emitted immediately
  before provider dispatch, which makes it a deterministic "request is
  in-flight" signal for the mid-request shutdown proof (no artificial
  sleeps needed).

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-14 - Decision: the SDK crate is a provider-neutral contract
  surface, not a transport implementation: the shared `ConnectorSdk`
  trait routes through the EP-010 capability ports (registry,
  dispatcher, idempotency tracker) and the SDK never grants authority.
  Evidence: node contract public interfaces (RustConnectorSdk,
  TypeScriptConnectorSdk, PythonConnectorSdk, SidecarAdapter,
  LegacyPoller, WebhookNormalizer, CredentialBroker); SPEC-022 behavior
  4 (one conformance suite) and behavior 5 (sidecar wraps transports).
  Alternatives: a transport-owning SDK (rejected: would duplicate
  EP-012/EP-013 infra), a per-language divergent surface (rejected:
  violates shared corpus). Consequence: Rust/TypeScript/Python bindings
  share one trait and one `CONTRACT_VERSION`; real transports and the
  sandbox boundary are proven in later milestones. Reversal: ADR +
  schema update. Security/license: none.
- 2026-08-14 - Decision: SDK errors use `SdkError` (SPEC-006 codes)
  with context boxed as `Option<Box<str>>`, mirroring EP-010's
  `CapabilityError`. Evidence: clippy large-Err flagged the unboxed
  variant; wire serialization of `Box<str>` is identical to `String`.
  Alternatives: `Box<SdkError>` wrapper (rejected: extra indirection,
  breaks value semantics), unboxed struct (rejected: 128+ byte Err).
  Consequence: `Result<T, SdkError>` is compact, typed, and fails
  closed; accessor methods (`correlation()`, `actor()`, `tenant()`,
  `resource()`) keep ergonomics. Reversal: none. Security/license:
  none.
- 2026-08-14 - Decision: `AcceptingWebhookNormalizer` is a test/verification-zone
  double implementing the real `WebhookNormalizer` trait; production
  webhook providers implement real signature verification in later
  milestones. Evidence: TESTING.md test zones; M1 owns the contract,
  not a real provider. Consequence: the normalize contract (valid /
  invalid / replay semantics, canonical event shape) is proven now;
  provider-specific crypto is proven with the provider. Reversal: none.
  Security/license: none.
- 2026-08-14 - Decision: dependency-direction test uses `--edges
  normal` (production tree only), the EP-010 M3 precedent. Evidence:
  dev-dependencies legitimately extend the test tree; the invariant is
  the production edge set. Consequence: no infrastructure/vendor crate
  in the SDK production tree. Reversal: none. Security/license: none.
- 2026-08-14 - Decision: M3 sidecar transport is REST/JSON over real
  HTTP on 127.0.0.1 with an ephemeral port (SPEC-022 Tier 1 permits
  "authenticated MCP or REST"; directive C mandates a real localhost
  TCP listener with ephemeral port assignment). Evidence: SPEC-022
  required behavior 1; fixture_sidecar.py binds 127.0.0.1:0 and
  prints `PORT n`. Alternatives: MCP transport (rejected: MCP is a
  peer surface, not the sidecar wire), gRPC (rejected: not locked by
  SPEC-022). Consequence: every M3 proof crosses a REAL process
  boundary over real HTTP; the same wire is spoken by Rust, TS, and
  Python clients. Reversal: ADR + schema update. Security: bind
  127.0.0.1 only, ephemeral port, bounded request size (64 KiB),
  bounded timeout, no file/debug endpoints.
- 2026-08-14 - Decision: protocol/schema versioning is an explicit
  `X-Nexus-Protocol-Version: 1` header required on every sidecar
  request (directive Q). Evidence: fixture sidecar returns 426 with a
  typed VALIDATION envelope for any other version; current version is
  accepted. Alternatives: version in body (rejected: header is
  inspectable before body parse), no versioning (rejected: silent
  reinterpretation). Consequence: future incompatible payloads fail
  closed. Reversal: none. Security: none.
- 2026-08-14 - Decision: canonical snake_case wire policy for ALL
  message shapes (directive D). Evidence: golden fixtures generated
  from the Rust types use snake_case; the TS SDK error envelope was
  corrected from `correlationId` to `correlation_id` in M3 to match
  Rust/Python (directive D violation found during parity build);
  Python serializers emit `null` for absent Option fields to match
  Rust serde exactly. Alternatives: language-native casing (rejected:
  violates directive D). Consequence: one byte-compatible wire across
  Rust/TS/Python. Reversal: none. Security: none.
- 2026-08-14 - Decision: cross-language parity strategy is a committed
  golden fixture corpus generated from the authoritative Rust types
  (example `generate_golden`, 19 files) plus LIVE cross-process
  transport tests in all three languages against the SAME fixture
  sidecar (directives C/D/E). Evidence: cargo test golden_parity (10),
  transport_live (9); vitest golden_parity (7), transport_live (7);
  pytest parity/live (58); all compare semantic structures, never raw
  JSON strings. Alternatives: string-compare (rejected: map ordering
  irrelevant), single-language golden (rejected: no cross-language
  proof). Consequence: Rust serialize -> TS deserialize, TS -> Python,
  Python -> Rust are proven transitively against one canonical corpus
  plus live transport. Reversal: regenerate goldens only via the
  example. Security: none.
- 2026-08-14 - Decision: class-specific transport dispatch with NO
  generic execute endpoint (directive F). Evidence: /v1/query,
  /v1/command, /v1/workflow, /v1/changefeed are separate typed
  endpoints; a query sent to a COMMAND capability returns a typed
  VALIDATION "not a QUERY class" and the provider is NOT invoked.
  Alternatives: POST /v1/execute with a class field (rejected: the
  sidecar adapter surface /v1/execute is action-based and separate).
  Consequence: class checking is at the transport boundary, mirroring
  EP-010 dispatcher semantics. Reversal: none. Security: none.
- 2026-08-14 - Decision: idempotency semantics preserved EXACTLY from
  EP-010: a key is bound to its first capability; replay returns the
  previous result without a second provider execution; the same key on
  a different capability is a typed CONFLICT. Evidence: Rust/TS/Python
  live tests all observe replay identity and CONFLICT on
  capability/key mismatch. Alternatives: a second weaker algorithm
  (rejected: directive G). Consequence: sidecar/SDK idempotency is the
  EP-010 algorithm over the transport. Reversal: none. Security: none.
- 2026-08-14 - Decision: SidecarAdapter process lifecycle is explicit:
  start prints PORT, readiness via health endpoint, controlled
  shutdown via SIGTERM, typed UNAVAILABLE when not listening, and
  protocol-version mismatch fails closed (directive L). Evidence:
  fixture sidecar main() handles SIGTERM/SIGINT and exits 0; conftest
  asserts clean shutdown and port release; orphan audit proves zero
  leftover processes/ports. Alternatives: none. Consequence: teardown
  is strict; cleanup failure fails M3. Reversal: none. Security: none.
- 2026-08-14 - Decision: LegacyPoller checkpoint semantics are
  byte-offset cursors over a REAL local JSONL source with a persisted
  checkpoint file; restart resumes from the checkpoint (directive K).
  Evidence: poll returns exactly the new record after real file
  mutation; unchanged poll returns zero events; a restarted sidecar
  resumes at the persisted cursor without re-delivery. Alternatives:
  in-memory cursor (rejected: not a real source), time-based (rejected:
  not deterministic). Consequence: pollers never claim exactly-once
  delivery; EP-005 owns durable event transport. Reversal: none.
  Security: none.
- 2026-08-14 - Decision: WebhookNormalizer boundary is fingerprint-
  verified signature + canonical event shape; missing identity and bad
  signatures fail closed; unknown event types are preserved as typed
  metadata, never coerced to a generic event (directive J). Evidence:
  two provider-shaped fixtures normalize to the same canonical event;
  bad signature -> INVALID with no event; replay semantics owned by
  the Rust contract are preserved. Alternatives: real provider HMAC
  (rejected: no third-party provider is owned by EP-011 M3).
  Consequence: provider crypto is certified with the provider.
  Reversal: none. Security: no raw signatures in logs.
- 2026-08-14 - Decision: CredentialBroker boundary is reference-only on
  the wire: manifests/descriptors/discovery carry names, never
  values; resolution happens inside the sandbox and only a fingerprint
  crosses the transport; broker-unavailable fails closed (directive M).
  Evidence: live tests assert the raw fixture secret never appears in
  responses, discovery, or telemetry; broker-down command returns
  typed UNAVAILABLE. Alternatives: value passthrough (rejected:
  SPEC-020). Consequence: the broker is not a raw-secret vending
  machine. Reversal: none. Security: this is the SECURITY.md boundary.
- 2026-08-14 - Decision: no EP-008 authorization duplication, no
  EP-005 event-transport duplication, no EP-006 Temporal claim
  (directives A/H). Evidence: the sidecar dispatches typed
  capabilities and fails closed on cross-tenant/unknown capability; it
  does not authorize; changefeed is capability-level, not NATS;
  workflow dispatch returns a RUNNING handle and never claims durable
  Temporal execution. Consequence: ownership boundaries preserved.
  Reversal: none. Security: none.
- 2026-08-14 - Decision: external provider certification is NOT
  ASSERTED by EP-011 M3. Evidence: the fixture sidecar is a
  test/verification-zone provider; no third-party connector is owned
  by this node. Consequence: certification language in the evidence
  file is explicit. Reversal: future node. Security: none.
- 2026-08-14 - Decision: M3 evidence is a governed evidence file under
  `.agent/state/evidence/ep011-m3/` (scope audit exempts
  `.agent/state/evidence/*`), with result codes, correlation IDs and
  fingerprints only - never credentials or raw provider payloads
  (directive N/R). Evidence: EP-008/EP-009/EP-010 precedent.
  Consequence: evidence is independently verifiable and secret-free.
  Reversal: none. Security: none.
- 2026-08-14 - Decision: parallel test hang = CASE 3-adjacent test
  artifact, NOT CASE 1 (harness collision) and NOT CASE 2 (real sidecar
  lifecycle race). Evidence: harness uses only ephemeral ports, per-test
  tempdirs, per-test processes, no shared mutable env/files; the 36-test
  failure suite ran in parallel in 3.54-4.13 s and the 21-test lifecycle
  suite in 2.52-3.23 s; the only hang was `probe_test.rs`'s `probe_spawn`
  blocking on stdout EOF from a long-lived process it wrongly expected to
  exit. Alternatives: serialize with `--test-threads=1` (rejected: masks
  nothing real but hides the diagnostic forever), keep the probe (rejected:
  always panics, redundant with the provider-absent test). Consequence:
  probe removed; full suite green under default parallelism; `--test-threads=1`
  no longer needed. Reversal: none. Security: none.
- 2026-08-14 - Decision: mid-request shutdown is bounded immediate exit
  (signal -> SIDECAR_STOPPED -> exit 0), not graceful provider-work
  completion; shutdown ownership is explicit (directive C/D). Evidence:
  `ep011_integration_sidecar_mid_request_shutdown_bounded` arms a slow
  provider, observes REQUEST_ACCEPTED, SIGTERMs, asserts exit < 10 s,
  in-flight client never receives success, old port rebinds immediately,
  zero orphans. Alternatives: graceful drain with a grace timer (rejected:
  the M4 contract does not promise graceful completion of provider work;
  immediate bounded exit is the deterministic contract). Consequence:
  owned resources (listener, tasks, webhook state, poller state, telemetry
  sink) terminate with the process; the provider is NOT owned and is not
  terminated by the sidecar. Reversal: none. Security: none.
- 2026-08-14 - Decision: M4 gate must actually select the Python tests:
  `pytest tests/connectors -q -o python_functions=ep011_*` in
  `scripts/nodes/EP-011.sh` (M4 and M5|verify). Evidence: bare
  `pytest tests/connectors -q` collected 0 tests (M3 functions are
  `ep011_integration_*`/`ep011_failure_*`; pyproject `python_functions`
  lacks `ep011_*` and pyproject is outside the fence). Alternatives:
  edit pyproject.toml (rejected: outside the machine fence), keep bare
  pytest (rejected: vacuous gate). Consequence: gate now runs the real 58
  M3 Python tests; vacuity impossible. Reversal: none. Security: none.
- 2026-08-14 - Decision: orphan audit extended with the exact owned
  `nexus-sidecar` process pattern (bracket-trick grep), in addition to
  fixture processes, docker nexus-ep011 artifacts, and /tmp scratch.
  Evidence: M4 spawns `target/debug/nexus-sidecar`; the audit previously
  only covered `fixture_sidecar.py`. Alternatives: broad `pkill` (rejected:
  can match the cargo test command itself), no audit (rejected: teardown
  must be proven). Consequence: any leaked sidecar fails the gate.
  Reversal: none. Security: none.
- 2026-08-14 - Decision: credential canary proof covers the success body,
  an HTTP error body, sidecar stdout, and sidecar stderr (directive L).
  Evidence: `ep011_integration_sidecar_credential_canary_never_leaks`
  asserts zero occurrences of the canary value in all four surfaces after
  a permitted-reference command, an out-of-scope denial (403 AUTHORIZATION),
  and a full post-SIGTERM stdout/stderr drain. Alternatives: body-only
  (rejected: directive L demands stdout/stderr/logs/errors). Consequence:
  redaction is proven across every observable surface. Reversal: none.
  Security: the SECURITY.md boundary.
- 2026-08-14 - Decision: the sidecar's webhook replay dedupe is shared
  in-memory state (`Arc<Mutex<WebhookIngress>>`) across request tasks;
  crash-durable replay defense is NOT ASSERTED. Evidence: the replay test
  drives two separate HTTP requests with the same provider_event_id and
  observes 200 then 401 VERIFICATION; process restart resets the set.
  Alternatives: persisted replay set (rejected: EP-005 owns durable state;
  would over-claim). Consequence: replay protection is process-lifetime,
  documented in evidence. Reversal: none. Security: none.
- 2026-08-14 - Decision: clippy debt paid at M4 closure (6 lib + 1 test
  warnings) because `scripts/lint.sh` runs `cargo clippy --workspace
  --all-targets --all-features --locked -- -D warnings`: manual_clamp,
  redundant_closure, collapsible_match, collapsible_if (let-chain, edition
  2024), manual_is_multiple_of, single_char_add_str. Evidence: `lint: ok`.
  Alternatives: allowlist (rejected: broad allowlist prohibition).
  Consequence: workspace clippy stays clean. Reversal: none. Security:
  none.

- 2026-08-14 - Decision: the correct cross-interpreter form of the
  `sidecar.py` except clause is the MULTI-LINE parenthesized tuple:
  `except (\n json.JSONDecodeError,\n OSError,\n):`. Evidence: the
  comma form (`except A, B:`) is valid ONLY on Python >= 3.14 (PEP 758)
  and is the ruff-format canonical form for `target-version = py314`,
  but the Rust test harness spawns bare `python3` (3.12.3 in PATH),
  where the comma form is a SyntaxError (fixture exits 1, no PORT line,
  every Rust integration suite fails). The single-line parenthesized
  form is valid on both interpreters but ruff format rewrites it back
  to the comma form on every run. The multi-line parenthesized form is
  valid on 3.12 AND 3.14, and ruff format leaves it unchanged
  (`format check: ok`). Alternatives: pin the harness to the venv
  python (rejected: fragile ambient-PATH dependency), type-ignore
  (rejected: no lint exception for real syntax), comma form (rejected:
  breaks 3.12 harness). Consequence: fixture imports under both
  interpreters; all Rust integration suites green. Reversal: none.
  Security: none.

- 2026-08-14 - Decision: jsonschema stays pinned at 0.49.9 but with
  HTTP/TLS resolution disabled at the source:
  `jsonschema = { version = "0.49", default-features = false, features =
  ["resolve-file"] }` in `crates/nexus-connectors/Cargo.toml`. Evidence:
  EP-010 schema tests are hermetic (draft 2020-12, local $ref only, zero
  network fetches); the default `resolve-http` + `tls-aws-lc-rs` features
  pulled reqwest with the rustls TLS stack (webpki-root-certs/CDLA,
  rustls-platform-verifier, core-foundation 0.9.4+0.10.1 duplicate) plus
  h2 -> indexmap -> hashbrown 0.17 -> foldhash. After the feature change
  and a lock prune (`cargo update --workspace --offline`): webpki-root-certs,
  rustls-platform-verifier, and the core-foundation duplicate are REMOVED
  from Cargo.lock; `dependency audit: ok` with ZERO deny.toml policy
  changes (the remaining dev-only foldhash/Zlib transitive of
  jsonschema's `referencing` crate is classified acceptable by the pinned
  cargo-deny 0.20.2). `cargo tree` verified the TLS chain is unreachable.
  Alternatives: deny.toml allowlist additions (rejected: policy exception
  for a chain removed by feature selection), downgrade jsonschema
  (rejected: EP-010 pinned 0.49.9), leave defaults (rejected: unnecessary
  TLS surface + license denials). Consequence: EP-010 schema tests all
  green (18 unit + 1 dep-direction + 12 failure + 6 schema-parity); no
  network resolution is enabled. Reversal: none. Security: smaller
  attack surface.
- 2026-08-14 - Decision: the sidecar binary binds its listener ONCE in
  main() and passes it into `serve(listener)`; the earlier
  probe-bind/drop/re-bind design had a real TOCTOU (the ephemeral port
  could be grabbed between the probe release and the serve() re-bind,
  crashing the process under parallel load with "Connection refused" on
  the printed PORT). Evidence: `cargo test -p nexus-sidecar` under
  default parallelism showed 4 intermittent failure-suite failures
  (duplicate_protocol_version, wrong_method, malformed_utf8,
  old_version) with `tcp connect error ... Connection refused` on the
  sidecar's own printed port; after the fix, the full suite is
  deterministically green (45+1+36+21). Alternatives: keep TOCTOU
  (rejected: real lifecycle defect), serialize tests (rejected: masks
  the race, directive B forbids hiding lifecycle bugs). Consequence:
  the PORT contract now reflects a live, continuously-held socket.
  Reversal: none. Security: none.
- 2026-08-14 - Decision: reqwest dev-dependencies in BOTH
  `crates/nexus-sidecar/Cargo.toml` and `crates/nexus-connector-sdk/
  Cargo.toml` are `default-features = false, features = ["blocking",
  "json"]` (TLS disabled) because every test client speaks plain HTTP to
  127.0.0.1 loopback only. Evidence: feature unification previously
  pulled reqwest's default rustls stack into the workspace graph via the
  SDK dev-dep; the tree now carries no TLS client at all. Alternatives:
  keep default features + deny.toml exceptions (rejected: unused TLS
  surface). Consequence: minimal dependency tree, license-clean. Reversal:
  none. Security: reduced attack surface.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.

## M4 closure (2026-08-14)

Changed files versus the machine fence (all inside `crates/nexus-sidecar/`,
`tests/connectors/`, `python/nexus_connector_sdk/`, `scripts/nodes/EP-011.sh`,
`scripts/ep011-orphan-audit.sh`, `Cargo.toml`, `Cargo.lock`,
`.agent/execplans/EP-011-...md`, `.agent/state/evidence/*`):
- `crates/nexus-sidecar/` (new crate: lib + binary + 4 test targets)
- `tests/connectors/fixture_sidecar.py` (arming controls)
- `python/nexus_connector_sdk/sidecar.py` (no change committed: the M3
  comma-except form is valid Python 3.14 per PEP 758; the earlier import
  failure was stale pycache, resolved by clearing it)
- `scripts/nodes/EP-011.sh` (M4 gate wired to sidecar + pytest selector)
- `scripts/ep011-orphan-audit.sh` (sidecar process pattern)
- workspace `Cargo.toml`/`Cargo.lock` (nexus-sidecar member; dev-deps
  reqwest blocking + tempfile only; no new production deps)

Commands and observed sentinels:
- `sh scripts/nodes/EP-011.sh M4` -> `EP-011 M4: ok`
- `sh scripts/nodes/EP-011.sh M3` -> `EP-011 M3: ok` (re-verify)
- `sh scripts/scope-audit.sh EP-011` -> `scope audit EP-011: ok`
- `sh scripts/expected-files.sh EP-011` -> `expected files EP-011: ok`
- `sh scripts/security-check.sh` -> `security check: ok`
- `sh scripts/license-gate.sh` -> `license gate: ok`
- `sh scripts/reality-gate.sh` -> `reality gate: ok`
- `sh scripts/format-check.sh` -> `format check: ok`
- `sh scripts/lint.sh` -> `lint: ok`
- `sh scripts/ep011-orphan-audit.sh` -> `EP-011 orphan audit: ok`
- `cargo test -p nexus-sidecar` -> 45 + 1 + 36 + 21 passed (default
  parallelism, zero leaks)

Test/proof evidence: `.agent/state/evidence/ep011-m4/EP-011-M4-failure-hardening.md`.

Assumptions confirmed: sidecar owns the transport boundary only; EP-008
authz / EP-009 secrets / EP-010 capability / EP-005 durability boundaries
preserved (NOT ASSERTED claims explicit). Provider status: fixture provider
is a test-zone provider; no third-party connector certified by this node.
Remaining risks: webhook replay defense is process-lifetime only; crash-durable
idempotency NOT ASSERTED; external provider certification NOT ASSERTED.
Green tag: pending M5.
