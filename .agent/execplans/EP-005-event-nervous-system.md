NODE-META-BEGIN
ID: EP-005
DEPS: EP-004
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-005
VERIFY_SENTINEL: node verify EP-005: ok
GREEN_TAG: green/EP-005
NODE-META-END

# 1. Purpose / Big Picture

Implement NATS JetStream, canonical events, outbox, replay, correlation, and durable consumers. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-005.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-005.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-004` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-005.md`
- `.agent/specs/SPEC-023-events-outbox-temporal-workflows-scheduling-and-human-approvals.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-005.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-005-event-nervous-system.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-005.txt`
- `.agent/node-contracts/EP-005.md`
- `scripts/nodes/EP-005.sh`
- `crates/nexus-events/`
- `infra/nats/`
- `schemas/event-envelope.schema.json`
- `tests/events/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `EventPublisher` | `nexus-events` | Defined by EP-005; provider-neutral and versioned |
| `EventConsumer` | `nexus-events` | Defined by EP-005; provider-neutral and versioned |
| `OutboxRepository` | `nexus-events` | Defined by EP-005; provider-neutral and versioned |
| `InboxRepository` | `nexus-events` | Defined by EP-005; provider-neutral and versioned |
| `StreamProvisioner` | `nexus-events` | Defined by EP-005; provider-neutral and versioned |
| `EventEnvelope` | `nexus-events` | Defined by EP-005; provider-neutral and versioned |
| `ConsumerCheckpoint` | `nexus-events` | Defined by EP-005; provider-neutral and versioned |

Acceptance obligations:

1. Database mutation and outbox insert are atomic
2. JetStream publish acknowledgement precedes outbox completion
3. Consumers deduplicate and resume after restart
4. Correlation and causation survive publish, replay, and projection

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement nats jetstream, canonical events, outbox, replay, correlation, and durable consumers.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-005-M1.txt`, `.agent/node-contracts/EP-005.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-005-event-nervous-system.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-005.txt`, `.agent/node-contracts/EP-005.md`, `scripts/nodes/EP-005.sh`, `crates/nexus-events/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep005_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-005.sh M1`

EXPECT:

- `EP-005 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-005 MILESTONE_PASS "M1 EP-005 M1: ok"`

FALLBACK: Use one canonical stream and subject namespace before introducing stream sharding. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-005][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-005.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-005-M2.txt`, `.agent/node-contracts/EP-005.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/nats/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep005_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-005.sh M2`

EXPECT:

- `EP-005 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-005 MILESTONE_PASS "M2 EP-005 M2: ok"`

FALLBACK: Use one canonical stream and subject namespace before introducing stream sharding. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-005][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-005 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-005-M3.txt`, `.agent/node-contracts/EP-005.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `schemas/event-envelope.schema.json`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep005_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-005.sh M3`

EXPECT:

- `EP-005 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-005 MILESTONE_PASS "M3 EP-005 M3: ok"`

FALLBACK: Use one canonical stream and subject namespace before introducing stream sharding. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-005][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-005 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-005-M4.txt`, `.agent/node-contracts/EP-005.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/events/`

CONTENT:

1. Create tests whose names begin `ep005_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-005.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-005 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-005 MILESTONE_PASS "M4 EP-005 M4: ok"`

FALLBACK: Use one canonical stream and subject namespace before introducing stream sharding. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-005][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-005.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-005-M5.txt`, `.agent/node-contracts/EP-005.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: Only the active ExecPlan progress, Decision Log, and ledger may change in this milestone.

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-005` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-005.sh M5`
2. `sh scripts/node-verify.sh EP-005`
3. `sh scripts/scope-audit.sh EP-005`

EXPECT:

- `EP-005 M5: ok`
- `node verify EP-005: ok`
- `scope audit EP-005: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-005 MILESTONE_PASS "M5 EP-005 M5: ok"`

FALLBACK: Use one canonical stream and subject namespace before introducing stream sharding. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-005][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-005` and observe `node verify EP-005: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [x] M2: Core behavior and deterministic invariants
- [x] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

M1 completed 2026-08-12: `crates/nexus-events/` created with the canonical
event contracts (SPEC-023): `EventEnvelope` (event ID, type, schema
version, source, subject, time, tenant, actor, correlation, causation,
data class, payload; closed wire model via `deny_unknown_fields`),
`EventType` (dotted lowercase slug), `EventDataClass` (SPEC-020 privacy
ladder), `EventError`/`EventErrorCode` (SPEC-006 codes), `OutboxRecord`/
`OutboxStatus`/`OutboxRepository` (transactional outbox behind the
UnitOfWork boundary), `InboxRecord`/`InboxStatus`/`InboxRepository`
(deduplication ledger), `ConsumerCheckpoint`/`ConsumerConfig`/
`EventConsumer` (durable resumable consumers), `StreamConfig`/
`StreamProvisioner` (one canonical stream), `EventPublisher` (JetStream
ack precedes outbox completion). ADR-009 adds the event vocabulary to
`docs/vocabulary/README.md` (EventType, EventDataClass, OutboxStatus,
InboxStatus, DurableConsumer). 12 `ep005_unit_` Rust tests +
dependency-direction test pass. Workspace membership extended; Cargo.lock
regenerated offline (90 packages). Node script M1 gate fixed to capture
real exit (was swallowing failures and printing `ok` - gate-integrity
lesson from EP-001). Sentinel: `EP-005 M1: ok`.

M2 completed 2026-08-12: `infra/nats/` created as the `nexus-nats`
workspace crate implementing the nexus-events ports on NATS JetStream
(pinned nats 2.14.3, async-nats 0.47.0, tokio 1.x). `subject.rs`
derives the canonical subject namespace (`nexus.<domain>.<event>.<tenant>`,
domain/tenant wildcards, consumer subjects); `encode.rs` serializes and
validates EventEnvelope <-> JetStream payload bytes (closed wire model);
`NatsStreamProvisioner` idempotently ensures the canonical stream;
`NatsEventPublisher` blocks on the JetStream ack so outbox completion
only follows durable storage (SPEC-023 behavior 2); `NatsEventConsumer`
exposes a durable pull consumer resuming from the last checkpoint
(SPEC-023 behavior 4). The port traits are synchronous, so each adapter
owns a tokio current-thread runtime bridge. 7 `ep005_unit_` tests in the
adapter + 12 in the contracts crate + dependency-direction pass; clippy
clean. Cargo.lock regenerated offline (205 packages). Sentinel:
`EP-005 M2: ok`.

M3 completed 2026-08-12: `schemas/event-envelope.schema.json` written
(closed wire model, event_type dotted-slug pattern, data_class enum,
required set, schema_version const 1.0.0). Owner architecture
clarification corrected an EP-005 contract defect: the M1 port traits
were synchronous, forcing M2's per-adapter current-thread tokio runtime
bridges. Publisher/Consumer/Provisioner operations are now natively
`async fn`; the adapter never owns a runtime; M3 integration tests run
on `#[tokio::test(flavor = "multi_thread")]` against a real
`nats:2.14.3` container with dynamically allocated host ports. Six
`ep005_integration_` tests prove stream provisioning idempotency,
JetStream publish-ack precedence, durable consumption with explicit
server-observed acks (`num_ack_pending` 3 -> 0), full envelope
round-trip equality, checkpoint resume semantics, and clean shutdown
with zero orphaned containers. Sentinel: `EP-005 M3: ok`.

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-12 (M1): **Event contracts live in `nexus-events`, provider
  neutral.** The crate owns the seven public interfaces from the node
  contract. NATS JetStream implements the ports in `infra/nats` (M2+);
  the contract crate imports `nexus-domain` (typed IDs) and `nexus-data`
  (UnitOfWork boundary) only. Evidence: dependency-direction test forbids
  tokio/nats/postgres/etc. in the resolved tree.
- 2026-08-12 (M1): **Event vocabulary added by ADR-009.** EventType
  (dotted slug), EventDataClass (SPEC-020 privacy ladder), OutboxStatus,
  InboxStatus, DurableConsumer are vocabulary-locked contracts owned by
  `nexus-events`. Workflow/Activity/Signal/Query/Schedule/
  ApprovalWorkflow/Compensation are Temporal-owned and deferred to the
  workflow node. Evidence: ADR-009, vocabulary README, unit tests.
- 2026-08-12 (M1): **`EventEnvelope` wire model is closed.** Serde
  `deny_unknown_fields` enforces additionalProperties: false; unknown
  fields are rejected at parse time, matching the M3 schema contract.
- 2026-08-12 (M1): **Node script M1 gate was a stub that swallowed test
  failures.** The original `EP-005.sh` printed `EP-005 M1: ok` even when
  `cargo test` failed because the rc check block was missing (the
  EP-001 gate-masking defect class). Fixed with `|| rc=$?` capture and
  the trailing rc check before printing the sentinel. Evidence: first M1
  run printed `ok` with a failing test; after the fix the same failure
  exited nonzero.
- 2026-08-12 (M2): **Adapter crate lives at `infra/nats/` as
  `nexus-nats`, a workspace member.** The M2 fence owns `infra/nats/`;
  the adapter implements the provider-neutral ports with real async-nats
  0.47.0 (pinned nats 2.14.3) and tokio 1.x. The contracts crate remains
  infrastructure-free (dependency-direction test still forbids nats/
  tokio there). Evidence: cargo tree check, M2 gate green.
- 2026-08-12 (M2): **Sync ports bridged to the async client with an
  owned tokio current-thread runtime.** The nexus-events port traits are
  synchronous (matching the sync postgres adapters), while async-nats
  0.47 is async. Each adapter owns a runtime and blocks on the async
  calls; JetStream publish ack semantics are preserved because
  `publish(...).await` returns only after the server ack.
- 2026-08-12 (M2): **One canonical subject namespace.** Subjects are
  derived deterministically: `nexus.<domain>.<event>.<tenant>`. A
  single-segment event type lives under the `general` domain. Wildcard
  helpers exist for domain and tenant-scoped durable consumers
  (SPEC-023 fallback doctrine: one stream, no sharding).
- 2026-08-12 (M3): **CONTRACT DEFECT CORRECTED: event ports made natively
  async; per-adapter runtimes removed.** M1 declared the
  infrastructure-facing ports synchronous; M2 therefore owned a tokio
  current-thread runtime inside every adapter instance and bridged with
  `Runtime::block_on`. Owner architecture clarification: Nexus
  infrastructure must use one process-owned async runtime (composition
  root); adapters must never own a runtime, call `block_on` from async
  context, or leak runtime lifecycle into domain contracts. `Publisher`,
  `Consumer`, and `Provisioner` operations are now declared `async fn`
  (native Rust async fn in traits, edition 2024; `async_fn_in_trait`
  lint allowed at crate root with Send-enforcement rationale). The NATS
  adapter stores only the JetStream context; `connect()` is async and
  must be called inside the composition root's runtime. M3 integration
  tests run on `#[tokio::test(flavor = "multi_thread")]` (the canonical
  test harness runtime), never on adapter-owned runtimes. The
  adapter's `ack` now retains delivered JetStream messages and
  explicitly acknowledges them on the server (verified via raw
  `num_ack_pending` on the real container). Evidence: 6/6 real
  nats:2.14.3 integration tests green; clippy `-D warnings` clean;
  `EP-005 M3: ok`.
- 2026-08-12 (M3): **Real-dependency M3 proofs against nats:2.14.3.**
  Six `ep005_integration_` tests prove: stream provisioning is
  idempotent; publish returns only after JetStream durable-storage ack;
  a durable pull consumer receives events and explicit acks clear
  `num_ack_pending` on the server; envelope survives
  encode/publish/consume/decode with full equality; checkpoint resume
  skips already-processed sequences; clean shutdown (dropping adapter
  handles + container) leaves zero orphaned containers. Host ports are
  dynamically allocated; readiness is proven through the published host
  port. No in-memory substitute; the pinned `nats:2.14.3` image is used.
- 2026-08-12 (M3): **Integration test fixture UUID corrected.** The
  event_id fixture format `...2c3d4e5fc{seed:02x}` produced an
  11-character final UUID group (Malformed). Corrected to
  `...2c3d4e5fc0{seed:02x}` to match the 12-character group shape used
  by tenant/correlation fixtures.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
