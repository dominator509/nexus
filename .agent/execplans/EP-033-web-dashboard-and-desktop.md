NODE-META-BEGIN
ID: EP-033
DEPS: EP-032
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-033
VERIFY_SENTINEL: node verify EP-033: ok
GREEN_TAG: green/EP-033
NODE-META-END

# 1. Purpose / Big Picture

Implement accessible React PWA, cloud dashboard, chat, operations center, approvals, settings, security console, and Tauri desktop. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-033.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-033.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-032` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-033.md`
- `.agent/specs/SPEC-004-user-experience-dashboard-desktop-and-onboarding.md`
- `.agent/specs/SPEC-017-web-desktop-ios-android-device-security-and-remote-control.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-033.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-033-web-dashboard-and-desktop.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-033.txt`
- `.agent/node-contracts/EP-033.md`
- `scripts/nodes/EP-033.sh`
- `apps/web/`
- `apps/desktop/`
- `packages/ui/`
- `tests/e2e/web/`
- `tests/accessibility/web/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `DashboardShell` | `@nexus/web` | Defined by EP-033; provider-neutral and versioned |
| `ChatWorkspace` | `@nexus/web` | Defined by EP-033; provider-neutral and versioned |
| `ObjectiveView` | `@nexus/web` | Defined by EP-033; provider-neutral and versioned |
| `ApprovalCenter` | `@nexus/web` | Defined by EP-033; provider-neutral and versioned |
| `FleetView` | `@nexus/web` | Defined by EP-033; provider-neutral and versioned |
| `SecurityConsole` | `@nexus/web` | Defined by EP-033; provider-neutral and versioned |
| `ProviderSettings` | `@nexus/web` | Defined by EP-033; provider-neutral and versioned |
| `AuditExplorer` | `@nexus/web` | Defined by EP-033; provider-neutral and versioned |

Acceptance obligations:

1. Web dashboard supports chat when phone use is impossible
2. PWA and Tauri share contracts without duplicating business logic
3. Core flows meet WCAG 2.2 AA checks
4. Realtime state, approvals, task graph, incidents, and settings are coherent

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement accessible react pwa, cloud dashboard, chat, operations center, approvals, settings, security console, and tauri desktop.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-033-M1.txt`, `.agent/node-contracts/EP-033.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-033-web-dashboard-and-desktop.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-033.txt`, `.agent/node-contracts/EP-033.md`, `scripts/nodes/EP-033.sh`, `apps/web/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep033_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-033.sh M1`

EXPECT:

- `EP-033 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-033 MILESTONE_PASS "M1 EP-033 M1: ok"`

FALLBACK: Ship responsive PWA first; Tauri remains a thin signed wrapper in the same node. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-033][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-033.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-033-M2.txt`, `.agent/node-contracts/EP-033.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `apps/desktop/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep033_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-033.sh M2`

EXPECT:

- `EP-033 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-033 MILESTONE_PASS "M2 EP-033 M2: ok"`

FALLBACK: Ship responsive PWA first; Tauri remains a thin signed wrapper in the same node. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-033][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-033 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-033-M3.txt`, `.agent/node-contracts/EP-033.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `packages/ui/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep033_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-033.sh M3`

EXPECT:

- `EP-033 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-033 MILESTONE_PASS "M3 EP-033 M3: ok"`

FALLBACK: Ship responsive PWA first; Tauri remains a thin signed wrapper in the same node. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-033][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-033 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-033-M4.txt`, `.agent/node-contracts/EP-033.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/e2e/web/`

CONTENT:

1. Create tests whose names begin `ep033_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-033.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-033 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-033 MILESTONE_PASS "M4 EP-033 M4: ok"`

FALLBACK: Ship responsive PWA first; Tauri remains a thin signed wrapper in the same node. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-033][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-033.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-033-M5.txt`, `.agent/node-contracts/EP-033.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/accessibility/web/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-033` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-033.sh M5`
2. `sh scripts/node-verify.sh EP-033`
3. `sh scripts/scope-audit.sh EP-033`

EXPECT:

- `EP-033 M5: ok`
- `node verify EP-033: ok`
- `scope audit EP-033: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-033 MILESTONE_PASS "M5 EP-033 M5: ok"`

FALLBACK: Ship responsive PWA first; Tauri remains a thin signed wrapper in the same node. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-033][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-033` and observe `node verify EP-033: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-005` `cross-device-continuity`: Start an objective by voice, continue in the web dashboard, approve on mobile, and receive the final artifact in the same task graph.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [ ] M2: Core behavior and deterministic invariants
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

### M1 completion (2026-08-20)

- `apps/web/` @nexus/web contract package created (TypeScript, framework-neutral): package.json, tsconfig (strict + noUncheckedIndexedAccess + exactOptionalPropertyTypes + verbatimModuleSyntax), tsconfig.build.json; depends only on `@nexus/contracts` (generated canonical bindings) + typescript/vitest dev deps.
- Eight public interfaces defined with typed validation and deny-unknown semantics: DashboardShell (canonical SPEC-004 navigation vocabulary, unknown routes fail closed), ChatWorkspace (typed messages, idempotent sends, chat-when-phone-impossible), ObjectiveView (typed ids + LF-005 continuity seam), ApprovalCenter (approval-class preservation, FOUR_EYES two distinct principals), FleetView, SecurityConsole, ProviderSettings (certification/route/cost/privacy/egress disclosure before activation; uncertified never activatable), AuditExplorer.
- Supporting contracts: AuthenticatedSession (auth-session schema parity), BusinessContext/BoundContext (explicit principal/tenant/business binding), ContextProjection (context-switch invalidation), ViewState (SPEC-004 six state kinds + connectivity CONNECTED/DEGRADED/OFFLINE/AUTH_EXPIRED/BACKEND_UNAVAILABLE + FRESH/STALE), PresentedCapability/KnownCapabilityVocabulary (VISIBLE != AUTHORIZED; unknown capability UNSUPPORTED), TypedCommandRequest/DispatchGate (typed capability ids, idempotency keys, invocation binding; auth-expiry fail closed, no blind replay), EventFilter/EventSubscription (EventEnvelope binding), PreferencePersistence (allowlist only; tokens/secrets/approval credentials refused), ThemePreference (theme never mutates authority), A11ySurface/FocusOrder (label/role/keyboard/focus/reduced-motion/non-color; NO WCAG claim), RedactedLogger/redact (safe fields only; canary-tested).
- Tests: 143 ep033_unit tests across 16 files (construction, validation, serialization, vocabulary rejection, dependency-direction, schema-parity against canonical schema files).
- Node script M1 rewired from EP-001-masking artifact-check branch to the real gate `scripts/ep033-m1-tests.sh` (10 guards: package/sources present, tsc --noEmit clean, non-zero passing, zero failures, zero skipped, dependency-direction observed, anti-masking ep033_unit_session + ep033_unit_capability sentinels, fence artifacts).
- Side gates: scope audit EP-033: ok; expected files EP-033: ok; reality gate: ok; license gate: ok; security check: ok (0 advisories); dependency audit: ok (blueprint ASCII clean); workspace install clean.
- Commands + sentinels: `sh scripts/ep033-m1-tests.sh` -> EP-033 M1: ok (143 tests); `sh scripts/nodes/EP-033.sh M1` -> EP-033 M1: ok (RC=0).

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-20 M1: The pre-created `scripts/nodes/EP-033.sh` M1 branch was EP-001-masking class (artifact-check only). Rewired to `scripts/ep033-m1-tests.sh` before any M1 sentinel could be claimed.
- 2026-08-20 M1: vitest emits ANSI color codes even under CI=true; gate greps required `sed -i 's/\x1b\[[0-9;]*m//g'` on the log (same class of issue EP-006 handled with sed in its node script).
- 2026-08-20 M1: `exactOptionalPropertyTypes` (workspace convention) rejects optional-property assignment of `undefined`; contract shapes use `field: string | undefined` (required field) instead of `field?: string` in internal shape interfaces.
- 2026-08-20 M1: The tool display layer masks `["NONE",`-shaped content as `***` in terminal output and mangles long digit/JWT-shaped strings in some writes; verified actual file bytes with od (files were correct; display-only). No functional impact.
- 2026-08-20 M1: auth-session.schema.json (and all nested schemas/) are NOT emitted by the contracts generator (top-level glob only), so AuthSession has no generated binding; the session contract binds field-for-field to the canonical schema and the schema-parity test reads the schema file to prevent drift.
- 2026-08-20 M1: `EventEnvelope.schema_version` generated type is the literal `"1.0.0"`; event filter version is a string, not a number.
- 2026-08-20 M1: pre-scaffolded `.agent/expected-files/EP-033.txt` listed all milestone dirs (apps/desktop/, packages/ui/, tests/e2e/web/, tests/accessibility/web/); trimmed to M1 scope following the EP-032 per-milestone fence-growth pattern (directive Y: no empty M2-M5 placeholders).
- 2026-08-20 M1: `pnpm` shell shim is wrapped by rtk-tee which collapses output/fails; use the real binary `/root/.local/share/mise/installs/pnpm/11.17.0/pnpm` (or PNPM_BIN) with output redirected to a file.
- 2026-08-20 M1: LF-005 (cross-device continuity) remains M5-owned; its runner `scripts/live-fire/LF-005.sh` currently delegates to `proof-runner.sh` which requires a `nexus-cli`/`nexusctl` proof registry that does not exist in this repo (no crate named nexus-cli) - the M5 milestone must rewire LF-005 to a real gate and create the proof, mirroring EP-031 LF-009 precedent.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-20 M1: M1 package boundary = `apps/web/` contract layer only (framework-neutral TypeScript). No React/Vite/Playwright in M1 (directive X: no premature full dashboard); React PWA rendering is M2/M3 work. Evidence: ExecPlan M1 CONTENT (workspace manifests + module roots + public interfaces + ep033_unit tests); alternatives considered: scaffolding the full React app in M1 (rejected: violates M1 scope and dependency-direction); consequence: desktop shell (M2) can import the same contracts; reversal: none; security: none; license: none (typescript/vitest already in workspace lockfile); compatibility: pnpm-lock.yaml updated (+13 lines) for the new importer.
- 2026-08-20 M1: Contract validation uses deny-unknown + typed enums mirroring the canonical schema `additionalProperties: false` and the Rust serde deny_unknown_fields pattern (EP-002/EP-032 precedent), so raw wire input can never fabricate vocabulary or authority. Evidence: 143 tests incl. vocabulary-rejection and schema-parity suites; alternatives: permissive validation (rejected: fail-closed doctrine); reversal: none; security: fail-closed by construction.
- 2026-08-20 M1: Session/business context is bound explicitly (AuthenticatedSession + BusinessContext + BoundContext) and context switch invalidates old projections (ContextProjection.requireCurrent fails closed). Evidence: ep033_unit_session tests (switch invalidation, cross-context refusal); alternatives: deriving context from a screen label (rejected: directive F/G); reversal: none.
- 2026-08-20 M1: M1 certification remains INTERNAL CONTRACT CERTIFIED; no provider, hardware, or deployment certification claimed (no React runtime, no browser, no WCAG scan in M1). WCAG 2.2 AA checks are owned by M3/M5 (Playwright + axe per SPEC-004 required tests); accessibility contracts in M1 are executable labels/roles/keyboard/focus/reduced-motion/non-color contracts only.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
