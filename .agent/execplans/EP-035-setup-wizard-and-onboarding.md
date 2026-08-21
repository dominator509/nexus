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

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-21: `assertEnum` generic inference breaks when the allowed set is typed `Set<string>`; the set must be declared `ReadonlySet<T>` so the union type is inferred and the returned value stays typed (otherwise constructor params reject `string`). Same class bit every contract file with enums.
- 2026-08-21: TypeScript `interface` ports are type-level only - a runtime surfaces test cannot assert `typeof X.Port`. The barrel export existence is enforced by `tsc --noEmit`; runtime surface tests assert the value objects and vocabulary instead.
- 2026-08-21: blueprint_validate scans TS source comments for non-ASCII: an em-dash in a discovery.ts doc comment failed dependency-audit's blueprint phase (same class as prior .rs/.md fixes). ASCII-only in every file under the package.
- 2026-08-21: Prettier flags 14 new apps/setup files on first format check (same class as EP-033's 64-file and EP-034's 8-file fixes); `prettier --write` resolves.
- 2026-08-21: The pre-created `scripts/nodes/EP-035.sh` M1 case was artifact-check-only (vacuity gap) and its M2-M5 cases run `cargo test -p nexus-setup` for a crate that does not exist until M2 (EP-001 masking class); M1 rewired to the real gate, M2-M5 left for their owning milestones with the unconditional-ok tail removed (rc propagation).
- 2026-08-21: pnpm-workspace.yaml needed `apps/setup` added and pnpm-lock.yaml gained the importer; both are M1-owned manifest changes recorded in the fence (EP-033 M2 precedent).
- 2026-08-21: Direct shell lacks dart/mise shims; `format-check.sh` must run under `scripts/env.sh` (node-verify path), matching the EP-020 env-shadowing lesson in reverse (shim needed, not shim-avoided).

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-21 | M1 contract package language is TypeScript under `apps/setup` (`@nexus/setup`, pnpm workspace), the Tauri frontend layer per ARCHITECTURE.md; the Rust behavior crate `crates/nexus-setup` is owned by M2 and the onboarding UI package `packages/onboarding` by M3. Evidence: ARCHITECTURE.md repository map (apps/setup = Tauri onboarding, Rust + TS); M1/M2/M3 CHANGE lists. Alternatives: a full Tauri shell in M1 (UI/runtime, not contract boundary). Consequence: provider-neutral contract layer reusable across future UI/runtime implementations. Reversal: ADR + plan update. Security/license/compat: none.
- 2026-08-21 | Wizard/step/integration/trust/hardware/owner/recovery vocabularies (WIZARD_STATES, WIZARD_STEPS, WIZARD_STEP_STATUSES, INTEGRATION_STATUSES, ENROLLMENT_TRUST_STATES, HARDWARE_PROVENANCES, OWNER_BOOTSTRAP_STATES, RECOVERY_OUTCOMES, RECOVERY_FAILURE_CLASSES, RECOVERY_MATERIAL_KINDS, DEPLOYMENT_VERIFICATION_STATES, CAPABILITY_CERTIFICATION_STATES) are EP-035-owned additions from the node directive's canonical candidate lists; SPEC-004/016 do not define these enums. Schema-parity tests bind DeploymentProfile + RecoveryKit to their canonical JSON schemas verbatim. Evidence: apps/setup contract files + ep035_unit_schema_parity. Alternatives: inventing parallel names (forbidden). Consequence: single canonical setup vocabulary in the barrel. Reversal: ADR + schema update. Security/license/compat: none.
- 2026-08-21 | State truthfulness is structural, not cosmetic: COMPLETE_LOCAL != VERIFIED, SELECTED != PROVISIONED/VERIFIED, DISCOVERED != TRUSTED, CONFIGURED != HEALTHY, OWNER_DETAILS != OWNER_AUTHORIZED, LOCAL_CHECKPOINT != REMOTE_VERIFIED, AMBIGUOUS_MUTATION != SAFE_TO_RETRY. Each boundary has an explicit typed transition requiring evidence. Evidence: wizard/deployment/hardware/owner/enrollment/integration/recovery tests. Alternatives: single status booleans (forbidden by directive). Consequence: fail-closed truthfulness invariants. Reversal: none without directive change. Security/license/compat: none.
- 2026-08-21 | First-owner bootstrap semantics (INITIALIZED/ALREADY_INITIALIZED/CONFLICT) are encoded as a deterministic pure contract with durable enforcement explicitly deferred to M2 (crates/nexus-setup). Evidence: owner.ts resolveFirstOwnerRequest + tests. Alternatives: in-memory enforcement in M1 (would be a fake durable store). Consequence: honest boundary (contract proven, provider deferred). Reversal: M2. Security/license/compat: none.
- 2026-08-21 | Enrollment credentials (BootstrapToken) classify secret/nonce as SECRET: toJSON/toString/redacted() never emit them; summaries use the redacted shape. Evidence: ep035_unit_enrollment canary tests. Alternatives: emitting secrets to summaries (forbidden). Consequence: no secret leakage path in the contract layer. Reversal: none. Security/license/compat: none.
- 2026-08-21 | M2-M5 milestone manifests trimmed to comments and expected-files trimmed to M1 scope because the listed future paths do not exist yet (EP-034 M1 precedent; artifact check validates ALL milestone manifests even at M1). Evidence: .agent/milestone-files/EP-035-M{2..5}.txt + expected-files. Alternatives: creating empty placeholder dirs (forbidden by directive Y). Consequence: fence matches reality; repopulated at each owning milestone. Reversal: none. Security/license/compat: none.
- 2026-08-21 | M1 makes no provider/hardware/deployment claims: real host deployment, hardware probing, owner provisioning, edge enrollment, discovery, provider authentication, integration health, and external recovery are NOT ASSERTED and owned by M2-M5 + native/deployment milestones. Evidence: certification boundary in progress + node contract reality rule. Alternatives: claiming provider certification from contract tests (forbidden). Consequence: honest INTERNAL CONTRACT CERTIFIED only. Reversal: none. Security/license/compat: none.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
