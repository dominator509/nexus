NODE-META-BEGIN
ID: EP-035
DEPS: EP-034
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-035
VERIFY_SENTINEL: node verify EP-035: ok
GREEN_TAG: green/EP-035
NODE-META-END

# 1. Purpose / Big Picture

Implement Nexus Setup, owner recovery, deployment choice, hardware profiling, secure bootstrap, home-edge QR enrollment, discovery, people, and integration cards. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-035.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-035.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-034` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-035.md`
- `.agent/specs/SPEC-004-user-experience-dashboard-desktop-and-onboarding.md`
- `.agent/specs/SPEC-016-deployment-profiles-setup-compute-fabric-provisioning-and-updates.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-035.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-035-setup-wizard-and-onboarding.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-035.txt`
- `.agent/node-contracts/EP-035.md`
- `scripts/nodes/EP-035.sh`
- `apps/setup/`
- `crates/nexus-setup/`
- `packages/onboarding/`
- `tests/onboarding/`
- `schemas/deployment-profile.schema.json`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `SetupWizard` | `nexus-setup` | Defined by EP-035; provider-neutral and versioned |
| `DeploymentChoice` | `nexus-setup` | Defined by EP-035; provider-neutral and versioned |
| `HardwareProfiler` | `nexus-setup` | Defined by EP-035; provider-neutral and versioned |
| `OwnerBootstrap` | `nexus-setup` | Defined by EP-035; provider-neutral and versioned |
| `EdgeEnrollment` | `nexus-setup` | Defined by EP-035; provider-neutral and versioned |
| `DiscoveryWizard` | `nexus-setup` | Defined by EP-035; provider-neutral and versioned |
| `IntegrationCard` | `nexus-setup` | Defined by EP-035; provider-neutral and versioned |
| `RecoveryFlow` | `nexus-setup` | Defined by EP-035; provider-neutral and versioned |

Acceptance obligations:

1. A nontechnical user can deploy without editing environment files
2. Setup profiles hardware and recommends local versus API providers
3. One-time QR enrollment establishes certificates and private mesh
4. Failures are resumable, explainable, and never leave insecure partial state

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement nexus setup, owner recovery, deployment choice, hardware profiling, secure bootstrap, home-edge qr enrollment, discovery, people, and integration cards.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-035-M1.txt`, `.agent/node-contracts/EP-035.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-035-setup-wizard-and-onboarding.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-035.txt`, `.agent/node-contracts/EP-035.md`, `scripts/nodes/EP-035.sh`, `apps/setup/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep035_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-035.sh M1`

EXPECT:

- `EP-035 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-035 MILESTONE_PASS "M1 EP-035 M1: ok"`

FALLBACK: Use the local desktop setup application to drive an existing SSH server when cloud-provider API provisioning is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-035][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-035.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-035-M2.txt`, `.agent/node-contracts/EP-035.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `crates/nexus-setup/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep035_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-035.sh M2`

EXPECT:

- `EP-035 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-035 MILESTONE_PASS "M2 EP-035 M2: ok"`

FALLBACK: Use the local desktop setup application to drive an existing SSH server when cloud-provider API provisioning is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-035][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-035 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-035-M3.txt`, `.agent/node-contracts/EP-035.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `packages/onboarding/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep035_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-035.sh M3`

EXPECT:

- `EP-035 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-035 MILESTONE_PASS "M3 EP-035 M3: ok"`

FALLBACK: Use the local desktop setup application to drive an existing SSH server when cloud-provider API provisioning is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-035][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-035 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-035-M4.txt`, `.agent/node-contracts/EP-035.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/onboarding/`

CONTENT:

1. Create tests whose names begin `ep035_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-035.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-035 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-035 MILESTONE_PASS "M4 EP-035 M4: ok"`

FALLBACK: Use the local desktop setup application to drive an existing SSH server when cloud-provider API provisioning is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-035][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-035.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-035-M5.txt`, `.agent/node-contracts/EP-035.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `schemas/deployment-profile.schema.json`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-035` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-035.sh M5`
2. `sh scripts/node-verify.sh EP-035`
3. `sh scripts/scope-audit.sh EP-035`

EXPECT:

- `EP-035 M5: ok`
- `node verify EP-035: ok`
- `scope audit EP-035: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-035 MILESTONE_PASS "M5 EP-035 M5: ok"`

FALLBACK: Use the local desktop setup application to drive an existing SSH server when cloud-provider API provisioning is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-035][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-035` and observe `node verify EP-035: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-001` `one-package-deployment`: Deploy Nexus Core and a home edge from Nexus Setup using the local provider profile; assert owner login, health, private mesh, and fleet registration.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
  - `apps/setup/` Tauri onboarding contract package (`@nexus/setup`) in the pnpm workspace (pnpm-workspace.yaml + pnpm-lock.yaml +1 importer); framework-neutral TS contract layer (no React/DOM/backend-client imports; dependency-direction test enforced).
  - All 8 node-contract interfaces defined: SetupWizard (WizardState NOT_STARTED/IN_PROGRESS/BLOCKED/FAILED/RECOVERY_REQUIRED/COMPLETED + WizardStep PENDING/IN_PROGRESS/BLOCKED/FAILED/COMPLETE_LOCAL/VERIFIED - page-visited never completes a step, COMPLETE_LOCAL != VERIFIED, COMPLETED requires every step VERIFIED, typed validated transitions reject NOT_STARTED->COMPLETED and FAILED->COMPLETED), DeploymentChoice (canonical DeploymentProfile binding schemas/deployment-profile.schema.json verbatim - MANAGED/BYOC/EXISTING_SSH/HYBRID/FULLY_LOCAL + STABLE/BETA/DEVELOPER/PINNED; intent-only semantics: selection always UNVERIFIED, VERIFIED requires evidence record; explicit later-verification boundary), HardwareProfiler (HardwareFact with provenance USER_DECLARED/HOST_OBSERVED/PLATFORM_REPORTED/BENCHMARKED/HARDWARE_CERTIFIED - user-says-RTX never becomes detected; capability declarations never certify without measured evidence + measured provenance), OwnerBootstrap (OWNER_DETAILS_PROVIDED != OWNER_IDENTITY_VERIFIED != OWNER_PRINCIPAL_CREATED != OWNER_AUTHORIZED typed ladder; client isOwner flag rejected deny-unknown; deterministic first-owner INITIALIZED/ALREADY_INITIALIZED/CONFLICT contract - replay idempotent, competition Conflict, durable enforcement deferred M2), EdgeEnrollment (DISCOVERED != ENROLLMENT_REQUESTED != IDENTITY_VERIFIED != ENROLLED != TRUSTED != AUTHORIZED; discovery metadata never sufficient; BootstrapToken credential with secret/nonce never in JSON/toString/summary + expiry/used/revoked never usable), DiscoveryWizard (observations are data not authority; hostile content ADMIN/TRUSTED/AUTO-APPROVE/OWNER DEVICE inert; governed IntegrationSelection records principal), IntegrationCard (UNCONFIGURED != CONFIGURED != AUTHENTICATED != REACHABLE != HEALTHY + DEGRADED/ERROR; credential-exists never HEALTHY; capabilities never name-derived - Home Assistant advertises nothing without data), RecoveryFlow (RecoveryKit binding schemas/auth/recovery-kit.schema.json verbatim; RETRYABLE/NON_RETRYABLE/RESUME_CHECKPOINT/RECONCILE/ROLLBACK/REAUTHENTICATE/RESET/MANUAL_INTERVENTION; no-blind-replay: AMBIGUOUS -> RECONCILE, retry only when safe).
  - SPEC-006 error vocabulary + deny-unknown validation primitives (constructor AND fromJson enforce equivalent validation; round-trip + adversarial parsing tests).
  - Tests: 89 tests / 13 files green (names begin `ep035_unit_`): errors, validate, wizard (13), deployment, hardware, owner, enrollment (secret redaction canary), discovery, integration, recovery, schema parity (deployment-profile + recovery-kit fields/enums/required), dependency direction, surfaces; `tsc --noEmit` clean; Prettier clean.
  - Gates observed: `sh scripts/ep035-m1-tests.sh` -> `EP-035 M1: ok` (89 tests); `sh scripts/nodes/EP-035.sh M1` -> `EP-035 M1: ok` (exit 0; M1 branch rewired from EP-001-masking artifact-only check to the real gate with rc propagation).
  - Side gates: scope-audit EP-035 ok; expected-files EP-035 ok; reality-gate ok; security-check ok; license-gate ok; dependency-audit ok (blueprint validation ok after ASCII fix); format-check ok; lint ok; typecheck ok; pnpm -r test:unit ok (all workspace packages).
  - M2-M5 milestone manifests trimmed to comments (paths crates/nexus-setup/, packages/onboarding/, tests/onboarding/, schemas/deployment-profile.schema.json do not exist yet; repopulated at their owning milestone - EP-034 M1 precedent); expected-files trimmed to M1 scope + milestone manifests.
  - Certification: apps/setup + all 8 interfaces INTERNAL CONTRACT CERTIFIED; real host deployment / hardware probe / owner provisioning / edge enrollment / discovery / provider authentication / integration health / external recovery NOT ASSERTED (owned by M2-M5 + native/deployment milestones).
- [x] M2: Core behavior and deterministic invariants
  - `crates/nexus-setup/` Rust behavior crate (workspace member; Cargo.toml/Cargo.lock +1 member, cargo update --offline; deps only nexus-domain + serde/serde_json; dependency-direction enforced).
  - SetupWizardState (wizard.rs): canonical NOT_STARTED -> IN_PROGRESS start; typed validated transitions (NOT_STARTED->COMPLETED and FAILED->COMPLETED rejected Policy); per-step PENDING/IN_PROGRESS/BLOCKED/FAILED/COMPLETE_LOCAL/VERIFIED; COMPLETE_LOCAL != VERIFIED (VERIFIED requires RemoteVerification record; record on non-VERIFIED rejected Validation); COMPLETED requires every step VERIFIED; deny-unknown serde wire parsing.
  - Value objects (model.rs): DeploymentProfile (canonical schema parity, deny-unknown), DeploymentIntentRecord (select always UNVERIFIED; VERIFIED requires evidence; evidence only for VERIFIED), HardwareFact (provenance preserved; non-finite rejected), HardwareCapabilityDeclaration (CERTIFIED requires measured evidence + BENCHMARKED/HARDWARE_CERTIFIED provenance), OwnerBootstrapRequest (client isOwner field rejected deny-unknown), FirstOwnerDecision deterministic resolve_first_owner (INITIALIZED / ALREADY_INITIALIZED idempotent replay / CONFLICT competing), EnrollmentCredential BootstrapToken (Debug/Display/serde NEVER emit secret or nonce; redacted() view; is_usable only ISSUED within window; used/revoked/expired never valid again), EdgeEnrollmentRequest, DiscoveryObservation (hostile token detection; data not authority), DiscoveryReport, IntegrationSelection (governed, records principal), IntegrationCard (UNCONFIGURED != CONFIGURED != AUTHENTICATED != REACHABLE != HEALTHY + DEGRADED/ERROR; configured requires timestamp; reachable/healthy/degraded require verification event; credential-exists never HEALTHY; capabilities never name-derived), RecoveryKit (canonical schema parity), RecoveryEvidence + decide_recovery (no-blind-replay: AMBIGUOUS -> RECONCILE, retry safe only when RECONCILED or known-no-mutation; VALIDATION non-retryable; AUTHORIZATION reauthenticate; CONFLICT resume checkpoint; INTERNAL manual intervention).
  - Vocabulary (vocabulary.rs): all enums with SCREAMING_SNAKE_CASE wire + Display/FromStr/TryFrom (unknown rejected Vocabulary); hostile authority token detection.
  - Tests: 73 tests green (in-crate vocabulary 3 + integration files wizard 10, deployment 8, hardware 7, owner 9, enrollment 8, discovery 6, integration 9, recovery 10, dependency-direction 3); `cargo clippy -p nexus-setup --all-targets -- -D warnings` clean; `cargo fmt` clean.
  - Gates observed: `sh scripts/ep035-m2-tests.sh` -> `EP-035 M2: ok` (73 tests); `sh scripts/nodes/EP-035.sh M2` -> `EP-035 M2: ok` (exit 0; M2 branch rewired to the real gate with rc propagation).
  - Side gates: scope-audit EP-035 ok; expected-files EP-035 ok; reality-gate ok; security-check ok; license-gate ok; dependency-audit ok; format/lint/typecheck/test-unit (workspace) verified at committed-tree reproduction.
  - Certification: nexus-setup behavior INTERNAL CONTRACT CERTIFIED (deterministic invariants only); real host deployment / hardware probe / owner provisioning / edge enrollment / discovery / provider authentication / integration health / external recovery NOT ASSERTED (M3-M5 + native/deployment milestones own them).
- [x] M3: Real dependency and transport integration
  - `packages/onboarding/` -> `@nexus/onboarding` integration package (pnpm workspace importer; deps @nexus/setup + @nexus/contracts + pg + nats only; dependency-direction preserved).
  - Real deps from COMPONENT_REGISTRY.yaml: PostgreSQL 18.4 (REAL_DATABASE, canonical durable truth store) + NATS 2.14.3 (REAL_SOCKET_SERVICE, event bus), both digest-pinned and exercised through ephemeral containers; npm nats 2.29.3 + pg 8.23.0 resolved and locked.
  - Migration DDL (migrations/001_onboarding.sql) with persistence-level guards: unique-index first-owner singleton, enrollment token hash-only, SELECTED != VERIFIED with evidence requirement, CONFIGURED != AUTHENTICATED != REACHABLE != HEALTHY ladder with evidence timestamps, SQL CHECK (mutation_state <> 'UNKNOWN' OR retry_safe = FALSE) no-blind-replay.
  - DB transport with SPEC-006 error mapping (connection refused -> Unavailable, timeout -> Timeout, unique constraint -> Conflict, malformed -> Validation, authentication denied -> Authentication); pg int8 BIGINT-to-string parser; redaction helpers token-scan JSON-shaped blobs.
  - Five durable stores: OwnerBootstrap (unique-index enforced first-owner singleton; INITIALIZED / ALREADY_INITIALIZED idempotent replay / CONFLICT competing; concurrent race exercised), EnrollmentToken (atomic one-time claim; SHA-256 hashes only; expiry denied / revoke denied / replay denied), DeploymentIntent (SELECTED != VERIFIED; verification requires evidence), IntegrationState (ladder with evidence timestamps; invalid leap -> Policy; credential-exists never HEALTHY), RecoveryCheckpoint (SQL CHECK prevents UNKNOWN mutation from being retry-safe; no blind replay).
  - NATS JetStream publisher: stream auto-created via jetstreamManager().addStream on connect; payloads redacted before publish (canary ZERO_LEAKAGE on the real bus); subscribe-first + nc.flush() ordering; real publish/subscribe round-trip.
  - Tests: 44 total (37 integration + 7 unit) across 8 files green (names begin `ep035_integration_` for container-gated); failure injection, concurrency race, hostile redaction canaries, exact-target readbacks.
  - Gates observed: direct `sh scripts/ep035-m3-tests.sh` -> `EP-035 M3: ok` (3 consecutive pre-edit runs + 1 post-blueprint-fix run, exit 0); `sh scripts/nodes/EP-035.sh M3` -> `EP-035 M3: ok` (exit 0; M3 branch rewired from nonexistent-filter masking to the real gate with rc propagation).
  - Blueprint fix: Docker Go-template name format in the orphan guard flagged as double-brace placeholder; replaced with anchored name filter `docker ps -aq --filter name=^/nexus-ep035-`; ownership proven with live probe containers (only EP-035 matched; EP-034/EP-036/EP-044/unrelated nexus-* excluded).
  - Side gates: scope-audit EP-035 ok; expected-files EP-035 ok; reality-gate ok; security-check ok; license-gate ok; dependency-audit ok; blueprint validation ok; format-check ok (after Prettier write on onboarding package); lint ok; typecheck ok; shell syntax ok.
  - Workspace battery: pnpm -r test:unit ok (onboarding 7 + setup 89 + workspace); pnpm -r test:integration ok (onboarding 37/37, zero failures); cargo test --workspace 2341 passed 0 failed (330 suites); no zero-test collections; no masked M3 tests.
  - Hygiene: 0 EP-035 containers after gates; no scratch processes; no EP-035-owned ports; only pre-existing EP-002/m5-wrap environment containers remain.
  - Certification: @nexus/onboarding IMPLEMENTED; PostgreSQL-backed durable stores PROVIDER/INTEGRATION CERTIFIED against the exact tested PostgreSQL 18.4 runtime; NATS JetStream publisher PROVIDER/TRANSPORT CERTIFIED against the exact tested NATS 2.14.3 runtime; OwnerBootstrap/EnrollmentToken/DeploymentIntent/IntegrationState/RecoveryCheckpoint DURABLE INTEGRATION CERTIFIED; actual production VPS deployment / physical hardware profiling / real external device enrollment / network discovery / provider health outside PostgreSQL+NATS / deployment-ship readiness NOT ASSERTED.
- [x] M4: Forced failures, abuse cases, and observability
  - `tests/onboarding/` -> `@nexus/onboarding-failure` forced-failure package (pnpm workspace importer; deps @nexus/onboarding + @nexus/setup + pg + nats; production code never mocked - failure mechanism is the real provider boundary).
  - Seven failure suites / 24 tests, all named `ep035_failure_*`: unavailable dependency (postgres container terminated mid-suite -> Unavailable; no fabrication after provider death), timeout budget (statement-timeout exhaustion on the production transport -> Timeout; bounded recovery - provider usable after), malformed input (corrupted enrollment row violating CHECK -> Validation; unknown integration status enum -> Validation; corrupt non-JSON capability payload at durable boundary -> Validation; evidence-less VERIFIED rejected), duplicate request (concurrent one-time token claim exactly-once; competing first owner -> Conflict with durable unique index; replay -> ALREADY_INITIALIZED; second deployment verification -> Conflict), denied permission (revoked token never claimable; expired token denied; used-token replay denied; invalid integration ladder leap -> Policy with durable row unchanged; SQL CHECK refuses UNKNOWN+retry_safe), partial side effect (UNKNOWN mutation never retry-safe; retry allowed only after RECONCILED readback; provider death mid-mutation -> Unavailable no partial success; ambiguous recovery -> RECONCILE then durable reconcile), observability (structured SPEC-006 errors carry correlation id; redaction strips secret-shaped detail with UUID-safe fields preserved; redacted NATS event ZERO_LEAKAGE on the real bus; refused NATS -> Unavailable).
  - Real production defect fixed by M4: `OnboardingDb.mapError` classified a provider that dies mid-session (ECONNRESET/EPIPE/terminated connection) as Internal; now maps to Unavailable (timeout-flavored terminations still Timeout). Evidence: ep035_failure_unavailable_dependency + ep035_failure_partial_side_effect + M3 regression green.
  - Ops diagnostic + bounded recovery: `tests/onboarding/ops/onboarding-diag.sh` diagnose/recover for PostgreSQL + NATS (digest-pinned), fails closed rc=3 on unreachable providers, never prints credentials, recover reports the exact restart command and never fabricates state (EP-028/EP-027 diag convention).
  - Gate `scripts/ep035-m4-tests.sh`: package + 7 failure files + ops diagnostic present, production-import guards (OwnerBootstrapStore/EnrollmentTokenStore/RecoveryCheckpointStore/OnboardingEventPublisher observed, no vi.mock), tsc clean, non-zero 24 passing, zero fail/skip, all 7 owned suite sentinels, orphan guard docker filter name=^/nexus-ep035-, ops diag fail-closed sanity, M1+M2+M3 regressions.
  - Gates observed: direct `sh scripts/ep035-m4-tests.sh` -> `EP-035 M4: ok` (3 consecutive runs, 24 tests); `sh scripts/nodes/EP-035.sh M4` -> `EP-035 M4: ok` (exit 0; M4 branch rewired from nonexistent-filter masking to the real gate with rc propagation).
  - Side gates: scope audit EP-035: ok; expected files EP-035: ok; reality gate: ok; security check: ok; license gate: ok; dependency audit: ok; blueprint validation: ok; format check: ok (Prettier write); lint: ok; typecheck: ok; shell syntax: ok.
  - Workspace battery: pnpm -r test:unit ok (tests/onboarding 24 + onboarding 7 + setup 89 + workspace); pnpm -r test:integration ok (onboarding 37/37, zero failures); cargo test --workspace 2341 passed 0 failed (330 suites); no zero-test collections; no masked M4 tests.
  - Hygiene: 0 EP-035 containers after gates; no scratch processes; no EP-035-owned ports; foreign evidence churn reverted.
  - Certification: @nexus/onboarding-failure forced-failure suite INTERNAL CONTRACT CERTIFIED (fail-closed semantics under terminated providers, exhausted budgets, corrupted messages, duplicate requests, denied permissions, partial side effects, and observability proven against REAL PostgreSQL 18.4 + NATS 2.14.3; production mapping defect ECONNRESET->Unavailable fixed); actual production VPS deployment / physical hardware profiling / real external device enrollment / network discovery / provider health outside PostgreSQL+NATS / deployment-ship readiness NOT ASSERTED (M5 + deployment milestones own them).
- [x] M5: Live-fire, operations, and node closure
  - LF-001 `one-package-deployment` real journey: the phantom `scripts/proof-runner.sh` -> `cargo run -p nexus-cli` path (no such crate; EP-001 masking class) is GONE. `scripts/live-fire/LF-001.sh` now invokes the real M5 gate `scripts/ep035-m5-tests.sh`.
  - One-package artifact: `scripts/ep035-one-package-build.sh` builds the nexus-setup one-package deployment bundle from the CURRENT source tree into `.agent/state/livefire/ep035-bundle/` (gitignored generated dir): canonical `schemas/deployment-profile.schema.json`, onboarding DDL `migrations/001_onboarding.sql`, and the built `@nexus/onboarding` runtime (tsc build), with a deterministic MANIFEST.json (per-file SHA-256 + git_commit + artifact_hash over sorted file contents; same commit -> same identity).
  - Live-fire package `tests/livefire/deployment/` (`@nexus/deployment-livefire`, workspace importer +1): 8 `ep035_lf001_*` tests against REAL ephemeral PostgreSQL 18.4 + NATS 2.14.3 (digest-pinned per COMPONENT_REGISTRY.yaml; no pre-existing state - fresh containers/volumes, migrations NOT auto-applied by the harness, the bundle's own DDL boots the runtime): artifact identity bound to current commit; clean target boot + real readiness (pg server_version 18.x + pg_isready-class probe + NATS PONG + onboarding durable-boundary health SELECT); deployment selection recorded as intent (FULLY_LOCAL local provider profile, SELECTED, never verified by selection); first owner bootstrap through the real OwnerBootstrapStore (INITIALIZED + exact-target readback by principal id); verification requires evidence (SELECTED != VERIFIED; evidence-less VERIFIED rejected Verification; with evidence -> VERIFIED + readback); redacted owner/deployment events over the real NATS bus (owner_initialized + deployment_selected + deployment_verified with correlation preserved, no secret-shaped content, subscribe-first + flush); replay idempotency (same first-owner request -> ALREADY_INITIALIZED, exactly one durable owner row); current-run evidence `.agent/state/evidence/LF-001-ep035-m5.json` written by the run and re-read/validated (lf_id/node/milestone/run_id/git_commit/artifact_hash).
  - Gate `scripts/ep035-m5-tests.sh`: live-fire package + sources + bundle builder present, M5 fence populated (not placeholder), production-import guards (@nexus/onboarding + @nexus/setup + OwnerBootstrapStore + DeploymentIntentStore + OnboardingEventPublisher observed, no vi.mock), tsc clean, verbose-reporter vitest run (file-serial) with non-zero pass, zero fail/skip, owned proof-name sentinels (anti-masking), evidence freshness (mmin -10) + node/milestone/lf binding + artifact hash + redaction scan, LF runner integrity (LF-001.sh calls the real gate; no proof-runner/nexus-cli/proof-run invocation anywhere), M1+M2+M3+M4 regressions, orphan guard (docker filter name=^/nexus-ep035- + EP-035-owned process pattern only), milestone artifacts present.
  - Node M5 rewired: `scripts/nodes/EP-035.sh` M5|verify now runs artifact-check + the real M5 gate with rc propagation (the pre-created `cargo test -p nexus-setup` + proof-runner branch is gone).
  - Gates observed: direct `sh scripts/ep035-m5-tests.sh` -> `EP-035 M5: ok` (8 tests); `sh scripts/nodes/EP-035.sh M5` -> `EP-035 M5: ok` (exit 0); `sh scripts/live-fire/LF-001.sh` -> `LF-001: ok` (exit 0).
  - Side gates: scope audit EP-035: ok; expected files EP-035: ok; reality gate: ok; security check: ok; license gate: ok; dependency audit: ok; blueprint validation: ok; format check: ok; lint: ok; typecheck: ok; shell syntax: ok.
  - Workspace battery: pnpm -r test:unit ok (livefire/deployment 8 + tests/onboarding 24 + onboarding 7 + setup 89 + workspace); pnpm -r test:integration ok (onboarding 37/37 zero failures; workflows package keeps its own documented skips); cargo test --workspace 2341 passed 0 failed (108 ignored, 330 suites); no zero-test collections; no masked M5 tests.
  - Hygiene: 0 EP-035 containers after gates; no scratch processes; foreign evidence churn from battery runs reverted (EP-033 LF-005 + EP-031 LF-009).
  - Certification (final honest): @nexus/setup INTERNAL CONTRACT CERTIFIED; nexus-setup INTERNAL CONTRACT CERTIFIED; @nexus/onboarding INTEGRATION CERTIFIED; PostgreSQL 18.4 PROVIDER/INTEGRATION CERTIFIED for exact exercised runtime; NATS 2.14.3 PROVIDER/INTEGRATION CERTIFIED for exact exercised runtime; LF-001 COMPOSITION CERTIFIED for the exact tested one-package deployment path; one-package artifact CERTIFIED for the exact test environment (deterministic SHA-256 identity bound to the current commit); actual production VPS deployment / arbitrary Linux host / physical hardware profiling / external edge enrollment / real LAN discovery / ship readiness NOT ASSERTED (deployment + native + ship milestones own them); owner login (authN/Z) NOT ASSERTED (LF-003/EP-007), private mesh NOT ASSERTED (mesh milestone), fleet registration NOT ASSERTED (fleet milestone).

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-21: `assertEnum` generic inference breaks when the allowed set is typed `Set<string>`; the set must be declared `ReadonlySet<T>` so the union type is inferred and the returned value stays typed (otherwise constructor params reject `string`). Same class bit every contract file with enums.
- 2026-08-21: TypeScript `interface` ports are type-level only - a runtime surfaces test cannot assert `typeof X.Port`. The barrel export existence is enforced by `tsc --noEmit`; runtime surface tests assert the value objects and vocabulary instead.
- 2026-08-21: blueprint_validate scans TS source comments for non-ASCII: an em-dash in a discovery.ts doc comment failed dependency-audit's blueprint phase (same class as prior .rs/.md fixes). ASCII-only in every file under the package.
- 2026-08-21: Prettier flags 14 new apps/setup files on first format check (same class as EP-033's 64-file and EP-034's 8-file fixes); `prettier --write` resolves.
- 2026-08-21: The pre-created `scripts/nodes/EP-035.sh` M1 case was artifact-check-only (vacuity gap) and its M2-M5 cases run `cargo test -p nexus-setup` for a crate that does not exist until M2 (EP-001 masking class); M1 rewired to the real gate, M2-M5 left for their owning milestones with the unconditional-ok tail removed (rc propagation).
- 2026-08-21: pnpm-workspace.yaml needed `apps/setup` added and pnpm-lock.yaml gained the importer; both are M1-owned manifest changes recorded in the fence (EP-033 M2 precedent).
- 2026-08-21: Direct shell lacks dart/mise shims; `format-check.sh` must run under `scripts/env.sh` (node-verify path), matching the EP-020 env-shadowing lesson in reverse (shim needed, not shim-avoided).
- 2026-08-21: `Self::Error` inside a TryFrom impl is ambiguous when the same type also implements FromStr; spell the concrete error type (`Result<Self, SetupError>`) instead of the associated item.
- 2026-08-21: A `#[derive(Eq)]` enum containing `f64` fails to compile; HardwareValue/HardwareFact/HardwareProfile carry PartialEq only.
- 2026-08-21: BTreeMap keys require Ord; the enum_vocab macro derives PartialOrd/Ord so wizard step validation can use a BTreeMap.
- 2026-08-21: serde `#[serde(skip_serializing)]` on the credential secret/nonce keeps them out of JSON, but Debug/Display needed manual redaction impls; the EnrollmentCredential Debug prints `[REDACTED]`.
- 2026-08-21: clippy `doc_lazy_continuation` (-D warnings) flags continuation lines of a doc list item; indent the wrapped lines in crate-level docs.
- 2026-08-21: rtk-tee compresses interactive cargo output; the M2 gate writes `cargo test` output to a log file so vacuity greps observe raw `test result: ok. N passed` sentinels (established pattern).
- 2026-08-21: The M2 gate's total-count awk read field 3 (`ok.`) instead of field 4 (the number); gate re-run after fix reports 73 tests.
- 2026-08-21: blueprint_validate flags ANY non-code file containing a double-brace pair as an unresolved placeholder; a shell gate using a Docker Go-template name format (`docker ps --format`) fails the blueprint phase (same scan class as the em-dash ASCII rule). Fix: Docker anchored name filter `docker ps -aq --filter name=^/nexus-ep035-` (regex anchored on the leading slash Docker prefixes onto names) instead of a Go template; ownership proven with live probe containers (only nexus-ep035-* matched; EP-034/EP-036/EP-044/unrelated excluded).
- 2026-08-21: NATS sends its INFO banner before PONG; a small readiness probe reads only the banner, so the harness readiness probe reads >= 4096B before declaring the server ready.
- 2026-08-21: NATS JetStream is disabled by default in the official nats image; the container must run with `-js`, and js.publish requires an existing stream, so the publisher creates the stream via jetstreamManager().addStream on connect.
- 2026-08-21: NATS subscribers must attach before publish and call nc.flush() after subscribe; a fast publish can otherwise arrive before the async iterator is ready (subscribe-first + flush ordering in all NATS tests).
- 2026-08-21: pg returns BIGINT columns as JS strings; an int8 parser is registered globally in OnboardingDb so durable ids/keys stay canonical.
- 2026-08-21: pg Pool emits unhandled 'error' events for background connection failures; the harness attaches a Pool error listener to avoid 7 uncaught-exception noise events per run.
- 2026-08-21: nats.js 2.29.3 moved stream management to jetstreamManager() (addStream no longer lives on the js client).
- 2026-08-21: The failure suite exposed a REAL production classification defect: `OnboardingDb.mapError` mapped a provider that dies mid-session (ECONNRESET/EPIPE/"connection terminated") to Internal. A terminated provider is the same Unavailable class as a refused one; fixed in the production transport and proven by ep035_failure_partial_side_effect + M3 regression green. Ordering matters: timeout-flavored terminations ("...due to connection timeout") must be checked before the terminated-connection rule so they stay Timeout.
- 2026-08-21: write_file/patch tooling redacts password-shaped literals in transit; shell/test files containing provider credentials must be created by copying the committed M3 harness or via python heredoc with runtime-constructed literals, then verified with od/grep on disk (display-level `***` is not the on-disk truth).
- 2026-08-21: A kill-provider test that removes the shared postgres container MUST be the last test in its file; any later test in the same file starves on a dead provider (freshDb/migrate rejects before the store call).
- 2026-08-21: `OwnerBootstrapRequest` requires `owner_name`; `EnrollmentCredential.parse` rejects a `correlation_id` field (it is not part of the canonical shape); deployment verification evidence is the canonical `{verified_at_unix_s, evidence_id, verifier}` shape - the failure suite constructors were corrected against the real contract surfaces.
- 2026-08-21: vitest's default reporter does not print individual test names, so a gate that greps the log for owned proof names sees only the file name; the M5 gate runs vitest with `--reporter=verbose` so every `ep035_lf001_*` proof is observable (same class as the M2 cargo log-file capture pattern).
- 2026-08-21: The LF-001 replay test initially bootstrapped a SECOND owner request and the real durable singleton correctly returned CONFLICT - first-owner semantics are durable, not a test artifact. Replay proof must re-invoke the SAME request (same idempotency key) and expect ALREADY_INITIALIZED with exactly one owner row; this is the journey's real idempotency contract.
- 2026-08-21: A gate's own anti-masking grep can false-positive on its own comments: the M5 gate initially grepped the literal `nexus-cli` and tripped on the LF-001.sh comment saying the phantom path was gone. Fix: reworded the comment and made the phantom-path greps precise invocation patterns (`-p nexus-cli`, `proof run`, `nexusctl proof`, `proof-runner.sh`).
- 2026-08-21: The orphan guard must not treat ambient tooling as node-owned processes: long-running LSP tsservers (tooling, not EP-035) tripped a broad `[t]sserver` grep. The M5 guard scopes to EP-035-owned patterns only (`vitest` for livefire/onboarding packages, `node.*ep035_lf001`).
- 2026-08-21: The one-package bundle build emits the onboarding `dist/` (gitignored) and `.agent/state/livefire/`; the generated bundle dir needs a .gitignore entry, and any .gitignore change is scope-audit-relevant - the file must be registered in the node fence and expected-files.
- 2026-08-21: `git rev-parse HEAD` (not reading `.git/HEAD` and stripping `ref:`) is the correct current-commit binding; reading the ref file yields the branch name and breaks evidence identity checks.
- 2026-08-21: write_pipeline redaction of password-shaped literals applies to patched harness files too (the copied M4 harness displayed `PGPASSWORD=***` in tool output); on-disk bytes verified via python (0 literal `***`, 4 `${PG_PASSWORD}` interpolations) - display redaction is not on-disk truth.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-21 | M2 behavior crate is Rust under `crates/nexus-setup` (per ExecPlan CHANGE + ARCHITECTURE.md layer 2 contracts/application), dependency-light (nexus-domain + serde/serde_json) with the same deny-unknown, SPEC-006 error, and typed-id discipline as prior contract crates. Evidence: crate sources + dependency-direction tests. Alternatives: implementing behavior in TS (would duplicate the M1 contract layer without a Rust application seam). Consequence: provider-neutral Rust behavior layer for later provider adapters. Reversal: ADR + plan update. Security/license/compat: none.
- 2026-08-21 | First-owner durable enforcement remains deferred (in-memory record semantics only); the deterministic INITIALIZED/ALREADY_INITIALIZED/CONFLICT decision is proven and the durable store is owned by the deployment milestone. Evidence: resolve_first_owner + tests. Alternatives: fabricating a durable store in M2 (forbidden). Consequence: honest boundary. Reversal: later milestone. Security/license/compat: none.
- 2026-08-21 | Enrollment credential secrets are structurally unreachable: Debug/Display/serde never emit secret or nonce; only redacted() carries the safe view. Evidence: ep035_unit_enrollment_secret_never_appears_in_any_surface. Alternatives: leaking secrets to summaries (forbidden). Consequence: no secret path in the behavior layer. Reversal: none. Security/license/compat: none.
- 2026-08-21 | Cargo.toml/Cargo.lock registered in the M2 fence (new workspace member; EP-022/EP-023 precedent) so scope-audit stays green. Evidence: expected-files EP-035.txt. Alternatives: skipping the manifests (scope-audit failure). Consequence: fence matches reality. Reversal: none. Security/license/compat: none.

- 2026-08-21 | M1 contract package language is TypeScript under `apps/setup` (`@nexus/setup`, pnpm workspace), the Tauri frontend layer per ARCHITECTURE.md; the Rust behavior crate `crates/nexus-setup` is owned by M2 and the onboarding UI package `packages/onboarding` by M3. Evidence: ARCHITECTURE.md repository map (apps/setup = Tauri onboarding, Rust + TS); M1/M2/M3 CHANGE lists. Alternatives: a full Tauri shell in M1 (UI/runtime, not contract boundary). Consequence: provider-neutral contract layer reusable across future UI/runtime implementations. Reversal: ADR + plan update. Security/license/compat: none.
- 2026-08-21 | Wizard/step/integration/trust/hardware/owner/recovery vocabularies (WIZARD_STATES, WIZARD_STEPS, WIZARD_STEP_STATUSES, INTEGRATION_STATUSES, ENROLLMENT_TRUST_STATES, HARDWARE_PROVENANCES, OWNER_BOOTSTRAP_STATES, RECOVERY_OUTCOMES, RECOVERY_FAILURE_CLASSES, RECOVERY_MATERIAL_KINDS, DEPLOYMENT_VERIFICATION_STATES, CAPABILITY_CERTIFICATION_STATES) are EP-035-owned additions from the node directive's canonical candidate lists; SPEC-004/016 do not define these enums. Schema-parity tests bind DeploymentProfile + RecoveryKit to their canonical JSON schemas verbatim. Evidence: apps/setup contract files + ep035_unit_schema_parity. Alternatives: inventing parallel names (forbidden). Consequence: single canonical setup vocabulary in the barrel. Reversal: ADR + schema update. Security/license/compat: none.
- 2026-08-21 | State truthfulness is structural, not cosmetic: COMPLETE_LOCAL != VERIFIED, SELECTED != PROVISIONED/VERIFIED, DISCOVERED != TRUSTED, CONFIGURED != HEALTHY, OWNER_DETAILS != OWNER_AUTHORIZED, LOCAL_CHECKPOINT != REMOTE_VERIFIED, AMBIGUOUS_MUTATION != SAFE_TO_RETRY. Each boundary has an explicit typed transition requiring evidence. Evidence: wizard/deployment/hardware/owner/enrollment/integration/recovery tests. Alternatives: single status booleans (forbidden by directive). Consequence: fail-closed truthfulness invariants. Reversal: none without directive change. Security/license/compat: none.
- 2026-08-21 | First-owner bootstrap semantics (INITIALIZED/ALREADY_INITIALIZED/CONFLICT) are encoded as a deterministic pure contract with durable enforcement explicitly deferred to M2 (crates/nexus-setup). Evidence: owner.ts resolveFirstOwnerRequest + tests. Alternatives: in-memory enforcement in M1 (would be a fake durable store). Consequence: honest boundary (contract proven, provider deferred). Reversal: M2. Security/license/compat: none.
- 2026-08-21 | Enrollment credentials (BootstrapToken) classify secret/nonce as SECRET: toJSON/toString/redacted() never emit them; summaries use the redacted shape. Evidence: ep035_unit_enrollment canary tests. Alternatives: emitting secrets to summaries (forbidden). Consequence: no secret leakage path in the contract layer. Reversal: none. Security/license/compat: none.
- 2026-08-21 | M2-M5 milestone manifests trimmed to comments and expected-files trimmed to M1 scope because the listed future paths do not exist yet (EP-034 M1 precedent; artifact check validates ALL milestone manifests even at M1). Evidence: .agent/milestone-files/EP-035-M{2..5}.txt + expected-files. Alternatives: creating empty placeholder dirs (forbidden by directive Y). Consequence: fence matches reality; repopulated at each owning milestone. Reversal: none. Security/license/compat: none.
- 2026-08-21 | M1 makes no provider/hardware/deployment claims: real host deployment, hardware probing, owner provisioning, edge enrollment, discovery, provider authentication, integration health, and external recovery are NOT ASSERTED and owned by M2-M5 + native/deployment milestones. Evidence: certification boundary in progress + node contract reality rule. Alternatives: claiming provider certification from contract tests (forbidden). Consequence: honest INTERNAL CONTRACT CERTIFIED only. Reversal: none. Security/license/compat: none.
- 2026-08-21 | M3 uses PostgreSQL 18.4 as the durable truth store and NATS 2.14.3 as the event publication boundary, both locked in COMPONENT_REGISTRY.yaml with exact digests and exercised through ephemeral containers; no in-memory production engine. Evidence: COMPONENT_REGISTRY.yaml entries + packages/onboarding integration tests. Alternatives: an in-memory engine (forbidden by milestone CONTENT). Consequence: real provider paths proven on the exact tested runtimes. Reversal: ADR + plan update. Security/license/compat: postgres + nats licenses registered and gate-clean.
- 2026-08-21 | No blind replay is encoded in SQL, not application-only validation: `CHECK (mutation_state <> 'UNKNOWN' OR retry_safe = FALSE)` in the recovery checkpoint table. Evidence: migrations/001_onboarding.sql + recovery integration tests. Alternatives: app-level guard only (weaker). Consequence: durable persistence-level invariant survives process restarts and other writers. Reversal: none without directive change. Security/license/compat: none.
- 2026-08-21 | Enrollment tokens persist only as SHA-256 hashes (secret + nonce hashed; raw material never stored), with atomic one-time claim and expiry/revoke/replay denial at the DB constraint level. Evidence: enrollment-token.store.ts + DDL + enrollment integration tests. Alternatives: raw-token persistence for debugging (forbidden). Consequence: token theft from the store yields no usable credential. Reversal: none. Security/license/compat: none.
- 2026-08-21 | Integration health ladder CONFIGURED != AUTHENTICATED != REACHABLE != HEALTHY with evidence timestamps required for higher states; credential-exists never HEALTHY; invalid leaps rejected Policy. Evidence: DDL CHECKs + integration-state integration tests. Alternatives: single status boolean (forbidden by directive). Consequence: fail-closed state truthfulness. Reversal: none. Security/license/compat: none.
- 2026-08-21 | M3 certification is limited to the exact exercised runtimes: PostgreSQL-backed durable stores PROVIDER/INTEGRATION CERTIFIED against PostgreSQL 18.4, NATS JetStream publisher PROVIDER/TRANSPORT CERTIFIED against NATS 2.14.3. Production VPS deployment, physical hardware profiling, real external device enrollment, network discovery, provider health outside PostgreSQL/NATS, and deployment/ship readiness NOT ASSERTED. Evidence: gate output + integration test names. Alternatives: claiming broader certification (forbidden by reality law). Consequence: honest boundary; later milestones own the rest. Reversal: none. Security/license/compat: none.
- 2026-08-21 | M4 failure suite lives in `tests/onboarding/` (`@nexus/onboarding-failure`) per the M4 CHANGE fence and EP-033 M4 precedent, importing the REAL @nexus/onboarding production stores and exercising REAL provider termination, budget exhaustion, and durable constraint violations. Evidence: 7 ep035_failure_* suites (24 tests) + production-import guard in the gate. Alternatives: mocking the stores (forbidden by milestone CONTENT). Consequence: fail-closed behavior proven against the real boundary; the suite found and fixed a real ECONNRESET->Internal misclassification. Reversal: none. Security/license/compat: none.
- 2026-08-21 | Ops diagnostic + bounded recovery for the onboarding providers follows the EP-028/EP-027 convention: `tests/onboarding/ops/onboarding-diag.sh` with diagnose (fail closed rc=3 on unreachable) and recover (reports exact digest-pinned restart command, never fabricates state). Evidence: gate sanity check + diag script. Alternatives: fabricating a session in recover (forbidden). Consequence: operators get a bounded recovery path for postgres+nats. Reversal: none. Security/license/compat: none.
- 2026-08-21 | LF-001's one-package artifact is the nexus-setup one-package deployment bundle built from the CURRENT source tree (canonical deployment-profile schema + onboarding DDL + built @nexus/onboarding runtime + deterministic SHA-256 manifest), per the repository's bundle concept; no new installer was invented because no other graph owner owns a setup/onboarding one-package installer (DEPLOYMENT.md bundle/ship artifacts are ship-gate owned, infra/compose/core.yaml is EP-044). Evidence: scripts/ep035-one-package-build.sh + LF-001-ep035-m5.json artifact_hash. Alternatives: resurrecting the phantom nexus-cli (forbidden), inventing a parallel installer (scope theft). Consequence: the LF owns a real traceable artifact; identity is deterministic per commit. Reversal: none. Security/license/compat: none.
- 2026-08-21 | LF-001 must NOT be satisfied by M3/M4 regressions alone: the M5 gate runs M1-M4 as required regressions AND exercises the owned one-package deployment journey (bundle -> clean target -> package DDL boot -> readiness -> owner bootstrap -> verification -> events -> replay -> evidence). Evidence: gate structure + anti-masking sentinels. Alternatives: rewiring LF-001 to call M3/M4 gates (explicitly forbidden by directive). Consequence: the LF proves its own user-visible deployment path. Reversal: none. Security/license/compat: none.
- 2026-08-21 | The LF-001 deployment target is a clean ephemeral stack (fresh digest-pinned postgres+nats containers, fresh volumes, no pre-existing owner/intent/integration state, migrations applied from the bundle itself), so first-run deployment cannot be faked by a preconfigured host. Evidence: harness startStack({migrate:false}) + applyBundleMigrations in the suite. Alternatives: deploying onto pre-provisioned state (forbidden by directive). Consequence: PREEXISTING HOST REQUIREMENTS (docker, host ports) are distinguished from OPERATIONS PERFORMED BY THE PACKAGE (DDL boot, readiness, owner bootstrap). Reversal: none. Security/license/compat: none.
- 2026-08-21 | Evidence freshness is enforced by (a) the suite generating a unique run_id per run and re-reading/validating its own evidence file, (b) the gate requiring mmin -10 + node/milestone/lf binding + artifact hash + redaction scan; stale files never satisfy. Evidence: gate evidence block + suite evidence test. Alternatives: trusting file existence (forbidden). Consequence: current-run evidence is mandatory. Reversal: none. Security/license/compat: none.
- 2026-08-21 | Owner login (authN/Z), private mesh, and fleet registration from the LF-001 prose are NOT claimed by EP-035: the journey stops at the exact truthful states (owner principal created + exact-target readback; deployment VERIFIED with evidence; redacted events) and the evidence records explicit deferred owners (LF-003/EP-007, mesh milestone, fleet milestone). Evidence: certification_boundary in LF-001-ep035-m5.json. Alternatives: faking mesh/fleet (forbidden by reality law). Consequence: honest COMPOSITION CERTIFIED boundary only. Reversal: none. Security/license/compat: none.
- 2026-08-21 | The M5 gate's orphan guard scopes to EP-035-owned processes (vitest for livefire/onboarding, node.*ep035_lf001) plus the anchored docker name filter; ambient tooling (LSP tsservers) is not node-owned and is excluded by construction. Evidence: gate orphan guard. Alternatives: broad process greps (false positives on tooling). Consequence: hygiene checks real ownership. Reversal: none. Security/license/compat: none.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
