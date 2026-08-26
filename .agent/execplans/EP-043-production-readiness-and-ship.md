NODE-META-BEGIN
ID: EP-043
DEPS: EP-042
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-043
VERIFY_SENTINEL: node verify EP-043: ok
GREEN_TAG: green/EP-043
NODE-META-END

# 1. Purpose / Big Picture

Execute all live-fire proofs, security and privacy review, load and hardware certification, restore and rollback drills, docs audit, release tag, and manual deploy handoff. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-043.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-043.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-042` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-043.md`
- `.agent/specs/SPEC-008-production-readiness-certification-and-ship-standard.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-043.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-043-production-readiness-and-ship.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-043.txt`
- `.agent/node-contracts/EP-043.md`
- `scripts/nodes/EP-043.sh`
- `release-evidence/`
- `PRODUCTION_READINESS.md`
- `OPERATIONS.md`
- `RELEASE.md`
- `ROLLBACK.md`
- `.agent/state/`
- `dist/release/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `ShipGate` | `all` | Defined by EP-043; provider-neutral and versioned |
| `ReleaseEvidence` | `all` | Defined by EP-043; provider-neutral and versioned |
| `ManualDeployHandoff` | `all` | Defined by EP-043; provider-neutral and versioned |
| `ProductionReadinessDecision` | `all` | Defined by EP-043; provider-neutral and versioned |

Acceptance obligations:

1. All graph nodes are DONE
2. All twenty-eight live-fire proofs pass
3. Required provider and hardware certification rows are signed
4. Security, privacy, performance, accessibility, observability, backup, restore, update, and rollback reviews pass
5. A release tag and exact manual deploy command are produced without deploying production

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for execute all live-fire proofs, security and privacy review, load and hardware certification, restore and rollback drills, docs audit, release tag, and manual deploy handoff.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-043-M1.txt`, `.agent/node-contracts/EP-043.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-043-production-readiness-and-ship.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-043.txt`, `.agent/node-contracts/EP-043.md`, `scripts/nodes/EP-043.sh`, `release-evidence/`, `.agent/state/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep043_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-043.sh M1`

EXPECT:

- `EP-043 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-043 MILESTONE_PASS "M1 EP-043 M1: ok"`

FALLBACK: Issue a release candidate with an explicit blocking list; do not mark it shippable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-043][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-043.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-043-M2.txt`, `.agent/node-contracts/EP-043.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `PRODUCTION_READINESS.md`, `dist/release/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep043_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-043.sh M2`

EXPECT:

- `EP-043 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-043 MILESTONE_PASS "M2 EP-043 M2: ok"`

FALLBACK: Issue a release candidate with an explicit blocking list; do not mark it shippable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-043][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-043 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-043-M3.txt`, `.agent/node-contracts/EP-043.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `OPERATIONS.md`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep043_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-043.sh M3`

EXPECT:

- `EP-043 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-043 MILESTONE_PASS "M3 EP-043 M3: ok"`

FALLBACK: Issue a release candidate with an explicit blocking list; do not mark it shippable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-043][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-043 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-043-M4.txt`, `.agent/node-contracts/EP-043.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `RELEASE.md`

CONTENT:

1. Create tests whose names begin `ep043_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-043.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-043 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-043 MILESTONE_PASS "M4 EP-043 M4: ok"`

FALLBACK: Issue a release candidate with an explicit blocking list; do not mark it shippable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-043][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-043.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-043-M5.txt`, `.agent/node-contracts/EP-043.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `ROLLBACK.md`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-043` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-043.sh M5`
2. `sh scripts/node-verify.sh EP-043`
3. `sh scripts/scope-audit.sh EP-043`

EXPECT:

- `EP-043 M5: ok`
- `node verify EP-043: ok`
- `scope audit EP-043: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-043 MILESTONE_PASS "M5 EP-043 M5: ok"`

FALLBACK: Issue a release candidate with an explicit blocking list; do not mark it shippable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-043][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-043` and observe `node verify EP-043: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

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

# 12. Surprises & Discoveries

- 2026-08-25: M1 gate regression strategy. The EP-042 M5 gate carries a
  node-fenced scope audit that cannot pass once EP-043 files exist (the
  audit compares against EP-042's own expected-files). Correct regression
  mechanism is the predecessor's owned test surfaces (cargo test
  -p nexus-release + tests/release vitest), not the predecessor's full
  gate. Recorded in gate header.
- 2026-08-25: Prettier + blueprint ASCII rules apply to the new package;
  em-dashes in model.ts comments tripped blueprint validation. Fixed.
- 2026-08-25: lint.sh resolves flutter via mise shims only after sourcing
  scripts/env.sh; direct `sh scripts/lint.sh` from a bare shell fails on
  `flutter: not found`. Canonical verify sources env.sh first (verify.sh
  does this).

- 2026-08-26: M3 real defect - absolute output path mangling. The M2
  CLI used path.join(root, output) for --output/--output-dir/--manifest;
  join() treats an absolute second argument as a relative segment, so
  operators passing absolute paths (e.g. --output /tmp/x/PR.md) got
  /root/nexus/tmp/x/PR.md. The M3 integration suite exposed it (first
  run: 7 failures, ENOENT). Fixed all three sites to path.resolve()
  (absolute wins). M2 gate never caught it because it used relative
  paths only.
- 2026-08-26: M3 test-infrastructure defect - vitest NODE_OPTIONS leak.
  Integration tests spawn the real CLI via execFile/spawn; vitest
  injects its own loader through NODE_OPTIONS, which the child node
  process inherited and conflicted with the CLI's --import loader
  (exit 1). Fixed by passing env with NODE_OPTIONS="" to child
  processes. Same pattern required for the cancellation proof (spawn).
- 2026-08-26: M3 residue lesson - pre-fix integration runs left parallel
  dirs under <repo>/tmp/<mkdtemp-name> because the join() bug made the
  CLI create output dirs at the mangled path while the test afterEach
  removed only the /tmp originals. Post-fix runs leave zero residue.
  Removed the stale tmp/ tree; format/scope gates green after cleanup.
  This is the same whole-owned-glob residue class as EP-042 M5.
- 2026-08-26: mktemp/os.tmpdir() probe: with TMPDIR unset, both land in
  /tmp in this environment; the repo tmp/ debris was purely the
  join()-bug artifact, not an environment TMPDIR override.
- 2026-08-26: GLOBAL_VERIFY_DEFECT (composition) - node-verify.sh runs
  verify.sh twice (once directly, once via scripts/nodes/EP-043.sh
  verify). LF-029 (EP-044's live-fire proof) starts the control plane,
  asserts it, then shuts it down gracefully. Now that EP-044 is
  at-least, the runtime smoke stage is mandatory and fails closed; the
  second verify.sh therefore failed with the plane down (VERIFY_EXIT=4,
  curl: Failed to connect 127.0.0.1:8443 after reality-gate). Same
  defect class EP-042 fixed in its M5 gate. Fix (EP-043-owned):
  scripts/nodes/EP-043.sh verify branch provisions the runtime via
  canonical local-start when unhealthy and re-smokes, mirroring the
  EP-037/EP-038 fixture-provisioning pattern; fails closed if the plane
  cannot be brought healthy. Not an EP-043 code regression - the
  readiness/manifest/operations surface was already green in the first
  ladder pass.

# 13. Decision Log

- 2026-08-25: M1 package language/layer. release-evidence/ is a TypeScript
  workspace package (@nexus/release-evidence) mirroring the EP-042 release
  chain convention (infra/release, installers, offline-bundle). Evidence:
  pnpm-workspace.yaml registration + tsconfig/build conventions identical
  to offline-bundle. Alternatives: Rust crate (rejected: the ship boundary
  composes TS release surfaces), Python (rejected: no ship tooling in
  Python). Consequence: zero new third-party deps; Web Crypto-compatible
  sha256; vitest proofs. Reversal: new ADR.
- 2026-08-25: M2 readiness engine. M2 implements the five acceptance
  obligations as a pure deterministic evaluation (readiness.ts) fed by a
  real repository adapter (repo-state.ts reading GRAPH.md, LEDGER,
  live-fire registry, certification RESULTS.md, evidence dir). I/O moves
  behind the adapter; pure domain stays node-free. The honest current
  verdict is NOT_READY (EP-043 not DONE, certification rows
  RELEASE-BLOCKING-PENDING, no fresh-clone rerun) and the report states
  exactly that with blocking reasons. Alternatives: a mocked "ready"
  demo (rejected: fabrication), a Rust CLI (rejected: ship boundary
  composes TS surfaces). Reversal: new ADR.
- 2026-08-25: M2 release manifest. dist/release/RELEASE_MANIFEST.json is
  produced by the manifest CLI from real fixture component bytes with
  real sha256 digests, strip-then-digest manifest digest (canonical
  EP-042 M1 shape), and honest SIGNATURE_PRESENT_NOT_VERIFIED markers
  (no key store/verifier exists; signatures are never fabricated).
- 2026-08-25: Dependency-direction evolution. M1's pure-contract scan
  forbade node imports across all of release-evidence/src; M2 adds I/O
  adapters (repo-state, cli) that legitimately use node builtins. The
  proof now classifies modules: pure domain (errors/model/readiness/
  manifest/report) node-free; adapters (repo-state/cli) node-allowed but
  provider-SDK-free. M1 gate scan updated to the same classification.
  This is the milestone's "keep domain rules pure and move I/O behind
  ports" requirement, not a gate weakening.
- 2026-08-25: PRODUCTION_READINESS.md is generated by the readiness CLI
  and added to .prettierignore (generated report, like dist/); the
  format gate validates first-party code, not generated output.
- 2026-08-25: vitest default reporter does not list individual test
  names in NO_COLOR mode; M2 gate uses --reporter=verbose so the
  anti-masking sentinels observe real test names in the log.
- 2026-08-25: RESOURCE_EXHAUSTION (recurring lesson). Host disk hit 100%
  (1.1G free) during the M2 verify run; MinIO returned 500 InternalError
  on battery buckets - the exact EP-041 docker-full fingerprint. NOT an
  EP-043 code regression. Remediated with `docker volume prune -f`
  (58.5GB reclaimed, 1478 -> 13 volumes; shared battery fixtures
  retained and verified intact: MinIO 403-alive, GlitchTip 200). Disk
  72% after. Same remediation as EP-041; never misclassify as code
  failure.
- 2026-08-25: Redaction patterns. Broadened sk-/ghp_/AKIA patterns to
  match dotted runtime canary shapes (sk-liv...7890) after the M1 suite
  caught 4 failures. Evidence: ep043_unit_error_redacts_secrets +
  ep043_unit_evidence_redacts_secret_shaped_content now green. Security
  impact: broader redaction is strictly safer. License/compat: none.
- 2026-08-25: Evidence digest binding. buildEvidence test helper originally
  applied overrides AFTER digest computation, producing objects whose
  digest did not bind their content; the M1 suite caught the inconsistency
  (ep043_unit_evidence_digest_binds_content). Fixed by passing overrides
  into the constructor. Consequence: test helpers construct canonical
  objects only.
- 2026-08-25: ShipGate verdict + readiness decision are computed, never
  trusted from input; parsers recompute and reject mismatches. This
  encodes SPEC-008 authority (GATE PASSED != SHIPPED; DECISION MADE !=
  SHIPPED) at the wire boundary.
- 2026-08-26: M3 OPERATIONS.md real command surface. OPERATIONS.md gains
  the EP-043 release operations section: readiness generation, manifest
  build, verify-manifest, ship-gate-status, certification-rows, runtime
  health check, evidence refresh, fresh-clone procedure, rollback
  reference, exit/sentinel semantics, component facts, and an honest
  NOT-available list. Every documented command resolves to the real
  release-evidence CLI and is executed by the gate. Command
  documentation without execution is not claimed (COMMAND DOCUMENTED !=
  COMMAND EXECUTED).
- 2026-08-26: M3 CLI subcommands. Three real subcommands added so the
  operations surface is real, not aspirational: ship-gate-status
  (inspect obligations/verdict/blocking reasons from real repo state;
  exit 0 = inspection succeeded, verdict carries truth),
  certification-rows (list real RESULTS.md rows),
  verify-manifest (recompute digests from real artifact bytes + manifest
  digest; fails closed on tamper/missing). This is the fence's
  "generated clients required by the exact changed-file fence".
- 2026-08-26: M3 integration suite. ep043_integration_* tests exercise
  the REAL CLI against the REAL repository: real repo state reads,
  real artifact digests, OPERATIONS.md command resolution, NOT_READY
  preservation, fail-closed negative proofs (missing GRAPH -> UNAVAILABLE,
  tampered manifest -> VERIFICATION_FAILED, ghost artifact -> NOT_FOUND,
  bad args -> exit 2, cancellation -> no partial write), idempotent
  component digests, bounded timeout, audit fields, deterministic event
  emission, and a real fresh-clone temp checkout (git clone --depth 1
  file:///root/nexus) proving the operational path with no hidden local
  state. The full fresh-clone-equivalent rerun as an acceptance
  obligation remains M5 / NOT ASSERTED.
- 2026-08-26: M3 transport truth. The real dependency boundary is the
  local repository state + artifact bytes (file transport), matching
  COMPONENT_REGISTRY's local ArtifactStore default. Cloud transport
  (AWS S3/R2/B2) is NOT exercised and is explicitly listed NOT
  available in OPERATIONS.md; signing remains
  SIGNATURE_PRESENT_NOT_VERIFIED. Ship-gate execution/signing and
  production deploy/rollback are M5-owned and NOT ASSERTED.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
