NODE-META-BEGIN
ID: EP-016
DEPS: EP-015
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-016
VERIFY_SENTINEL: node verify EP-016: ok
GREEN_TAG: green/EP-016
NODE-META-END

# 1. Purpose / Big Picture

Implement hybrid retrieval, context capsules, memory consolidation, retention, privacy, and graph-aware context construction. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-016.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-016.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-015` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-016.md`
- `.agent/specs/SPEC-002-data-memory-fabric-search-and-world-graph.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-016.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-016-context-engine-and-memory-consolidation.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-016.txt`
- `.agent/node-contracts/EP-016.md`
- `scripts/nodes/EP-016.sh`
- `crates/nexus-context/`
- `crates/nexus-memory-workers/`
- `packages/workflows/src/memory/`
- `tests/context/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `ContextEngine` | `nexus-context` | Defined by EP-016; provider-neutral and versioned |
| `ContextCapsule` | `nexus-context` | Defined by EP-016; provider-neutral and versioned |
| `HybridRetriever` | `nexus-context` | Defined by EP-016; provider-neutral and versioned |
| `MemoryConsolidator` | `nexus-context` | Defined by EP-016; provider-neutral and versioned |
| `MemoryProposal` | `nexus-context` | Defined by EP-016; provider-neutral and versioned |
| `PrivacyFilter` | `nexus-context` | Defined by EP-016; provider-neutral and versioned |
| `GraphExpansionPolicy` | `nexus-context` | Defined by EP-016; provider-neutral and versioned |

Acceptance obligations:

1. Context is purpose-limited and permission-filtered
2. Exact, full-text, vector, graph, recency, and importance signals are combined
3. Agents cannot write canonical memory directly
4. Private shared-room requests use private response routing

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement hybrid retrieval, context capsules, memory consolidation, retention, privacy, and graph-aware context construction.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-016-M1.txt`, `.agent/node-contracts/EP-016.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-016-context-engine-and-memory-consolidation.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-016.txt`, `.agent/node-contracts/EP-016.md`, `scripts/nodes/EP-016.sh`, `crates/nexus-context/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep016_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-016.sh M1`

EXPECT:

- `EP-016 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-016 MILESTONE_PASS "M1 EP-016 M1: ok"`

FALLBACK: Disable semantic consolidation and use deterministic retrieval if model-assisted consolidation fails evaluation. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-016][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-016.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-016-M2.txt`, `.agent/node-contracts/EP-016.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `crates/nexus-memory-workers/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep016_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-016.sh M2`

EXPECT:

- `EP-016 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-016 MILESTONE_PASS "M2 EP-016 M2: ok"`

FALLBACK: Disable semantic consolidation and use deterministic retrieval if model-assisted consolidation fails evaluation. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-016][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-016 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-016-M3.txt`, `.agent/node-contracts/EP-016.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `packages/workflows/src/memory/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep016_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-016.sh M3`

EXPECT:

- `EP-016 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-016 MILESTONE_PASS "M3 EP-016 M3: ok"`

FALLBACK: Disable semantic consolidation and use deterministic retrieval if model-assisted consolidation fails evaluation. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-016][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-016 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-016-M4.txt`, `.agent/node-contracts/EP-016.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/context/`

CONTENT:

1. Create tests whose names begin `ep016_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-016.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-016 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-016 MILESTONE_PASS "M4 EP-016 M4: ok"`

FALLBACK: Disable semantic consolidation and use deterministic retrieval if model-assisted consolidation fails evaluation. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-016][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-016.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-016-M5.txt`, `.agent/node-contracts/EP-016.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: Only the active ExecPlan progress, Decision Log, and ledger may change in this milestone.

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-016` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-016.sh M5`
2. `sh scripts/node-verify.sh EP-016`
3. `sh scripts/scope-audit.sh EP-016`

EXPECT:

- `EP-016 M5: ok`
- `node verify EP-016: ok`
- `scope audit EP-016: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-016 MILESTONE_PASS "M5 EP-016 M5: ok"`

FALLBACK: Disable semantic consolidation and use deterministic retrieval if model-assisted consolidation fails evaluation. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-016][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-016` and observe `node verify EP-016: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [x] M2: Core behavior and deterministic invariants
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-15 | Decision: Create `crates/nexus-context` as the EP-016 context plane crate. It re-exports the canonical SPEC-002 memory/context vocabulary from lower layers (`MemoryRecord`/`MemoryProposal`/`MemoryQuery`/`MemoryCandidate`/`Sensitivity`/`RetentionPolicy`/`EmbeddingRef`/`MemoryRepository`/`WorldGraphRepository`/`VectorRepository` from nexus-data, `MemoryType`/`NexusId`/`TenantId` from nexus-domain, `ContextCapsule`/`CapsuleId`/`CapsuleState`/`CapsuleReference`/`ContextCapsuleService` from nexus-fabric, `RetrievalPolicy`/`RetrievalBlend`/`ProposalEvaluator`/`RetentionEngine`/`LifecycleEngine` from nexus-memory) so EP-016 callers have a single import surface and locked names are never redefined, and adds the EP-016-owned provider-neutral ports (`ContextEngine`, `HybridRetriever`, `MemoryConsolidator`, `PrivacyFilter`, `GraphExpansionPolicy`) with typed request/outcome value types (`ContextRequest`, `RetrievalSignals`, `ConsolidationRequest`, `ConsolidationOutcome`, `FilteredCandidate`, `GraphExpansionRequest`, `GraphExpansion`, `GraphNodeRef`, `GraphEdgeRef`), the new vocabulary (`ContextPurpose`, `GraphExpansionMode`, `PrivacyFilterDecision`, `ConsolidationMode`; ADR-023 + vocabulary README), and the SPEC-006 `ContextError`/`ContextErrorCode`. The M1 gate was amended from artifact-check-only to run `cargo test --locked -p nexus-context ep016_unit` (vacuity gap, EP-015 M1 precedent). The fence was amended for Cargo.toml/Cargo.lock (workspace member registration), `references/ADR-023-context-engine-vocabulary.md`, and `docs/vocabulary/README.md`. Evidence: `EP-016 M1: ok` (24 ep016_unit tests + 1 dependency-direction), clippy clean (No issues found; result_large_err documented allow), format ok. Alternatives: redefine `ContextCapsule`/`MemoryProposal` in the crate (rejected: SPEC-002 names are vocabulary locked, already implemented in nexus-fabric/nexus-data); skip purpose-limitation vocabulary (rejected: SPEC-020 and acceptance obligation 1 require purpose-limited context). Consequence: deterministic purpose-limited, permission-filtered, bounded context construction contracts; consolidation always emits proposals for policy evaluation. Reversal: revert M1 commit. Security: redacted SPEC-006 errors; no credentials or sensitive content in messages. License: no new dependency classes (workspace members only). Compatibility: additive crate + vocabulary; no existing surface changed.
- 2026-08-15 | Decision: Implement the EP-016 M2 pure domain workers in `crates/nexus-memory-workers` (SPEC-002; ADR-023). Every policy choice below is a hard, documented rule; the pipeline order (permission -> purpose -> lifecycle -> hybrid -> diversity -> privacy -> budget -> capsule) is a security property, not a heuristic. (1) Permission-before-ranking: `PermissionFilter` excludes cross-tenant, unauthorized-namespace, and above-ceiling candidates BEFORE any scoring; unauthorized memory never enters the scoring pool (leak and side-channel reduction). (2) Purpose limitation is a hard constraint: `PurposeLimiter` scopes namespaces, memory types, and sensitivity ceilings per `ContextPurpose` (TaskExecution = room/device/procedure, Planning = business/project, Search = tenant-wide within permission, Notification = shared-safe subset only, SystemMaintenance = system/security); same query under different purposes yields different permissible sets. (3) Lifecycle filtering: `ActiveMemoryLifecycleFilter` excludes Deleted/Rejected/Superseded (unless historical requested)/Proposed and retention-expired records; legal hold (indefinite retention) preserves storage but never auto-selects a record into active context. (4) Hybrid score composition is a normalized deterministic blend of exact/full-text/vector/graph/recency/importance/confidence; exact structured/entity matches form a separate ranking tier above all non-exact candidates so a direct entity match never loses to vague embedding similarity; vector-unavailable sets semantic_available=false and the blend renormalizes over remaining signals (no synthetic embedding score). (5) Diversity: candidates cluster by supersession chain / derivation root and are capped per cluster (max_per_cluster, default 2); representative highest-quality candidate retained, provenance preserved (context compression is never memory deletion). (6) Graph expansion is bounded and purpose-aware: depth from `GraphExpansionMode` (DIRECT=0, ONE_HOP=1, TWO_HOP=2), deterministic sorted fanout cap, per-purpose relation allowlists (repair expands device->room->household->incident->procedure, never household->family-member), visited-set cycle safety, tenant boundary rejection, sensitivity boundary, and a node budget that marks `bounded=true`. (7) Privacy/disclosure: shared-room requests deny private/sensitive memories above the shared-safe ceiling (HOUSEHOLD) regardless of relevance; private channels may include them per permission; presence is evidence, never authority; `routing_decision` records private-route=true with delivery_owned=false (delivery is a later node's concern, never asserted here). (8) Context budget: fixed proportional allocation (RequiredExact 30 / ObjectiveState 20 / CriticalRecent 20 / HighValueRetrieved 15 / GraphContext 10 / OptionalSemantic 5) with required-exact guaranteed at least one slot so low-value retrieved memories cannot crowd out required state. (9) Determinism: all ordering uses stable tie-breakers (exact tier, total, observed time, canonical memory id); no HashMap iteration in output; same inputs -> same capsule. (10) Consolidation is proposal-before-canonical: `DeterministicMemoryConsolidator` always emits `MemoryProposal` (status PROPOSED) and never mutates canonical memory; `ProposalEvaluator` decides approval; deterministic fallback merges sources conservatively (confidence = min, sensitivity capped by source max and request ceiling, derived_from preserves provenance); semantic consolidation is reported ModelAssisted only when an injected adapter actually returns a result, else DeterministicFallback (never simulated); duplicate identical requests are idempotent (content-hash dedupe, no duplicate canonical mutation). (11) Telemetry is redacted by construction: purpose, counts, signal classes, graph depth, namespace FNV fingerprints, privacy decisions, consolidation mode, correlation id; never raw content, secrets, embeddings, or the capsule. (12) Errors use SPEC-006 `ContextError`/`ContextErrorCode` with safe redacted diagnostics; no memory content in messages. All I/O (candidate fetch, source records, graph nodes/edges, semantic model, clock) is injected through ports (`CandidateProvider`, `SourceProvider`, `GraphProvider`, `SemanticConsolidator`, injected `LifecycleContext.now_epoch_ms`); the worker never performs SQL, HTTP, filesystem, clock, random, or model calls. The M2 gate was amended from nexus-context-only to run `cargo test --locked -p nexus-memory-workers` first so the M2-owned crate is genuinely exercised (vacuity gap, EP-015 precedent). Evidence: `EP-016 M2: ok`; `cargo test --locked -p nexus-memory-workers` = 58 ep016_unit tests + 1 dependency-direction (3 suites, 0.26s) all green; nexus-context M1 regression 24+1 green; `cargo clippy --locked -p nexus-memory-workers -- -D warnings` clean; format ok; `sh scripts/lint.sh` -> `lint: ok`; `sh scripts/scope-audit.sh EP-016` -> `scope audit EP-016: ok`. Alternatives: rank-then-filter (rejected: leaks existence via score ordering and violates directive B); let vector similarity dominate exact matches (rejected: directive F); unconstrained graph traversal (rejected: directive H); auto-canonicalize proposals (rejected: SPEC-002 behavior 5); simulate model consolidation when unavailable (rejected: directive O). Consequence: deterministic, auditable, privacy-first context construction owned by EP-016; later nodes consume the same worker contracts. Reversal: revert M2 commit. Security: permission before scoring, purpose as hard constraint, shared-room disclosure denial, redacted telemetry/errors. License: no new dependency classes (workspace members only). Compatibility: additive crate; no existing surface changed.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
