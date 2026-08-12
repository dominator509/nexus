NODE-META-BEGIN
ID: EP-004
DEPS: EP-003
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-004
VERIFY_SENTINEL: node verify EP-004: ok
GREEN_TAG: green/EP-004
NODE-META-END

# 1. Purpose / Big Picture

Implement PostgreSQL, pgvector, repositories, memory records, world graph abstraction, and migrations. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-004.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-004.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-003` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-004.md`
- `.agent/specs/SPEC-002-data-memory-fabric-search-and-world-graph.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-004.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-004-data-memory-and-world-graph.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-004.txt`
- `.agent/node-contracts/EP-004.md`
- `scripts/nodes/EP-004.sh`
- `crates/nexus-data/`
- `crates/nexus-memory/`
- `migrations/`
- `schemas/memory-record.schema.json`
- `tests/data/`
- `tests/memory/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `UnitOfWork` | `nexus-memory` | Defined by EP-004; provider-neutral and versioned |
| `RepositorySet` | `nexus-memory` | Defined by EP-004; provider-neutral and versioned |
| `MemoryRepository` | `nexus-memory` | Defined by EP-004; provider-neutral and versioned |
| `WorldGraphRepository` | `nexus-memory` | Defined by EP-004; provider-neutral and versioned |
| `PostgresWorldGraphRepository` | `nexus-memory` | Defined by EP-004; provider-neutral and versioned |
| `VectorRepository` | `nexus-memory` | Defined by EP-004; provider-neutral and versioned |
| `MemoryRecord` | `nexus-memory` | Defined by EP-004; provider-neutral and versioned |
| `MemoryQuery` | `nexus-memory` | Defined by EP-004; provider-neutral and versioned |
| `MemoryCandidate` | `nexus-memory` | Defined by EP-004; provider-neutral and versioned |
| `RetentionPolicy` | `nexus-memory` | Defined by EP-004; provider-neutral and versioned |

Acceptance obligations:

1. PostgreSQL stores canonical state with tenant isolation and additive migrations
2. pgvector is one retrieval index rather than the source of truth
3. WorldGraphRepository can be replaced without domain changes
4. Memory provenance, confidence, sensitivity, retention, supersession, and deletion are enforced

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement postgresql, pgvector, repositories, memory records, world graph abstraction, and migrations.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-004-M1.txt`, `.agent/node-contracts/EP-004.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-004-data-memory-and-world-graph.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-004.txt`, `.agent/node-contracts/EP-004.md`, `scripts/nodes/EP-004.sh`, `crates/nexus-data/`, `tests/memory/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep004_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-004.sh M1`

EXPECT:

- `EP-004 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-004 MILESTONE_PASS "M1 EP-004 M1: ok"`

FALLBACK: Use PostgreSQL recursive queries and adjacency tables only; do not add a dedicated graph database. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-004][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-004.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-004-M2.txt`, `.agent/node-contracts/EP-004.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `crates/nexus-memory/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep004_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-004.sh M2`

EXPECT:

- `EP-004 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-004 MILESTONE_PASS "M2 EP-004 M2: ok"`

FALLBACK: Use PostgreSQL recursive queries and adjacency tables only; do not add a dedicated graph database. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-004][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-004 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-004-M3.txt`, `.agent/node-contracts/EP-004.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `migrations/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep004_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-004.sh M3`

EXPECT:

- `EP-004 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-004 MILESTONE_PASS "M3 EP-004 M3: ok"`

FALLBACK: Use PostgreSQL recursive queries and adjacency tables only; do not add a dedicated graph database. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-004][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-004 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-004-M4.txt`, `.agent/node-contracts/EP-004.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `schemas/memory-record.schema.json`

CONTENT:

1. Create tests whose names begin `ep004_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-004.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-004 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-004 MILESTONE_PASS "M4 EP-004 M4: ok"`

FALLBACK: Use PostgreSQL recursive queries and adjacency tables only; do not add a dedicated graph database. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-004][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-004.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-004-M5.txt`, `.agent/node-contracts/EP-004.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/data/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-004` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-004.sh M5`
2. `sh scripts/node-verify.sh EP-004`
3. `sh scripts/scope-audit.sh EP-004`

EXPECT:

- `EP-004 M5: ok`
- `node verify EP-004: ok`
- `scope audit EP-004: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-004 MILESTONE_PASS "M5 EP-004 M5: ok"`

FALLBACK: Use PostgreSQL recursive queries and adjacency tables only; do not add a dedicated graph database. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-004][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-004` and observe `node verify EP-004: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

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

M1 completed 2026-08-12: `crates/nexus-data/` created with the canonical
memory/data contracts - `MemoryRecord`, `MemoryQuery`, `MemoryCandidate`,
`MemoryProposal`, `RetentionPolicy`/`RetentionUnit`, `Sensitivity`,
`MemoryStatus`, `EmbeddingRef` (matching `schemas/memory-record.schema.json`
exactly), `DataError`/`DataErrorCode` (SPEC-006 codes), `UnitOfWork`,
`RepositorySet`, and the `MemoryRepository`, `WorldGraphRepository`,
`PostgresWorldGraphRepository`, `VectorRepository` ports. ADR-008 adds the
memory vocabulary to `docs/vocabulary/README.md`; workspace membership and
Cargo.lock regenerated offline (89 packages). 13 `ep004_unit_` Rust tests +
dependency-direction test and 7 `tests/memory/` Python agreement tests pass.
Sentinel: `EP-004 M1: ok`. Fence extended with Cargo.toml, Cargo.lock,
docs/vocabulary/README.md, references/ADR-008, milestone M1 file.

M2 completed 2026-08-12: `crates/nexus-memory/` created with the
deterministic behavior engines: `ProposalEvaluator` (SPEC-002 behavior 5:
writes are proposals; only approved low-sensitivity proposals promote to
facts), `RetentionEngine` (legal-hold override, indefinite retention,
deletion-time computation; RFC 3339 parsed without chrono),
`LifecycleEngine` (PROPOSED -> ACTIVE -> SUPERSEDED/DELETED with terminal
deletion and supersession target validation), `RetrievalPolicy` (hybrid
blend ranking + per-namespace diversity cap). 19 `ep004_unit_` tests +
dependency-direction test pass. Sentinel: `EP-004 M2: ok`. Workspace
membership extended; Cargo.lock regenerated offline.

M3 completed 2026-08-12: `migrations/` created with two additive SQL
migrations. `001_memory_and_world_graph.sql`: `memory_records`
(SPEC-002 behavior 4, mirroring `schemas/memory-record.schema.json`;
native UUID keys, JSONB content, content_hash check, tenant status
constraint, supersedes self-check, tenant-scoped indexes, GIN FTS),
`world_graph_edges` adjacency table (fallback doctrine; recursive walks
only, no dedicated graph database). `002_memory_embeddings_vector.sql`:
pgvector extension, `memory_embeddings` (memory_id FK cascade, model and
dimensions provenance, `vector(384)`), tenant index, HNSW
`vector_cosine_ops` index. 6 `ep004_integration_` tests against real
`pgvector/pgvector:pg18` (pinned VERSIONS.lock.yaml) in ephemeral
containers with dynamically allocated host ports: JSONB round-trip,
tenant isolation, transactional supersession, recursive adjacency walk,
vector extension + HNSW + cosine proof, migration idempotency. postgres
0.19.14 dev-dep with `with-serde_json-1` + `with-uuid-1`; uuid dev-dep.
Cargo.lock regenerated offline (90 packages). Sentinel: `EP-004 M3: ok`.

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-12 (M1): **Memory/data contracts live in `nexus-data`; behavior
  and PostgreSQL adapters in `nexus-memory` (M2).** The node contract's
  interface map lists all ten interfaces under the memory boundary;
  the milestone fences split them: M1 owns `crates/nexus-data/` and
  `tests/memory/`, M2 owns `crates/nexus-memory/`. Decision: `nexus-data`
  holds the provider-neutral contracts and ports (UnitOfWork, RepositorySet,
  MemoryRepository, WorldGraphRepository, VectorRepository, MemoryRecord,
  MemoryQuery, MemoryCandidate, MemoryProposal, RetentionPolicy, enums,
  errors); `nexus-memory` will hold behavior and the PostgreSQL/pgvector
  implementations. Evidence: EP-004 milestone fences, M1 gate green.
  Alternative rejected: put everything in one crate (violates the M1/M2
  fence split).
- 2026-08-12 (M1): **Memory vocabulary added by ADR-008.** Sensitivity,
  MemoryStatus, RetentionUnit, EmbeddingRef, MemoryProposal are
  vocabulary-locked contracts owned by `nexus-data`; MemoryType stays in
  `nexus-domain`. Wire values mirror `schemas/memory-record.schema.json`
  (bootstrap; M4 amends the schema to lock enum values). Evidence: ADR-008,
  vocabulary README, unit tests, Python agreement tests. Alternative
  rejected: free-form strings (lose parse-time rejection).
- 2026-08-12 (M1): **`supersedes`/`derived_from`/`embedding_ref` are
  optional on the wire.** The bootstrap schema requires 15 fields and marks
  the provenance-chain and index-reference fields optional; the Python
  agreement test initially asserted 18 required and was corrected to the
  schema's actual contract. Evidence: `tests/memory/` green against the
  real schema. Consequence: `MemoryRecord` validation requires the 15 core
  fields; provenance chains are optional enrichment.
- 2026-08-12 (M2): **Behavior engines live in `nexus-memory`, not in the
  port crate.** `nexus-data` holds contracts/ports; `nexus-memory` owns
  deterministic behavior: `ProposalEvaluator` (proposal-to-fact policy,
  SPEC-002 behavior 5), `RetentionEngine` (legal-hold-aware retention,
  behaviors 4/8), `LifecycleEngine` (PROPOSED/ACTIVE/SUPERSEDED/DELETED
  transitions, terminal deletion), `RetrievalPolicy` (hybrid blend +
  per-namespace diversity, behavior 6). Evidence: 19 `ep004_unit_` tests +
  dependency-direction test. Alternative rejected: put behavior in the
  port crate (blurs contract/implementation split and breaks the M2
  fence).
- 2026-08-12 (M2): **RFC 3339 parsing implemented without chrono.**
  Dependency-direction forbids chrono in domain-adjacent crates, so the
  retention engine parses the fixed-width RFC 3339 UTC prefix with the
  Hinnant civil-from-days algorithm. Evidence: round-trip test. Consequence:
  only the canonical UTC form is accepted; fractional seconds and offsets
  are rejected (schema uses date-time UTC).
- 2026-08-12 (M3): **Memory and graph tables use native PostgreSQL UUID
  columns.** SPEC-002 requires typed IDs; EP-003 used TEXT columns, but the
  EP-004 migrations declare `UUID PRIMARY KEY` / `UUID NOT NULL` and the
  integration tests pass `uuid::Uuid` values via the postgres
  `with-uuid-1` feature. Evidence: `ep004_integration_*` green against
  `pgvector/pgvector:pg18`; `uid()` parse helper in
  `tests/integration_postgres.rs`. Alternative rejected: TEXT columns
  (loses DB-level type enforcement) and the `uuid` crate's `Uuid` only as
  strings (client-side ToSql mismatch, observed as WrongType errors).
  Consequence: uuid 1.x dev-dep; dev-deps stay invisible to the
  dependency-direction gate.
- 2026-08-12 (M3): **Timestamp parameters use SQL literals, not typed
  parameters.** The postgres crate rejects `&str` for a `$n::timestamptz`
  parameter (client-side ToSql type check) and the workspace deliberately
  avoids chrono, so the tests embed fixed RFC 3339 UTC literals with
  `::timestamptz` casts. Evidence: round-trip / supersession tests green.
  Alternative rejected: adding chrono + `with-chrono-1` (introduces the
  dependency the M2 decision explicitly excluded).
- 2026-08-12 (M3): **pgvector test values are generated 384-dimension
  literals embedded in SQL.** The postgres crate cannot map `String` to
  the `vector` OID (client-side ToSql rejection), so the test builds a
  384-dimension `[0.1,...]` literal and interpolates it into the SQL text
  with `::vector` cast; the embedding row also inserts its parent
  `memory_records` row first because the FK is real and enforced.
  Evidence: `ep004_integration_pgvector_extension_and_hnsw_are_real`
  green. Consequence: the HNSW index and cosine operator are exercised
  against the real extension.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
