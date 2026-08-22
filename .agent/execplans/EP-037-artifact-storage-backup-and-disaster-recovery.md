NODE-META-BEGIN
ID: EP-037
DEPS: EP-036
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-037
VERIFY_SENTINEL: node verify EP-037: ok
GREEN_TAG: green/EP-037
NODE-META-END

# 1. Purpose / Big Picture

Implement ArtifactStore with local, NAS, SeaweedFS, MinIO compatibility, R2, B2, and S3 plus encrypted backup, restore, and migration. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-037.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-037.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-036` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-037.md`
- `.agent/specs/SPEC-024-artifacts-object-storage-backup-restore-and-disaster-recovery.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-037.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-037-artifact-storage-backup-and-disaster-recovery.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-037.txt`
- `.agent/node-contracts/EP-037.md`
- `scripts/nodes/EP-037.sh`
- `crates/nexus-artifacts/`
- `connectors/storage-local/`
- `connectors/storage-nas/`
- `connectors/storage-seaweedfs/`
- `connectors/storage-s3/`
- `infra/storage/`
- `tests/artifacts/`
- `tests/backup/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `ArtifactStore` | `nexus-artifacts` | Defined by EP-037; provider-neutral and versioned |
| `ArtifactMetadata` | `nexus-artifacts` | Defined by EP-037; provider-neutral and versioned |
| `ArtifactVersion` | `nexus-artifacts` | Defined by EP-037; provider-neutral and versioned |
| `ArtifactHash` | `nexus-artifacts` | Defined by EP-037; provider-neutral and versioned |
| `StorageMigration` | `nexus-artifacts` | Defined by EP-037; provider-neutral and versioned |
| `BackupSet` | `nexus-artifacts` | Defined by EP-037; provider-neutral and versioned |
| `RestorePlan` | `nexus-artifacts` | Defined by EP-037; provider-neutral and versioned |
| `RetentionClass` | `nexus-artifacts` | Defined by EP-037; provider-neutral and versioned |

Acceptance obligations:

1. Local filesystem, NAS, SeaweedFS, R2, B2, S3, and MinIO compatibility use one contract
2. Artifacts are content-addressed and versioned
3. Backups encrypt before leaving the node
4. Restore and backend migration verify hashes before deletion

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement artifactstore with local, nas, seaweedfs, minio compatibility, r2, b2, and s3 plus encrypted backup, restore, and migration.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-037-M1.txt`, `.agent/node-contracts/EP-037.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-037-artifact-storage-backup-and-disaster-recovery.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-037.txt`, `.agent/node-contracts/EP-037.md`, `scripts/nodes/EP-037.sh`, `crates/nexus-artifacts/`, `infra/storage/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep037_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-037.sh M1`

EXPECT:

- `EP-037 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-037 MILESTONE_PASS "M1 EP-037 M1: ok"`

FALLBACK: Use local encrypted filesystem storage and restic-compatible backup before enabling scalable object storage. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-037][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-037.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-037-M2.txt`, `.agent/node-contracts/EP-037.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/storage-local/`, `tests/artifacts/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep037_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-037.sh M2`

EXPECT:

- `EP-037 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-037 MILESTONE_PASS "M2 EP-037 M2: ok"`

FALLBACK: Use local encrypted filesystem storage and restic-compatible backup before enabling scalable object storage. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-037][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-037 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-037-M3.txt`, `.agent/node-contracts/EP-037.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/storage-nas/`, `tests/backup/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep037_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-037.sh M3`

EXPECT:

- `EP-037 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-037 MILESTONE_PASS "M3 EP-037 M3: ok"`

FALLBACK: Use local encrypted filesystem storage and restic-compatible backup before enabling scalable object storage. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-037][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-037 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-037-M4.txt`, `.agent/node-contracts/EP-037.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/storage-seaweedfs/`

CONTENT:

1. Create tests whose names begin `ep037_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-037.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-037 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-037 MILESTONE_PASS "M4 EP-037 M4: ok"`

FALLBACK: Use local encrypted filesystem storage and restic-compatible backup before enabling scalable object storage. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-037][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-037.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-037-M5.txt`, `.agent/node-contracts/EP-037.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/storage-s3/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-037` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-037.sh M5`
2. `sh scripts/node-verify.sh EP-037`
3. `sh scripts/scope-audit.sh EP-037`

EXPECT:

- `EP-037 M5: ok`
- `node verify EP-037: ok`
- `scope audit EP-037: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-037 MILESTONE_PASS "M5 EP-037 M5: ok"`

FALLBACK: Use local encrypted filesystem storage and restic-compatible backup before enabling scalable object storage. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-037][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-037` and observe `node verify EP-037: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-002` `restore-existing-nexus`: Restore encrypted state onto a fresh deployment and prove identities, policies, memories, skills, and connectors reattach.
- `LF-020` `storage-backend-portability`: Write versioned artifacts, migrate between local and one S3-compatible backend, verify hashes and metadata, and remove the old copy only after approval.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary (2026-08-21): crates/nexus-artifacts @nexus-artifacts ArtifactStore contract crate (workspace member +1; deps nexus-domain + serde/serde_json only, no SDK/transport; provider-neutral contract for local/NAS/SeaweedFS/MinIO compatibility/R2/B2/S3 - StorageBackend vocabulary rejects unknown backends, MinIO marked compatibility-only (community repository archived); DataClass/RetentionClass/BackupState/RestoreVerificationState/MigrationState vocabularies; ArtifactHash canonical hex SHA-256 content addressing; ArtifactMetadata/ArtifactVersion with version-lineage binding + encryption-before-egress policy (sensitive class on remote backend requires EncryptionMetadata, local exempt); BackendLocation opaque reference rejects credential-bearing URLs; BackupSet state ladder DECLARED != CREATED != VERIFIED != RESTORED with recovery key never stored beside backup; RestorePlan hash-verification gates destructive steps; StorageMigration copy-verify-approve-delete ordering (delete only after verification + approval); RecoveryKey reference-only; ArtifactStore port with put/get/verify/delete/create_backup/restore/migrate/list/set_retention; 25 ep037_unit_* tests green (0 failed/ignored); clippy -D warnings clean; fmt clean; dependency-direction proof (cargo tree depth 1 rejects storage SDK/transport/framework crates); gate scripts/ep037-m1-tests.sh 8 anti-masking sentinels + vacuity guards; node M1 rewired from phantom node-artifact-check masking to real gate with rc propagation; infra/storage/ topology root (adapter ownership table + truthfulness boundaries); side gates: scope audit EP-037: ok, preflight: ok, reality gate: ok, security check: ok, license gate: ok, dependency audit: ok, blueprint validation: ok; expected-files later-owned dirs (connectors/storage-local M2, connectors/storage-nas + tests/backup M3, connectors/storage-seaweedfs M4, connectors/storage-s3 M5) recorded; certification (honest): EP-037 M1 contract layer INTERNAL CONTRACT CERTIFIED, ArtifactStore semantics INTERNAL CONTRACT CERTIFIED, content addressing + backup/restore/migration invariants INTERNAL CONTRACT CERTIFIED where tested; real local/NAS/SeaweedFS/MinIO/R2/B2/S3 backend adapters, real encryption, real backup/restore/migration live-fire, real provider access, hardware certification NOT ASSERTED (M2-M5 + deployment/native/ship milestones own them); ExecPlan Progress updated
- [ ] M2: Core behavior and deterministic invariants
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
