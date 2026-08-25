NODE-META-BEGIN
ID: EP-042
DEPS: EP-041
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-042
VERIFY_SENTINEL: node verify EP-042: ok
GREEN_TAG: green/EP-042
NODE-META-END

# 1. Purpose / Big Picture

Implement signed releases, installers, offline bundle, transactional updates, staged rollout, backup-before-update, provider migration, and rollback drills. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-042.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-042.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-041` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-042.md`
- `.agent/specs/SPEC-016-deployment-profiles-setup-compute-fabric-provisioning-and-updates.md`
- `.agent/specs/SPEC-024-artifacts-object-storage-backup-restore-and-disaster-recovery.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-042.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-042-deployment-release-update-and-rollback.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-042.txt`
- `.agent/node-contracts/EP-042.md`
- `scripts/nodes/EP-042.sh`
- `crates/nexus-release/`
- `apps/setup/src/update/`
- `infra/release/`
- `installers/`
- `offline-bundle/`
- `.github/workflows/release.yml`
- `tests/release/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `ReleaseManifest` | `nexus-release` | Defined by EP-042; provider-neutral and versioned |
| `SignedComponent` | `nexus-release` | Defined by EP-042; provider-neutral and versioned |
| `CompatibilityMatrix` | `nexus-release` | Defined by EP-042; provider-neutral and versioned |
| `UpdatePlan` | `nexus-release` | Defined by EP-042; provider-neutral and versioned |
| `CanaryRing` | `nexus-release` | Defined by EP-042; provider-neutral and versioned |
| `RollbackReceipt` | `nexus-release` | Defined by EP-042; provider-neutral and versioned |
| `OfflineBundle` | `nexus-release` | Defined by EP-042; provider-neutral and versioned |
| `ManualPromotion` | `nexus-release` | Defined by EP-042; provider-neutral and versioned |

Acceptance obligations:

1. One signed distribution supports managed, BYOC, hybrid, and local profiles
2. Updates verify signatures, back up, migrate, canary, observe, promote, or roll back
3. Offline bundles contain approved images, models, licenses, SBOMs, and manifests
4. Production promotion remains an exact manual action

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement signed releases, installers, offline bundle, transactional updates, staged rollout, backup-before-update, provider migration, and rollback drills.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-042-M1.txt`, `.agent/node-contracts/EP-042.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-042-deployment-release-update-and-rollback.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-042.txt`, `.agent/node-contracts/EP-042.md`, `scripts/nodes/EP-042.sh`, `crates/nexus-release/`, `.github/workflows/release.yml`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep042_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-042.sh M1`

EXPECT:

- `EP-042 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-042 MILESTONE_PASS "M1 EP-042 M1: ok"`

FALLBACK: Ship pinned full-version updates before independent component updates. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-042][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-042.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-042-M2.txt`, `.agent/node-contracts/EP-042.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `apps/setup/src/update/`, `tests/release/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep042_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-042.sh M2`

EXPECT:

- `EP-042 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-042 MILESTONE_PASS "M2 EP-042 M2: ok"`

FALLBACK: Ship pinned full-version updates before independent component updates. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-042][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-042 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-042-M3.txt`, `.agent/node-contracts/EP-042.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/release/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep042_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-042.sh M3`

EXPECT:

- `EP-042 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-042 MILESTONE_PASS "M3 EP-042 M3: ok"`

FALLBACK: Ship pinned full-version updates before independent component updates. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-042][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-042 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-042-M4.txt`, `.agent/node-contracts/EP-042.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `installers/`

CONTENT:

1. Create tests whose names begin `ep042_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-042.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-042 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-042 MILESTONE_PASS "M4 EP-042 M4: ok"`

FALLBACK: Ship pinned full-version updates before independent component updates. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-042][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-042.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-042-M5.txt`, `.agent/node-contracts/EP-042.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `offline-bundle/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-042` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-042.sh M5`
2. `sh scripts/node-verify.sh EP-042`
3. `sh scripts/scope-audit.sh EP-042`

EXPECT:

- `EP-042 M5: ok`
- `node verify EP-042: ok`
- `scope audit EP-042: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-042 MILESTONE_PASS "M5 EP-042 M5: ok"`

FALLBACK: Ship pinned full-version updates before independent component updates. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-042][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-042` and observe `node verify EP-042: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

## EP-042 M1 DISPATCH (boot 2026-08-25, lease 178c364)

M1: Contract, vocabulary, and package boundary.

- M1-owned exact paths: `.agent/execplans/EP-042-*.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-042.txt`, `.agent/node-contracts/EP-042.md`, `scripts/nodes/EP-042.sh`, `crates/nexus-release/`, `.github/workflows/release.yml`, plus M1-added `scripts/ep042-m1-tests.sh` (gate) and `references/ADR-028-deployment-release-update-vocabulary.md` (vocabulary ADR), and workspace manifest `Cargo.toml`/`Cargo.lock` (EP-038 M1 precedent).
- M1 invariants (encoded in the crate and proven by tests):
  - RELEASE MANIFEST EXISTS != RELEASE VERIFIED; SIGNATURE PRESENT != SIGNATURE VALID
  - UPDATE PLAN EXISTS != UPDATE EXECUTED; first update step is BACKUP (backup-before-update); plans never contain PROMOTE (promotion is manual)
  - CANARY OBSERVING != PROMOTED; canary verdicts never deploy
  - PROMOTION DECISION != DEPLOYMENT; ManualPromotion requires a human approval ref and never performs deployment
  - ROLLBACK RECEIPT REQUIRES BACKUP REF; receipt != rollback verified
  - OFFLINE BUNDLE EXISTS != OFFLINE BUNDLE VERIFIED; bundles require approved image + model + license + SBOM contents
  - One signed ReleaseManifest supports MANAGED/BYOC/EXISTING_SSH/HYBRID/FULLY_LOCAL profiles (channel vocabulary from schemas/deployment-profile.schema.json)
  - Every public vocabulary deny-unknown; versioned serialization preserves/rejects schema_version; digest = alg:hex with >=32 hex chars
- Anti-fabrication: production code has no placeholder/demo/sample-success/hidden fallback; test-double code stays in #[cfg(test)]; no claim of signature validity, update execution, promotion, or deployment - M1 certifies contract/vocabulary only.
- Non-vacuous gate `scripts/ep042-m1-tests.sh`: material presence, real `cargo test -p nexus-release --locked` with >= 30 passed / zero failed / zero ignored, per-interface anti-masking sentinels, dependency-direction (nexus-domain + serde + serde_json + sha2 only, no provider SDK), no-placeholder scan, clippy -D warnings, fmt check, node M1 wired to gate (no artifact-check masking).
- Regression requirement: EP-041 node gates remain green (node verify EP-041: ok) - M1 adds a workspace member and must not break the workspace battery.
- Committed-tree reproduction: after the M1 commit, rerun the gate on the committed tree and confirm `EP-042 M1: ok` + side gates (scope audit, expected files, security, license, dependency audit, reality gate, format, lint, typecheck, test-unit, blueprint validation).
- Certification boundary: M1 certifies the nexus-release contract/vocabulary package boundary only. NOT ASSERTED: real signature verification against keys, update execution, canary rollout, backup/restore, rollback drills, offline bundle production, release build, deployment, provider certification.
- No graph-next after M1 (graph-next is the scheduler's authority, not a per-milestone tool).

- [x] M1: Contract, vocabulary, and package boundary
- [x] M2: Core behavior and deterministic invariants
- [x] M3: Real dependency and transport integration
- [x] M4: Forced failures, abuse cases, and observability
- [x] M5: Live-fire, operations, and node closure

## EP-042 M3 DISPATCH (2026-08-25)

M3: Real dependency and transport integration.

- M3-owned exact paths: `infra/release/` (new workspace package @nexus/release-infra: src/errors.ts, src/sigv4.ts, src/s3.ts, src/transport.ts, src/cli.ts, src/index.ts, scripts/release-probe.sh, scripts/release-publish.sh, scripts/release-fetch.sh, providers/seaweedfs.yaml, containers/seaweedfs.yaml, fixtures/release-manifest.json + fixtures/components/{nexus-core,nexus-model}, README.md, package.json, tsconfig.json, tsconfig.build.json), `tests/release/src/integration/ep042_integration_transport.test.ts` + `tests/release/vitest.integration.config.ts`, gate `scripts/ep042-m3-tests.sh`, node `scripts/nodes/EP-042.sh` M3 branch, `.agent/expected-files/EP-042.txt` (+EP-042-M3.txt, +scripts/ep042-m3-tests.sh), ExecPlan, ledger.
- M3 invariants: DIGEST PRESENT != ARTIFACT VERIFIED (fetch recomputes sha256 over real bytes, fails closed on mismatch); TRANSPORT CONFIG EXISTS != TRANSPORT EXECUTED; UPDATE PLAN EXISTS != UPDATE EXECUTED (transport never executes); RELEASE MANIFEST EXISTS != RELEASE VERIFIED; SIGNATURE FIELD EXISTS != SIGNATURE VERIFIED.
- Real component used: SeaweedFS 4.43 (chrislusf/seaweedfs:4.43@sha256:4d5118...) from COMPONENT_REGISTRY (PROVIDER CERTIFIED S3 gateway in EP-037 M4); M3 gate runs a REAL digest-pinned container with runtime credentials.
- Non-vacuous gate `scripts/ep042-m3-tests.sh`: resource preflight, M1+M2 regressions, material presence, workspace+registry registration, node M3 anti-masking, sh -n, real container start + healthz, real transport probe (probe_verified: true), real release-publish.sh + release-fetch.sh with cmp-verified bytes, vitest integration suite (>= 14 passed zero failed), anti-masking sentinels (8 proof classes), no-placeholder scan, typecheck, zero EP-042 M3 residue after teardown.
- Integration proofs (14, all real container): readiness (healthz + probe), publish/fetch roundtrip + head, digest binding negatives (bytes mismatch, missing declared digest, malformed manifest, corrupted stored bytes, missing object), wrong-secret auth denied, unreachable timeout, cancellation, idempotency (re-publish one object same digest), audit redaction (runtime secret canary never leaks).
- Regression requirement: M1 gate + M2 gate rerun green inside the M3 gate (EP-042 M1: ok, EP-042 M2: ok); side gates: format ok, typecheck ok, unit ok, security ok, dependency audit ok, license gate ok, reality gate ok, blueprint ok, scope audit EP-042 ok.
- Committed-tree reproduction: rerun the full gate + node M3 on the committed tree.
- Certification boundary: REAL SigV4 transport over a real SeaweedFS S3 gateway INTERNAL BEHAVIOR CERTIFIED for exact exercised local surface; real signature verification NOT ASSERTED (no key store/verifier); update execution NOT ASSERTED; canary rollout / backup-restore / rollback drills / offline bundle production / release build / deployment / remote synchronization NOT ASSERTED; external clouds (R2/B2/AWS) NOT ASSERTED.
- No graph-next after M3 (scheduler authority).

## EP-042 M5 DISPATCH (2026-08-25)

M5: Live-fire, operations, and node closure (offline bundle).

- M5-owned exact paths: `offline-bundle/` (new workspace package @nexus/offline-bundle: src/errors.ts, src/model.ts, src/produce.ts, src/verify.ts, src/install.ts, src/rollback.ts, src/evidence.ts, src/cli.ts, src/index.ts, scripts/ts-resolve-loader.mjs, scripts/bundle-produce.sh, scripts/bundle-verify.sh, scripts/bundle-install.sh, scripts/bundle-rollback.sh, OPERATIONS.md, README.md, package.json, tsconfig.json), `tests/release/src/bundle/ep042_bundle_offline.test.ts` + `tests/release/vitest.bundle.config.ts`, gate `scripts/ep042-m5-tests.sh`, node `scripts/nodes/EP-042.sh` M5/verify branch, `.agent/expected-files/EP-042.txt` (+EP-042-M5.txt, +scripts/ep042-m5-tests.sh, +offline-bundle/), `.agent/state/evidence/ep042-m5/EP-042-M5-evidence.json`, ExecPlan, ledger.
- M5 invariants: OFFLINE BUNDLE EXISTS != OFFLINE BUNDLE VERIFIED (verifyBundle proves each declared file exists + digest matches real bytes + manifest binding + release id + no duplicate/escape + self-digest); OFFLINE BUNDLE VERIFIED != OFFLINE INSTALL VERIFIED (install composes the M4 transactional installer with bytes from local bundle files only, transport absent); ROLLBACK RECEIPT EXISTS != ROLLBACK PROVEN (drill writes receipt only after verified restoration; wrong/corrupt backup source denied); SIGNATURE PRESENT != SIGNATURE VALID (evidence records SIGNATURE_PRESENT_NOT_VERIFIED).
- Non-vacuous gate `scripts/ep042-m5-tests.sh`: resource preflight, control-plane runtime smoke (EP-044 stage DONE), M1+M2+M3+M4 regressions, material presence, workspace registration, node M5 anti-masking, sh -n, typecheck, real bundle-produce.sh (cmp-verified payloads), real bundle-verify.sh (VERIFIED), real OFFLINE bundle-install.sh with NO transport (transport_required: false, cmp-verified installed bytes), real wrong-backup rollback denial (ROLLBACK_FAILED), real bundle-rollback.sh (prior state restored + verified + receipt after verification), real tampered bundle denial (BUNDLE_DIGEST_MISMATCH), real path traversal denial (PATH_ESCAPE), current-run evidence written + redacted + validated, vitest bundle suite (>= 16 passed zero failed), anti-masking sentinels (19 proof classes), no-placeholder scan, expected-files EP-042 full list, side gates (scope audit/security/dependency/license/reality/blueprint), zero EP-042 M5 residue after teardown.
- Bundle proofs (19, all real): production (real files + real digests + required kinds), verification positive, missing file denied, changed file denied, malformed digest denied, duplicate path denied, path traversal denied, symlink escape denied, wrong release denied, manifest tamper denied, self-digest tamper denied, offline install succeeds (transport absent), offline install component missing denied, offline install unverified denied, rollback drill restores prior + receipt after verification, rollback wrong backup denied, evidence bound + redacted, evidence stale rejected, evidence tampered rejected.
- Regression requirement: M1/M2/M3/M4 gates rerun green inside the M5 gate; side gates green; expected-files EP-042 full list green.
- Committed-tree reproduction: rerun the full gate + node M5 + M1-M4 regressions + node verify EP-042 on the committed tree.
- Certification boundary: offline-bundle production/verification/offline-install/rollback-drill/evidence INTERNAL BEHAVIOR CERTIFIED for exact exercised local surface; production host upgrade / real release signature verification / canary rollout / production backup-restore / production rollback / deployment / AWS-R2-B2 transport NOT ASSERTED.
- No graph-next after M5 (scheduler authority; closure sequence owns graph-next).

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

## 2026-08-25 - EP-042 M4 discoveries

- Discovery: Node native TypeScript type-stripping cannot load the canonical @nexus/setup update core directly because its relative imports are extensionless (bundler resolution) and it uses TS enums. A resolution-only ESM loader (installers/scripts/ts-resolve-loader.mjs) + `--experimental-transform-types` lets the installer CLI execute the REAL canonical code with zero bundler and zero duplicated logic. The relative loader URL itself was treated as a package by Node - an absolute loader path is required.
- Discovery: chattr +i provides a real EACCES-equivalent permission denial even when running as root; the M4 failure suite uses it for the denied-permission proof (no mock).
- Discovery: symlink-escape detection must resolve the deepest existing ancestor of the target path, not the target itself (the staged file does not exist at check time); a parent-directory symlink to outside was otherwise only caught after the write, which was too late.

## 2026-08-25 - EP-042 M5 discoveries

- Discovery: the canonical M2 Digest type returned by contentDigest() already carries the full `sha256:` prefix in asString() - wrapping it with a second `sha256:` prefix produced a malformed digest that verifyBundle correctly rejected (real defect found by the bundle suite).
- Discovery: the strip-then-digest self-binding must EXCLUDE the digest field entirely, not set it to null; contentDigest over an object with `bundle_digest: null` produced a different canonical form than the verifier's `...rest` strip (real defect found by the bundle suite).
- Discovery: parseReleaseManifest (M2) accepts a parsed OBJECT, not a wire string; JSON.parse before the canonical parse was required in produce/verify/install (real defect found by the bundle suite).
- Discovery: the M4 rollbackRelease records expectedBackupDigest but does not verify it against the backup bytes; the M5 rollback drill layer verifies the backup source digest via the canonical M4 verifyBackupDigest surface before any restore (wrong/corrupt source denied at the drill layer; M4 surface untouched).
- Discovery: the M5 gate's blueprint invocation must use `python3 scripts/blueprint_validate.py` (not `sh`) - the first gate run failed at the final side gate on that invocation error; dependency-audit once hit a transient cargo-deny lock/network failure during the long gate run (passed in isolation and on rerun).

# 13. Decision Log

## 2026-08-25 - EP-042 M1 decisions

- Decision: crates/nexus-release is a Rust contract crate (mirrors nexus-observability EP-038 pattern) owning all 8 node-contract interfaces; deps limited to nexus-domain + serde + serde_json + sha2. Evidence: node contract interface map (all owning package nexus-release); EP-038 M1 precedent; dependency-direction gate. Alternatives: Python package (rejected - ARCHITECTURE.md assigns Rust contract layer); schemas-generated bindings (rejected - no cross-language contract in M1; deployment-profile.schema.json already canonicalizes mode/release_channel). Consequence: M1 provides typed Rust contract surface; later milestones add installers/update engine in TS/Rust apps without duplicating canonical names. Reversal: ADR + vocabulary update in the same milestone.
- Decision: Digest format gate = `sha256:` + lowercase hex >= 32 chars (EP-041 artifact-identity precedent). Evidence: node contract identity rule alg:hex >=32; tests prove rejection of missing prefix, unsupported alg, short hex, non-hex, uppercase hex. Consequence: real content digests computed by the crate are exactly 64 chars.
- Decision: content_digest() excludes the self-referential digest field (manifest_digest/bundle_digest set to None before serialization). Evidence: without it the binding is unverifiable (chicken-and-egg); test ep042_unit_manifest_exists_not_verified caught the naive implementation. Consequence: digest binds content excluding its own field - verifiable and deterministic. Reversal: strip-then-digest is the canonical form; keep.
- Decision: UpdateStepKind has NO PROMOTE variant; UpdatePlan requires first step BACKUP; RollbackReceipt.backup_ref is mandatory (not Option); CanaryVerdict has NO PROMOTED variant; ManualPromotion requires approval_ref and carries exact_manual_command only. Evidence: SPEC-016 behaviors 6/7; node contract acceptance obligations 2 and 4; vocabulary + construction tests. Consequence: promotion and deployment cannot be expressed inside the update engine surface - they remain exact manual actions.
- Decision: CompatibilityEntry.version must exactly match SignedComponent.version in matrix.check(); min/max bound range checking applies to update compatibility. Evidence: matrix tests; a release declares exact component versions, bounds guide updates. Consequence: unknown/mismatched/out-of-range versions fail closed.
- Decision: Provider names stay free-form strings (ObjectRef.backend); no provider enum in domain contracts. Evidence: ARCHITECTURE.md forbidden moves (provider names in domain capabilities). Consequence: M1 contract surface is provider-neutral; installers may normalize at the boundary (M3+).
- Decision: M1 gate scripts/ep042-m1-tests.sh is the authoritative gate (no artifact-check masking); node M1 branch rewired. Evidence: EP-041/EP-038 precedent; anti-masking sentinels; vacuity guards. Consequence: committed-tree reproduction is a real gate run, not a file existence check.
- Security impact: redaction-first error surface; signature key material and secret-shaped values never appear in error messages (runtime-constructed canary test).
- License impact: none (MIT crate, no new third-party dependency beyond sha2 already in the workspace).
- Compatibility impact: schema_version fixed at 1; serde deny_unknown_fields on every record; roundtrip tests preserve schema_version.

## 2026-08-25 - EP-042 M2 decisions

- Decision: M2 deterministic update behavior lives in apps/setup/src/update/ as pure TypeScript modules (boundary adaptation of the canonical M1 Rust contracts) with proofs in tests/release/ (new @nexus/release-tests vitest workspace package). Evidence: M2 fence (apps/setup/src/update/ + tests/release/); EP-035 vitest package convention; tests/livefire/deployment TS test package pattern; ARCHITECTURE.md canonical-truth-in-Rust + boundary adaptation. Alternatives: extending crates/nexus-release in M2 (rejected - M1 owns that crate; fence assigns M2 to the TS surface); Rust test crate at tests/release (rejected - the update core is TS in the setup app). Consequence: canonical contracts stay in crates/nexus-release; the TS core adapts the wire format with deny-unknown parsing and mirrors the M1 invariants. Reversal: ADR + fence amendment.
- Decision: Update core reuses EP-035 canonical ReleaseChannel/DeploymentMode bindings from contracts/deployment instead of redefining them (TS2308 barrel collision forced the choice; ARCHITECTURE.md forbids duplicating canonical domain names). Evidence: apps/setup/src/contracts/deployment.ts already mirrors schemas/deployment-profile.schema.json. Consequence: one canonical binding for channel/mode across EP-035 and EP-042.
- Decision: parseUpdatePlan enforces backup-first step (M1 contract parity) and state PLANNED only; the planner emits canonical steps BACKUP/MIGRATE/CANARY/OBSERVE/ROLLBACK (rollback as declared contingency), never PROMOTE. Evidence: M1 UpdatePlan::new backup-first; SPEC-016 behavior 6; fence G (missing backup precondition denied, missing rollback path denied). Consequence: no plan without backup-first can exist through the typed boundary.
- Decision: digest binding uses Web Crypto (globalThis.crypto.subtle) - pure, framework-neutral, no node builtin imports. Evidence: dependency-direction gate; TS2345 fixed with Uint8Array<ArrayBuffer> typing. Consequence: sha256Hex is real 64-char hex; contentDigest is strip-then-digest over canonical JSON.
- Decision: backup-before-update policy evaluates a BackupProof {backup_id, install_id, digest, completed_at, state}; digest format validated with isDigestString (real defect: initial check accepted "not-a-digest" - fixed). Evidence: fence I; test ep042_unit_backup_proof_bad_digest_denied.
- Decision: rollback preconditions require plan rollback path + receipt bound to plan/versions + backup_ref + drill evidence; drill NOT_RUN/absent = NOT_PROVEN. Evidence: fence J; tests.
- Decision: promotion gate (evaluatePromotionGate / evaluateFullPromotionGate) returns only LOCKED/AWAITING_HUMAN_APPROVAL/APPROVED_MANUAL_ONLY and enforces backup+rollback preconditions before approval. Evidence: fence K; tests.
- Decision: evidence builder redacts secret-shaped values (runtime-constructed canaries in tests); redaction_applied flag reflects canary treatment. Evidence: fence L; tests.
- Security impact: redaction-first; no tracked secret literals (canaries runtime-constructed); no node builtins in update core.
- License impact: none (no new third-party dependency; vitest/typescript already in workspace).
- Compatibility impact: wire format matches M1 serde (snake_case, SCREAMING_SNAKE_CASE, deny-unknown); pnpm-lock.yaml updated for the new workspace package.

## 2026-08-25 - EP-042 M3 decisions

- Decision: M3 real release transport lives in infra/release/ as a new workspace package @nexus/release-infra (pnpm glob infra/* already registered). It implements real AWS SigV4 request signing over Web Crypto (HMAC-SHA256) and a minimal S3 client over global fetch, with digest-bound publish/fetch of release manifests + component artifacts, a readiness probe (healthz + PUT/GET/digest/DELETE), idempotent publish, timeout/cancellation, and current-run redacted audit events. Evidence: M3 fence (infra/release/); ExecPlan M3 CONTENT 1 (use selected open-source component from COMPONENT_REGISTRY); COMPONENT_REGISTRY id seaweedfs (digest-pinned, PROVIDER CERTIFIED for S3-gateway in EP-037 M4). Alternatives: Python boto3 transport (rejected - boto3 not a repo dependency; repo is Rust+TS); curl+openssl SigV4 (rejected - no SDK-free shell SigV4 path); Rust connector reuse (rejected - fence assigns infra/release/, not connectors/). Consequence: real transport over the certified S3-gateway surface with zero new third-party deps.
- Decision: transport scripts (release-probe.sh, release-publish.sh, release-fetch.sh) are real POSIX sh that invoke the transport CLI (node src/cli.ts) with runtime env credentials; the CLI runs under Node 24 native TS type-stripping. Evidence: fence N (infra/release owned scripts actually execute, no echo-only); real probe/publish/fetch output observed in gate; cmp-verified fetched bytes. Consequence: scripts are not mocks; failures exit nonzero.
- Decision: integration proofs live in tests/release/src/integration/ep042_integration_transport.test.ts with a dedicated vitest.integration.config.ts so the M2 gate's unit-only run stays untouched. Evidence: fence N/O/P (M2 regression must stay green); M2 gate runs vitest run src/__tests__ only. Consequence: 14 ep042_integration_* proofs run only under the M3 gate with the real container.
- Decision: SeaweedFS container is digest-pinned (sha256:4d5118...) with runtime-generated credentials in a temp s3.config; exact EP-042 M3 ownership naming (nexus-ep042-m3-*) and teardown verified to zero residue. Evidence: fence B/R; EP-037 M4 precedent; gate pressure + residue checks. Consequence: no shared fixture mutation, no broad prune.
- Decision: real defects found+fixed by the M3 suite: (1) SigV4 canonical URI encoded the leading slash (%2F...) - fixed to encode path segments and rejoin with '/'; (2) SigV4 canonical query double-encoded pre-encoded prefixes and the URL.search setter re-encoded %2F - canonical query is now computed once by signRequest and used verbatim for the href; (3) SeaweedFS createBucket returns 403 without bucket existence - probe creates the bucket first; (4) fetch output dirs must exist - scripts mkdir -p; (5) manifest digests are canonical sha256:hex - extractDeclaredDigests strips the prefix; (6) redaction shape regex missed short runtime secrets - audit() scrubs exact configured credential values plus shapes.
- Security impact: redaction-first audit; no tracked secret literals (runtime-constructed s3.config, canary test proves zero credential leakage in audit events); no-placeholder scan clean; security-check green.
- License impact: none (no new third-party npm or cargo dependency; SeaweedFS Apache-2.0 already recorded in COMPONENT_REGISTRY).
- Compatibility impact: @nexus/release-infra depends only on Web Crypto + global fetch (no node builtins in src except CLI I/O); tests/release gains workspace dep; pnpm-lock.yaml updated.

## 2026-08-25 - EP-042 M4 decisions

- Decision: M4 real local installer lives in installers/ as a new workspace package @nexus/installers (pnpm glob added as exact "installers" entry, not "installers/*" - the package.json sits at the package root like apps/setup). It implements a REAL transactional installer: canonical manifest validation (parseReleaseManifest + verifyManifestDigestBinding from @nexus/setup), backup-before-update (real bytes copied, real sha256 digest, verified; backup failure denies the update), staged replacement (real bytes, digest-checked), validation, atomic switch (rename), rollback (restore prior bytes + verify), quarantine, typed failure classification (17 classes), abuse-case guards (path traversal, symlink escape via deepest-existing-ancestor realpath, duplicate overwrite, foreign-root cleanup), append-only journal, redacted observability, ops diagnostic + bounded recovery (installer-recover.sh). Evidence: M4 fence (installers/); ExecPlan M4 CONTENT 1-6; SPEC-016 behavior 6; SPEC-024. Alternatives: extending crates/nexus-release (rejected - M1 owns the contract crate); pure policy in apps/setup (rejected - fence assigns installers/). Consequence: canonical truth stays in M1/M2; installers/ is the local execution boundary. Reversal: ADR + fence amendment.
- Decision: M4 failure proofs (ep042_failure_*) use REAL failure mechanisms, never mocks: unavailable dependency (declared component with no artifact bytes -> UNAVAILABLE; real container termination in the gate -> UNREACHABLE/TIMEOUT), timeout (pre-aborted AbortController -> STAGING_FAILED, staged state removed), malformed input (corrupt manifest -> MANIFEST_INVALID), duplicate request (same install id re-install), denied permission (chattr +i on the staging root - real EACCES even as root), cancelled work (AbortController mid-install -> old state valid, no partial success), partial side effect (backup completed + install failed -> old state remains), backup failure (immutable backup root -> BACKUP_FAILED, update denied), staged digest mismatch, rollback (prior state restored + verified; missing/corrupt source denied), path traversal, symlink escape, duplicate overwrite, foreign-root cleanup, forged receipt (journal honesty), redaction canary, observability journal ladder. Evidence: fence I-L; tests all green 21/21; gate executes real installer scripts with cmp-verified bytes.
- Decision: installer scripts invoke the CLI under node --experimental-transform-types with a resolution-only ESM loader (installers/scripts/ts-resolve-loader.mjs) because Node native type-stripping cannot load the canonical @nexus/setup update core directly (extensionless relative imports + TS enums). The loader resolves extensionless specifiers to .ts/.tsx/index.ts without rewriting content; the CLI executes the REAL canonical code. Evidence: discovery documented; loader path must be absolute (Node treats a relative loader URL as a package); 12 unit proofs + 21 failure proofs + gate green.
- Decision: real container termination proof lives in the M4 gate (start SeaweedFS, publish fixture release, docker rm -f the container, prove fetch fails closed UNREACHABLE/TIMEOUT) rather than in the vitest suite, mirroring EP-040 M4. Evidence: gate output; fence CONTENT 2 (terminate a test container as a real mechanism).
- Security impact: redaction-first journal/evidence; runtime-constructed canaries; no tracked secret literals; chattr +i runtime only; no-placeholder scan clean; security-check green.
- License impact: none (no new third-party dependency; SeaweedFS Apache-2.0 already recorded).
- Compatibility impact: pnpm-workspace.yaml + pnpm-lock.yaml updated for @nexus/installers; tests/release gains workspace dep; M1/M2/M3 regressions stay green.

## 2026-08-25 - EP-042 M5 decisions

- Decision: M5 real offline bundle lives in offline-bundle/ as a new workspace package @nexus/offline-bundle (pnpm exact glob 'offline-bundle'; deps @nexus/setup + @nexus/installers workspace:*, zero third-party runtime deps). It implements REAL bundle production from REAL files (produce), digest-bound verification (verify: missing/changed/malformed/duplicate/traversal/symlink/wrong-release denied, manifest binding + bundle self-digest), OFFLINE install composing the M4 transactional installer with artifact bytes read from local bundle files only (no transport), rollback drill (receipt only after verified restoration; wrong backup source denied via canonical M4 verifyBackupDigest), and current-run redacted evidence (stale/tampered rejected). Evidence: M5 fence (offline-bundle/); ExecPlan M5 CONTENT 1-6; SPEC-016 behavior 5; SPEC-024; ADR-028 OfflineBundle vocabulary. Alternatives: extending installers/ (rejected - M4 owns the local installer surface; fence assigns offline-bundle/); a Python bundle tool (rejected - repo distribution stack is Rust+TS). Consequence: canonical release/update/install truth stays in M1/M2/M3/M4; offline-bundle/ is the offline distribution boundary. Reversal: ADR + fence amendment.
- Decision: bundle verification and the self-digest binding use the canonical strip-then-digest form (digest field EXCLUDED, never null) computed via the M2 contentDigest surface, and payload digests via the M2 sha256Hex surface. Evidence: M1 content_digest() semantics (strip-then-digest); real defects found by the bundle suite (double sha256: prefix from asString(); bundle_digest: null canonical form mismatch). Consequence: produce and verify agree on the same canonical bytes; a tampered bundle manifest always fails BUNDLE_SELF_DIGEST_MISMATCH.
- Decision: offline installation composes the REAL M4 installRelease; the bundle is the artifact source and the digest-bound component mapping matches component.digest to bundle item digest (no name guessing; a component absent from the bundle is denied BUNDLE_MISSING_FILE). Evidence: fence I/J (offline must actually mean offline); gate runs install with NO transport container and env; install result reports transport_required: false, source: local-bundle-only; cmp-verified installed bytes. Consequence: offline install provably never touches the M3 transport.
- Decision: the rollback drill verifies the backup source digest via the M4 verifyBackupDigest surface BEFORE invoking rollbackRelease (M4 records but does not verify expectedBackupDigest); a wrong/corrupt source is denied ROLLBACK_FAILED before any restore; the receipt is written only after exact prior bytes are verified restored. Evidence: fence M; drill test wrong-backup denied + success drill restores + receipt after verification; M4 surface untouched (closed milestone). Consequence: ROLLBACK RECEIPT EXISTS != ROLLBACK PROVEN holds at the drill layer.
- Decision: evidence is machine-readable JSON under .agent/state/evidence/ep042-m5/, bound to run_id + git_commit, redacted BEFORE the evidence digest (runtime canaries scrubbed), with stale/tampered/secret-shaped rejection in validateEvidence. Evidence: fence O/P; tests ep042_bundle_evidence_*; gate writes + validates current-run evidence. Consequence: no raw secrets ever enter the record; evidence self-digest binds the redacted content.
- Security impact: redaction-first evidence; runtime-constructed canaries; no tracked secret literals; no-placeholder scan clean; security-check green.
- License impact: none (no new third-party dependency).
- Compatibility impact: pnpm-workspace.yaml + pnpm-lock.yaml updated for @nexus/offline-bundle; tests/release gains workspace dep; M1/M2/M3/M4 regressions stay green.

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.

## EP-042 Outcomes (2026-08-25)

Node complete: EP-042 Deployment, Release, Update, and Rollback (SPEC-016, SPEC-024). M1 CLOSED `5837f57`, M2 CLOSED `2b61ade`, M3 CLOSED `1876ff9`, M4 CLOSED `03813e3`, M5 CLOSED (behavior `8212dd8` + build fix `29d0aa4` + gate provisioning `5414d97` + residue hardening `932e769`). FINAL_VERIFIED_COMMIT = `8b75264` (evidence refresh bound to 932e769); green/EP-042 -> 8b75264. Closure commit metadata-only (ledger NODE_DONE + ExecPlan outcomes).

Exact commands and observed sentinels: `sh scripts/nodes/EP-042.sh M1|M2|M3|M4|M5` each printed `EP-042 M<k>: ok` exit 0; `sh scripts/node-verify.sh EP-042` printed `node verify EP-042: ok` exit 0 on committed tree 8b75264 (expected-files EP-042 full list ok; verify: ok 15-gate ladder incl control-plane runtime smoke against 127.0.0.1:8443, EP-037 MinIO battery env from /tmp/nexus-battery-env.sh, EP-038 GlitchTip battery env from /tmp/ep038-verify-gt.env, live-fire ladder 440+ green suites 0 failed; node EP-042 verify: ok; M5 gate ok with real bundle produce/verify/offline-install/rollback/tamper/traversal denials + current-run redacted evidence). M1-M5 regressions green; side gates green: scope audit EP-042: ok, expected files EP-042: ok (full list), security check: ok (0 advisories), dependency audit: ok, license gate: ok, reality gate: ok, blueprint validation: ok, format check: ok, lint: ok, typecheck: ok, test-unit: ok.

Test and proof evidence: 76 ep042_unit_* M1 proofs + 99 ep042_unit_* M2 proofs + 14 ep042_integration_* M3 proofs (real SeaweedFS S3 gateway) + 17 transport units + 21 ep042_failure_* M4 proofs (real chattr EACCES, real container termination, real byte corruption) + 12 installer units + 19 ep042_bundle_* M5 proofs + 7 offline-bundle units; workspace battery 440+ green suites 0 failed on committed verify; M5 gate executes the real bundle-produce/verify/install/rollback scripts with cmp-verified bytes.

Real defects found and fixed during M5 closure: (1) tests/release/tsconfig.build.json duplicated compiler options without allowImportingTsExtensions -> TS5097 during workspace build; fixed by extending the base tsconfig (latent M3-era defect, commit 29d0aa4). (2) Canonical node verify composition: LF-029 (runtime-smoke live-fire) shuts the EP-044 control plane down before the M5 gate runs, making the gate unpassable by construction when it required a pre-running runtime; the M5 gate now provisions the runtime itself through canonical local-start and fails closed if it cannot be brought healthy (proven with runtime up AND with runtime down; commit 5414d97). (3) M5 gate temp-residue cleanup/check only covered FIXTURE_BASE/EVIDENCE_BASE globs, missing the per-proof nexus-ep042-m5-<label>-<ts>-<rand> vitest roots; hardened cleanup + fail-closed check to the whole owned glob (proven with pre-seeded residue; commit 932e769). (4) Two canonical-verify invocation defects (my environment setup, not code): battery env files must be sourced (MinIO /tmp/nexus-battery-env.sh + GlitchTip /tmp/ep038-verify-gt.env) and NEXUS_SMOKE_URL must be exported for the runtime smoke; after sourcing, verify went fully green. (5) Environment event: MinIO battery fixture OOM-killed twice (11GB RAM, no swap) - classified RESOURCE_EXHAUSTION, not code; mitigated by terminating ~10 stale tsserver/LSP processes (~5.3GB recovered); retained battery fixtures verified intact.

Assumptions confirmed: canonical battery env files retained at /tmp/nexus-battery-env.sh + /tmp/ep038-verify-gt.env (EP-040/EP-041 convention); NEXUS_SMOKE_URL=http://127.0.0.1:8443 for local control-plane smoke; foreign LF evidence churn (EP-031/033/035/037 run_ids rebound by live-fire reruns) reverted before each commit, never committed.

Provider and hardware status: real SeaweedFS 4.43 S3 gateway (digest-pinned) exercised in M3/M4 gates and battery; real MinIO exercised in EP-037 battery; control plane runtime (EP-044) exercised over real HTTP in smoke + M5 gate; no external cloud provider (AWS/R2/B2) contacted - transport to external clouds NOT ASSERTED.

Remaining risks: production host upgrade, real release-signature verification (no key store/verifier exists), production canary rollout, production backup/restore, production rollback, production deployment, offline production install, remote AWS/R2/B2 transport, arbitrary production environments - all NOT ASSERTED; NODE_DONE does not expand these certifications.

Green tag: green/EP-042 -> 8b75264 (FINAL_VERIFIED_COMMIT; the tree that emitted `node verify EP-042: ok` exit 0). Remote sync: deferred to closure push per repository convention; verify via ls-remote after push (no force-push).
