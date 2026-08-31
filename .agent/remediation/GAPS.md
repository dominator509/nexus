# RX-000 Gap Log - logged first, fixed one at a time, reported after each

## GAP-001 (RESOLVED) - Authoritative AUD-001...AUD-065 register was unavailable locally

**Status:** RESOLVED 2026-08-29
**Severity:** was BLOCKING

**Resolution:** Dominic provided the audit source: ChatGPT share
https://chatgpt.com/share/6a926876-0c84-83e8-a9da-4f3d53dd1ddc ("Audit Nexus Repository").
The full conversation was extracted (React Router single-flight stream decoded) and
all 90 findings imported verbatim into `register_data.py` / `AUDIT_FINDINGS.tsv`:

- AUD-001...006 from the master audit report
- AUD-007...012 compute-fabric continuation
- AUD-013...026 EP-037 storage/DR + communications + Sentinel continuation
- AUD-027...041 EP-030/031 Sentinel + client continuation
- AUD-042...065 setup/bootstrap + storage + observability + supply-chain + EP-040/041 continuation
- AUD-066...090 EP-042 update path / EP-043 / EP-044 continuation

Cumulative severities match the audit exactly: P0 0, P1 72, P2 18 (total 90).
Repair-node ownership: sec.12 of the remediation graph (AUD-066...090) + RX-node
ownership language (AUD-001...065). All rows OPEN; verifier green.

**Verifier:** `.agent/remediation/verify-remediation-register.sh` -> PASS
(90/90 registered, quarantine active: generation 2, release not allowed).

## GAP-002 (RX-005) - EP-004/EP-005/EP-006 persistence and retry truth (AUD-007, AUD-008, AUD-023)

**Status:** LOGGED 2026-08-30 - fix ONE at a time from top severity; report after each.

### GAP-002a (AUD-007, P1) - EP-004 closed without production PostgreSQL repository / UnitOfWork / pgvector adapters
- Only `MemoryRepository` / `WorldGraphRepository` / `VectorRepository` / `UnitOfWork` traits exist in `nexus-data`.
- No concrete PostgreSQL implementations anywhere; `integration_postgres.rs` drives raw SQL, bypassing production abstractions.
- Tenant isolation is application convention, not DB RLS (`memory_records`/`world_graph_edges` have no `ENABLE ROW LEVEL SECURITY`).
- `memory_embeddings` FK binds only `memory_id`; tenant_id is not part of a composite FK to `memory_records`.

### GAP-002b (AUD-008, P1) - EP-005 NATS checkpoint persistence is a no-op; outbox/inbox absent

**NATS consumer portion: RESOLVED 2026-08-30 (VERIFIED_FIXED, commit pending)**
- `checkpoint()`/`save_checkpoint()` now persist to a real JetStream KV bucket
  (`nexus_checkpoints`, keyed by consumer name) - durable, survives restart;
  `checkpoint()` reads the stored checkpoint, `save_checkpoint()` writes it.
- `poll()` now creates an EPHEMERAL per-call pull consumer positioned by the
  application-owned `after_sequence` (DeliverAll for 0). The pre-fix durable
  consumer per sequence (`{consumer}-{after_sequence}`) accumulated unbounded
  server-side state and defeated durability; a single stable durable consumer
  would track its own position and ignore the checkpoint, so it is avoided by
  design (at-least-once + inbox dedup, matching SPEC-023 behavior 4).
- Proof (live-fire `nats:2.14.3`): integration + failure suites 19/19; new
  tests prove checkpoint round-trip equality, overwrite advance, persistence
  across a fresh connection, None for unsaved consumers, and resume-after-
  checkpoint skipping processed events.

**Still LOGGED - GAP-002b2 (AUD-008 remainder): Outbox/Inbox PostgreSQL implementations absent**
- `OutboxRepository` / `InboxRepository` ports exist in `nexus-events`
  (SPEC-023 behaviors 1 & 4); no PostgreSQL implementations anywhere.
- The restart proof now exercises the real checkpoint (no manual sequence
  fiddling), but transactional outbox/inbox persistence in `nexus-pg` is a
  distinct piece of work - scope decision with Dominic before implementation.

**GAP-002b2: RESOLVED 2026-08-30 (VERIFIED_FIXED, commit pending)**
- Port corrected: `OutboxRepository::append` dropped its unimplementable
  `&mut dyn UnitOfWork` parameter (the trait exposes no statement
  execution; no caller existed). Atomicity is expressed by binding the
  repository to the same `PgUnitOfWork` as the domain repositories.
- Migration 004: `outbox` + `inbox` tables (idempotent, status CHECK
  constraints, scan indexes). Platform-level ledgers - deliberately no
  tenant RLS (publisher scan is cross-tenant by design).
- `PgOutboxRepository`: append (PENDING), fetch_pending (PENDING+FAILED,
  oldest first, in-flight PUBLISHING excluded), mark_publishing/
  mark_published/mark_failed (idempotent per row, Conflict on missing,
  attempts incremented on failure - bounded retry).
- `PgInboxRepository`: record_delivery deduplicates via ON CONFLICT DO
  NOTHING (first sighting true, replay false), mark_done/mark_failed,
  fetch_new (NEW+FAILED per consumer).
- `PgUnitOfWork::with_tx` generalized over the closure error type
  (`E: From<DataError>`); `From<DataError> for EventError` added to
  nexus-events preserving the SPEC-006 code ladder and correlation.
- Proof (live-fire `pgvector/pgvector:pg18`): atomicity both ways (domain
  write + append commit together, roll back together), publisher lifecycle
  (pending -> publishing excluded -> published; failed retried with
  attempts), inbox dedup + lifecycle + consumer isolation, migration
  idempotency covers the new tables. nexus-pg 14/14; data+events+pg
  52/52; workspace check clean (0 errors, 0 warnings).

### GAP-002c (AUD-023, P2) - Temporal does not enforce permanent/transient retry classification

**RESOLVED 2026-08-30 (VERIFIED_FIXED, commit pending)**
- `ERROR_CODE_CLASS` added to `@nexus/workflows`: canonical SPEC-006 code ->
  retry class (PERMANENT for VALIDATION/AUTHENTICATION/AUTHORIZATION/POLICY/
  EXTERNAL_PROVIDER/VERIFICATION/COMPENSATION/INTERNAL_INVARIANT;
  UNAVAILABLE/TIMEOUT/RATE_LIMIT/TRANSIENT(CONFLICT) otherwise).
  `NexusWorkflowError.isRetryable()` now derives from it - one source of
  truth, no drift.
- `toTemporalRetry` now supplies `nonRetryableErrorTypes` derived from the
  policy: every SPEC-006 code whose retry class is NOT in
  `retryableErrorClasses` is declared non-retryable. Under the default
  policy VALIDATION/POLICY/AUTH can never consume the five attempts.
- New `failure.ts` classifier + `NexusFailureInterceptor` wired into the
  activity worker: `NexusWorkflowError` is rethrown at the activity
  boundary as `ApplicationFailure` with type = SPEC-006 code and
  `nonRetryable` per the code's class, so both the failure itself and the
  policy agree.
- Proof: temporal 74/74 (8 retry mapping tests incl. permanent/narrow/
  empty-class classification, 5 failure-classifier tests), workflows
  109/109 after the errors.ts refactor, typecheck exit 0.

## GAP-002d (RX-005, CORRECTION from Dominic 2026-08-30) - AUD-023 "VERIFIED_FIXED" rests on a test double, not a real boundary

**Status:** LOGGED 2026-08-30 - fix ONE at a time from top severity; report after each.

**Severity:** P1 (gate fired green on a shell; TESTING.md line 36 violated)

### Findings (audit, every claim verified against the tree)

1. **The interceptor is proven only with a hand-rolled `next()` double.**
   `infra/temporal/src/__tests__/ep006_unit_failure.test.ts` lines 25-49:
   the SDK call chain is simulated by `const next = async () => { throw new
   NexusWorkflowError(...) }`. This proves the interceptor's try/catch
   logic, NOT that it works at a real Temporal activity boundary.
   The test file itself admits it: "Test double for the SDK call chain".

2. **No real Temporal server exercises the AUD-023 classification path.**
   The real-server suite (`tests/workflows/` - real temporalio/server:1.31.2
   + postgres:18.4 containers) exists and is genuinely live-fire for
   approval/binding/restart, but:
   - `grep -rn "throw new NexusWorkflowError" tests/workflows/` -> ZERO hits
   - No activity in the real suite throws a NexusWorkflowError, so the
     NexusFailureInterceptor is never exercised at the real boundary.
   - `grep -rn "NexusWorkflowError" tests/workflows/src/` matches only
     stack/worker/session helpers and restart/compensation tests that do
     NOT throw typed failures through an activity.

3. **The RX-005 battery never runs the real-server suite.**
   `scripts/rx005-remediation-tests.sh` AUD-023 section runs only
   `pnpm --filter @nexus/temporal test:unit` (unit tests with the double)
   and `pnpm --filter @nexus/workflows test:unit`. The real Temporal
   server suite is absent from the battery, so the closure tag
   `green-v2/RX-005/7d50efe` attested "8/8" without ever connecting to a
   live Temporal server for the retry classification claim.

4. **TESTING.md line 36 is the law and was not met.** "Temporal tests
   include the official test environment and at least one real server
   E2E." `@temporalio/testing` is installed as a devDependency but
   `TestWorkflowEnvironment` appears NOWHERE in source - the official
   test environment is present in package.json and unused.

5. **The register row is misleading.** AUD-023 marked VERIFIED_FIXED with
   evidence "temporal 74/74" - but 74 unit tests with a `next()` double
   do not prove the boundary classification that the finding describes
   ("non-retryable ApplicationFailure is never supplied").

**FIX IN PROGRESS (2026-08-30):**
- Added `tests/workflows/src/ep006_failure_retry_classification_real.test.ts`:
  REAL boundary proof - a runEffect activity throws NexusWorkflowError
  through the real worker+interceptor against temporalio/server:1.31.2.
  - PERMANENT (POLICY): surfaced to client as type=POLICY,
    nonRetryable=true, and attempted EXACTLY ONCE (the AUD-023 claim:
    before the fix, permanent failures burned five attempts).
  - TRANSIENT (UNAVAILABLE): retried by the real engine (attempts=2),
    workflow completes SUCCEEDED.
  - 2/2 green against the real server (33s).
- Fixed `tests/workflows/package.json` test:integration: removed the
  `-t ep006_integration` filter that was SKIPPING all ep006_failure_*
  real-server tests (8 tests were silently not running). Full suite now
  10 files / 20 tests green (93s).
- Wired the real-server suite into scripts/rx005-remediation-tests.sh as
  a distinct AUD-023 gate section (TESTING.md line 36 compliance).

**OFFICIAL ENVIRONMENT PROOF ADDED (2026-08-30):**
- TESTING.md line 36 requires BOTH the official test environment AND a
  real server E2E. Added
  `infra/temporal/src/__tests__/ep006_official_environment_interceptor.test.ts`:
  TestWorkflowEnvironment.createLocal() launches a REAL Temporal server
  binary; the REAL worker + REAL NexusFailureInterceptor + REAL activity
  throwing NexusWorkflowError cross real gRPC. Asserts the client-visible
  failure is ApplicationFailure(type=POLICY, nonRetryable=true) and the
  permanent activity was attempted exactly once. 1/1 green (1.7s).
- New vitest.config.ts for @nexus/temporal (fileParallelism: false,
  180s timeouts) so the official-environment test coexists with the unit
  suite. Full temporal suite now 12 files / 75 tests green.

**AUD-012 REMEDIATED (RX-006, 2026-08-31):**
- Root cause: register_node() ignored the supplied WireGuard public key
  and synthesized an unrelated random mkey; wireguard_config() fabricated
  openbao:mesh/{tenant}/{node_id} with no code creating/storing that key;
  the live proof used placeholder keys.
- Fix: register_node now binds the caller-supplied key (32-byte hex,
  mkey: optional), verifies the provider round-trip (StateConflict on
  mismatch), and rejects placeholder/empty keys before any provider call.
  wireguard_config now resolves the private-key reference through a real
  SecretStore (fail-fast: no store -> StateConflict; unresolvable ->
  NotFound); it never fabricates a reference.
- Proof: mesh_live_proof generates REAL X25519 keypairs (openssl), stores
  the private key in a REAL OpenBao dev container under the REAL headscale
  node id, registers the derived public key, resolves the reference, and
  asserts cryptographic binding (stored mesh_key == registered identity).
- Battery: scripts/rx006-remediation-tests.sh 8/8 green (unit 18, hostile
  6, nexus-trust 17, real integration 10, orphan audit, workspace, clippy,
  register 90/90). Register AUD-012 -> VERIFIED_FIXED.

**AUD-011 REMEDIATED (RX-007, 2026-08-31):**
- Root cause: SkillExecutor verified the signed package and declared
  permissions, then executed with Command::new. No filesystem/namespace/
  seccomp isolation, privilege drop, or network enforcement; permissions
  were env vars a hostile payload could ignore.
- Fix: on Linux the subprocess is a REAL OS sandbox in pre_exec:
  unshare(CLONE_NEWNS|CLONE_NEWNET|CLONE_NEWIPC|CLONE_NEWUTS); private
  mount tree; bounded tmpfs at /tmp (only writable location); /, /proc,
  /sys remounted read-only; payload materialized inside the sandbox; real
  setgroups/setgid/setuid drop to uid/gid 65534; PR_SET_NO_NEW_PRIVS; a
  seccomp BPF deny-list filter (mount/umount2/ptrace/kexec*/module-load/
  power-cycle/swap/identity/keyring/bpf/process_vm_*/perf/io_uring/open_by_
  handle/chroot/pivot_root/setns/unshare -> EPERM, default allow). Any
  sandbox step failure makes spawn fail closed - the skill is never
  executed unsandboxed on Linux.
- Proof: rx007 hostile payloads probe the sandbox from the inside - uid
  65534, host writes fail while /tmp tmpfs writable, /proc/net/dev shows
  loopback only (no host iface), Seccomp: 2 + NoNewPrivs: 1.
- Battery: scripts/rx007-remediation-tests.sh 6/6 green. Register
  AUD-011 -> VERIFIED_FIXED.

**AUD-022 REMEDIATED (RX-007, 2026-08-31):**
- Root cause: stdout drained synchronously to EOF before stderr, no
  execution timeout; a child filling stderr while keeping stdout open
  blocked itself while the parent waited forever. ProcessRunner repeated
  the pattern.
- Fix: SkillExecutor and ProcessRunner now drain stdout and stderr
  CONCURRENTLY over channel-backed reader threads; a wall-clock deadline
  (SKILL_EXEC_TIMEOUT / PROCESS_EXEC_TIMEOUT, overridable via
  with_timeout) kills the process group (SkillExecutor) or the direct
  child (ProcessRunner, safe-only crate) on expiry; the result is
  observable as timed_out / HarnessExitStatus::Timeout - never a
  fabricated success; a 2s bounded receive grace prevents a pipe-
  inheriting grandchild from hanging the runner.
- Proof: rx007 stderr-flood payloads (360KB > 64KB pipe buffer x several)
  complete normally; hung payloads are killed at the deadline.
- Battery: scripts/rx007-remediation-tests.sh 6/6 green. Register
  AUD-022 -> VERIFIED_FIXED.

**AUD-083 REMEDIATED (RX-008, 2026-08-31):**
- Root cause: EP-044 main.rs initialized no telemetry at startup; the
  crate had no OpenTelemetry/tracing dependency; the M4 'telemetry'
  test only checked that stdout did not expose a tenant identifier.
- Fix: RuntimeTelemetry::init builds a validated
  nexus_observability::TelemetryContext (component nexus-control-plane,
  node, environment, operation=startup) and startup_line() emits a
  structured log line through the REAL nexus-otel export boundary
  (export_structured_log), which re-verifies
  RedactedEnvelope::assert_exportable() before any byte is produced.
  main.rs initializes telemetry at startup and prints the startup
  line. Unit regressions prove context validity, empty-component
  fail-closed, structured exportable line, and that the tenant id
  never leaks.
- Battery: scripts/rx008-remediation-tests.sh green. Register
  AUD-083 -> VERIFIED_FIXED.

**AUD-084 REMEDIATED (RX-008, 2026-08-31):**
- Root cause: main.rs hard-coded only health and capabilities; the
  router exposed only /healthz, /readyz, /v1/capabilities; the
  REST/MCP/A2A/auth/query/command/workflow/event/artifact surfaces
  were not composed despite SPEC-003.
- Fix: the runtime is now the APPLICATION COMPOSITION ROOT -
  RuntimeComposition composes the REAL InMemoryCapabilityRegistry
  (runtime.health + runtime.capabilities descriptors), the REAL
  CapabilityDispatcher, the REAL McpEngine with a real tool registry,
  the REAL A2AGatewayImpl with a hash-bound MemoryArtifactStore, and
  the REAL MemoryOutbox. The server router exposes the SPEC-003
  surfaces over HTTP (/v1/discover, /v1/mcp/initialize|tools|call,
  /v1/a2a/tasks + run + stream, /v1/artifacts publish/fetch,
  /v1/events append/pending), every handler driving a real engine.
  A2AGatewayImpl's artifact port gained Send+Sync so the composition
  satisfies axum State bounds.
- Proof: ep044 integration drives the REAL binary over REAL HTTP -
  discover, MCP init/list/call, A2A submit/run/stream, artifact
  publish/fetch (hash-bound; fabricated id 404), events append/pending
  - all green.
- Battery: scripts/rx008-remediation-tests.sh green. Register
  AUD-084 -> VERIFIED_FIXED; AUD-054 -> FIXED_UNVERIFIED (co-owned
  with RX-019, which verifies OTel export/Grafana/Prometheus).
**AUD-059 REMEDIATED (RX-009, 2026-08-31):**
- Root cause: the tamper seal was a SHA-256 checksum stored beside the
  evidence and recomputed during verification; the ArtifactSigner trait
  had no implementation. Anyone able to change evidence could change its
  checksum - the seal proved nothing.
- Fix: nexus-supply-chain now has a REAL Ed25519 ArtifactSigner (ring
  0.17, already in the workspace lock - no new dependency class):
  keygen, deterministic RFC 8032 signing, and fail-closed verification
  with typed SignatureInvalid for any tamper/wrong-key/short-key.
  scripts/sbom/generate.sh seals evidence with evidence.json.sig +
  evidence.json.pub; verify.sh cryptographically verifies with typed
  SIGNATURE_MISSING / SIGNATURE_INVALID classes and a pinned-key mode
  that refuses a wholesale evidence+sig+pubkey swap.
- Proof: forced-failures proves tamper-and-RESEAL is still rejected
  (SIGNATURE_INVALID with seal_matches=true, signature_verified=false) -
  the cryptographic signature, not the checksum, is what catches
  tampering. Legitimate-denial fixtures are re-signed so their intended
  class fires.
- Battery: scripts/rx009-remediation-tests.sh green. Register
  AUD-059 -> VERIFIED_FIXED.

**AUD-060 REMEDIATED (RX-009, 2026-08-31):**
- Root cause: the certified SBOM inventory was Cargo-centric only,
  although the shipped product contains pnpm/TypeScript, Flutter/Dart,
  images, and model/data artifacts.
- Fix: sbom_ecosystems adapter inventories the REAL repository state:
  pnpm-lock.yaml (378 TypeScript packages), every pubspec.lock (137
  Dart packages across 5 lockfiles), models/, tests/data, and app
  images. Fail-closed on missing/malformed lockfiles. ecosystems.json
  carries per-ecosystem counts + package lists + artifact inventory and
  is cryptographically signed like the main evidence; verify.sh verifies
  it with typed ECOSYSTEMS_MISSING / ECOSYSTEMS_STALE /
  ECOSYSTEMS_SIGNATURE_INVALID classes, bound to run_id + git_commit.
- Proof: hostile case - tampered ecosystem counts are rejected even
  when the Cargo evidence is untouched.
- Battery: scripts/rx009-remediation-tests.sh green. Register
  AUD-060 -> VERIFIED_FIXED.

**AUD-065 REMEDIATED (RX-009, 2026-08-31):**
- Root cause: verifyBundle proved file hashes, manifest digest and
  bundle self-digest but never cryptographically checked
  SignedComponent.signature; the M5 gate shipped AAAA01BBBB01 dummy
  signatures and expected VERIFIED. No signature verifier existed.
- Fix: verifyBundle now imports the bundle's Ed25519 public key
  (signing-key.pub.jwk carried in the bundle root) and cryptographically
  verifies EVERY component signature over its canonical artifact digest
  (Node WebCrypto; signing side is crypto.sign(null, msg, privateKey),
  cross-verified live). New typed classes SIGNING_KEY_MISSING /
  SIGNATURE_MISSING / SIGNATURE_INVALID fail closed. The M5 gate now
  produces REAL signatures with a real keypair; evidence
  signature-state is SIGNATURE_VERIFIED_ED25519; boundary.txt asserts
  real Ed25519 signature verification, proven.
- Proof: three hostile proofs - dummy/placeholder signature fails
  (SIGNATURE_INVALID), signature from a wrong key fails
  (SIGNATURE_INVALID), missing signing key fails (SIGNING_KEY_MISSING);
  the vitest bundle suite (19 proofs) runs with real signatures.
- Battery: scripts/rx009-remediation-tests.sh green. Register
  AUD-065 -> VERIFIED_FIXED.
**AUD-076 REMEDIATED (RX-010, 2026-08-31):**
- Root cause: collectReleaseTag() read .git/HEAD verbatim and the
  readiness gate only required a nonempty string; a branch pointer
  (ref: refs/heads/master) satisfied the release-tag obligation.
- Fix: collectReleaseTag now requires HEAD to resolve to a refs/tags/*
  ref. Branch pointers and detached commits fail closed to "" so the
  readiness release-tag obligation can never be met by merely being on
  a branch.
- Proof: unit proofs - tag ref accepted, branch pointer rejected,
  detached commit rejected, missing .git fails closed.
- Battery: scripts/rx010-remediation-tests.sh green. Register
  AUD-076 -> VERIFIED_FIXED.

**AUD-077 REMEDIATED (RX-010, 2026-08-31):**
- Root cause: digestBytes() decoded arbitrary binary artifact bytes
  with TextDecoder (lossy UTF-8) before hashing; distinct binary
  sequences collapsed onto the same U+FFFD replacement characters and
  produced identical digests.
- Fix: new sha256Bytes() hashes the Uint8Array directly (pure-JS
  FIPS 180-4 over raw bytes); digestBytes returns
  sha256:<sha256Bytes(bytes)> with no decode round-trip. String
  hashing (sha256HexSync) delegates to the same bytes core.
- Proof: hostile - [0x61,0xff] and [0x61,0xfe] now produce distinct
  digests; known FIPS vector sha256("hello") asserted.
- Battery: scripts/rx010-remediation-tests.sh green. Register
  AUD-077 -> VERIFIED_FIXED.

**AUD-078 REMEDIATED (RX-010, 2026-08-31):**
- Root cause: canonicalManifestPayload() used
  JSON.stringify(manifest, Object.keys(manifest).sort()); a replacer
  ARRAY is applied recursively, so every nested component property
  (identity, digest, signature, SBOM, artifact ref) was discarded from
  the serialized digest input. Re-verification repeated the broken
  algorithm.
- Fix: canonicalize() recursively key-sorts at every nesting level and
  canonicalManifestPayload serializes the canonicalized payload, so
  the manifest digest cryptographically binds ALL nested component
  state.
- Proof: hostile - swapping component artifact bytes (nested digest)
  and tampering the nested signature value both change the manifest
  digest (invisible under the old serialization).
- Battery: scripts/rx010-remediation-tests.sh green. Register
  AUD-078 -> VERIFIED_FIXED.

**AUD-079 REMEDIATED (RX-010, 2026-08-31):**
- Root cause: canonicalEvidenceDigest() had the same top-level replacer
  flaw; nested certification state, evidence refs, drill status/
  timestamps, review verdicts/evidence and capability-status values
  were not cryptographically bound, and tests reproduced the flawed
  serialization.
- Fix: canonicalEvidenceDigest() serializes canonicalize(payload) -
  recursive key sorting binds every nested field.
- Proof: hostile - nested drill status flip, nested review verdict
  flip, and nested capability-status flip all change the evidence
  digest; the reproduction test now asserts the canonical form.
- Battery: scripts/rx010-remediation-tests.sh green. Register
  AUD-079 -> VERIFIED_FIXED.

**AUD-066 REMEDIATED (RX-011, 2026-08-31):**
- Root cause: rollbackRelease() checked only that the backup directory
  existed/nonempty (assertBackupUsable), restored it, then copied the
  caller's expectedBackupDigest directly into a verified: VERIFIED
  receipt. verifyBackupDigest() existed but was never called.
- Fix: rollbackRelease now verifies the backup source digest against
  the REAL backup bytes (verifyBackupDigest) BEFORE any restore. A
  wrong/corrupt backup is denied with ROLLBACK_FAILED; the caller's
  digest is never copied into a VERIFIED receipt without proof.
- Proof: hostile - a non-empty backup whose content does not match the
  declared digest is denied (ROLLBACK_FAILED) and the live install is
  untouched (no restore happens).
- Battery: scripts/rx011-remediation-tests.sh green. Register
  AUD-066 -> VERIFIED_FIXED.

**AUD-067 REMEDIATED (RX-011, 2026-08-31):**
- Root cause: the atomic switch recursively deleted installRoot and
  only afterward called renameSync(stagingRoot, installRoot). A
  rename/mount/filesystem failure left the live installation deleted;
  the code explicitly did not auto-rollback.
- Fix: the switch now preserves the current install by renaming it
  aside (rename, not delete) before committing the staged state. If the
  commit rename fails, the preserved install is restored at its
  original path. The preserved install is removed only after the new
  state is verified.
- Proof: hostile - staging on a different filesystem (/dev/shm tmpfs
  vs /tmp) forces a real EXDEV commit failure; the old install survives
  with its original bytes at the original path.
- Battery: scripts/rx011-remediation-tests.sh green. Register
  AUD-067 -> VERIFIED_FIXED.

**AUD-068 REMEDIATED (RX-011, 2026-08-31):**
- Root cause: staged bytes were validated against caller-controlled
  InstallComponent.declaredDigest, not the manifest digest; extra
  opts.components not declared by the manifest were staged; opts.
  releaseId was not bound to manifest.release_id.
- Fix: installRelease now binds the request to the validated release
  manifest - opts.releaseId must equal manifest.release_id, every
  supplied component must be declared by the manifest, and every
  supplied declaredDigest must equal the manifest's digest for that
  component. Staged bytes are validated against these manifest-bound
  digests; violations are denied with MANIFEST_INVALID before any
  mutation.
- Proof: hostile - unbound component digest, extra undeclared
  component, and release-id mismatch all fail closed with
  MANIFEST_INVALID and no filesystem mutation.
- Battery: scripts/rx011-remediation-tests.sh green. Register
  AUD-068 -> VERIFIED_FIXED.

**AUD-069 REMEDIATED (RX-011, 2026-08-31):**
- Root cause: installRelease() unconditionally reset its journal at
  entry; there was no completed-install lookup, install-ID ownership
  check, or replay refusal before filesystem mutation.
- Fix: installRelease now reads the journal BEFORE any mutation. A
  completed install (last state INSTALLED) for the same install_id
  refuses replay; a journal owned by a different install_id refuses the
  request (AUTHORIZATION_DENIED) instead of being reset/overwritten.
- Proof: hostile - replay of a completed install is refused with the
  installed state untouched; a foreign journal owner is refused.
- Battery: scripts/rx011-remediation-tests.sh green. Register
  AUD-069 -> VERIFIED_FIXED.
