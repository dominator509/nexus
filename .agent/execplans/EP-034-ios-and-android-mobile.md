NODE-META-BEGIN
ID: EP-034
DEPS: EP-033
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-034
VERIFY_SENTINEL: node verify EP-034: ok
GREEN_TAG: green/EP-034
NODE-META-END

# 1. Purpose / Big Picture

Implement Flutter iOS and Android apps, passkeys, biometrics, voice, push, Bluetooth, approvals, remote controls, and secure local storage. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-034.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-034.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-033` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-034.md`
- `.agent/specs/SPEC-017-web-desktop-ios-android-device-security-and-remote-control.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-034.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-034-ios-and-android-mobile.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-034.txt`
- `.agent/node-contracts/EP-034.md`
- `scripts/nodes/EP-034.sh`
- `apps/mobile/`
- `packages/mobile-contracts/`
- `tests/e2e/mobile/`
- `tests/accessibility/mobile/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `MobileSession` | `apps/mobile` | Defined by EP-034; provider-neutral and versioned |
| `VoiceRemote` | `apps/mobile` | Defined by EP-034; provider-neutral and versioned |
| `ApprovalPrompt` | `apps/mobile` | Defined by EP-034; provider-neutral and versioned |
| `DeviceEnrollment` | `apps/mobile` | Defined by EP-034; provider-neutral and versioned |
| `BluetoothDiscovery` | `apps/mobile` | Defined by EP-034; provider-neutral and versioned |
| `SecureStore` | `apps/mobile` | Defined by EP-034; provider-neutral and versioned |
| `PushInbox` | `apps/mobile` | Defined by EP-034; provider-neutral and versioned |
| `RemoteControl` | `apps/mobile` | Defined by EP-034; provider-neutral and versioned |

Acceptance obligations:

1. iOS and Android share Flutter UI and use native modules for passkeys, biometrics, Bluetooth, secure stores, push, and background audio
2. High-risk approvals bind to device and user
3. Offline low-risk controls follow cached policy
4. Accessibility and large text remain functional

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement flutter ios and android apps, passkeys, biometrics, voice, push, bluetooth, approvals, remote controls, and secure local storage.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-034-M1.txt`, `.agent/node-contracts/EP-034.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-034-ios-and-android-mobile.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-034.txt`, `.agent/node-contracts/EP-034.md`, `scripts/nodes/EP-034.sh`, `apps/mobile/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep034_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-034.sh M1`

EXPECT:

- `EP-034 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-034 MILESTONE_PASS "M1 EP-034 M1: ok"`

FALLBACK: Disable unsupported background features per platform while preserving foreground chat, voice, push, and approvals. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-034][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-034.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-034-M2.txt`, `.agent/node-contracts/EP-034.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `packages/mobile-contracts/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep034_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-034.sh M2`

EXPECT:

- `EP-034 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-034 MILESTONE_PASS "M2 EP-034 M2: ok"`

FALLBACK: Disable unsupported background features per platform while preserving foreground chat, voice, push, and approvals. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-034][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-034 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-034-M3.txt`, `.agent/node-contracts/EP-034.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/e2e/mobile/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep034_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-034.sh M3`

EXPECT:

- `EP-034 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-034 MILESTONE_PASS "M3 EP-034 M3: ok"`

FALLBACK: Disable unsupported background features per platform while preserving foreground chat, voice, push, and approvals. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-034][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-034 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-034-M4.txt`, `.agent/node-contracts/EP-034.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/accessibility/mobile/`

CONTENT:

1. Create tests whose names begin `ep034_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-034.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-034 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-034 MILESTONE_PASS "M4 EP-034 M4: ok"`

FALLBACK: Disable unsupported background features per platform while preserving foreground chat, voice, push, and approvals. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-034][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-034.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-034-M5.txt`, `.agent/node-contracts/EP-034.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: Only the active ExecPlan progress, Decision Log, and ledger may change in this milestone.

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-034` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-034.sh M5`
2. `sh scripts/node-verify.sh EP-034`
3. `sh scripts/scope-audit.sh EP-034`

EXPECT:

- `EP-034 M5: ok`
- `node verify EP-034: ok`
- `scope audit EP-034: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-034 MILESTONE_PASS "M5 EP-034 M5: ok"`

FALLBACK: Disable unsupported background features per platform while preserving foreground chat, voice, push, and approvals. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-034][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-034` and observe `node verify EP-034: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-004` `multi-user-identity`: Enroll two adults and one restricted user; prove separate context, permissions, preferences, and mobile devices.
- `LF-022` `mobile-step-up`: Request a high-risk action by voice, refuse voice-only authorization, approve with mobile biometric and passkey, execute, and verify.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
  - Flutter 3.44.7 (stable, revision 84fc5cbb22) / Dart 3.12.2 via mise; `apps/mobile/` flutter package created.
  - Contract layer: 8 public interfaces (device, session, approvals, enrollment, voice, bluetooth, secure_store, push, remote) + supporting SPEC-017 vocabulary bound to canonical schemas; deny-unknown validation (rejectUnknownKeys then direct value readers); provider-neutral dependency direction (contract layer imports no provider packages).
  - Tests: 44 tests / 5 files green (`flutter test`: All tests passed), names begin `ep034_unit_`; serialization round-trips prove canonical schema parity; `flutter analyze` clean; `dart format` clean.
  - Gates observed: `scripts/ep034-m1-tests.sh` -> `EP-034 M1: ok`; `sh scripts/nodes/EP-034.sh M1` -> `EP-034 M1: ok` (exit 0).
  - Side gates: scope-audit EP-034 ok; reality-gate ok (after removing scaffold TODO placeholder); security-check ok; license-gate ok; dependency-audit ok; expected-files EP-034 ok.
  - Committed as `[EP-034][M1] contract, vocabulary, and package boundary`; committed-tree reproduction green; tree clean.
- [x] M2: Core behavior and deterministic invariants
  - `packages/mobile-contracts/` pure-Dart behavior package (nexus_mobile_contracts) path-depending on the nexus_mobile contract barrel.
  - ApprovalBindingService: high-risk approvals bind to device AND user (SPEC-017 behavior 4; node contract); actionable-prompt, usable-session, active-binding, device-match, principal-match, human-class-for-R3/R4 guards; idempotent exactly-once resolution with CONFLICT on divergent re-resolution (bounded in-memory ring store).
  - OfflinePolicyCache: offline low-risk controls follow cached policy (SPEC-017 behavior 6; node contract); only explicitly allowed entries cached; stale never actionable; R3/R4 never run from cache; unknown capability fails closed; deny-unknown CachedPolicyEntry wire parsing.
  - Telemetry: canonical TelemetryEvent context; SanitizingTelemetrySink redacts secret-shaped values (bearer/jwt/token/secret/password/api-key/authorization) before emission; never raw prompt content.
  - All failures typed SPEC-006 errors (policy/authorization/conflict/vocabulary/validation) with correlation preserved.
  - Tests: 38 tests / 4 files green, names begin `ep034_unit_`; `flutter analyze` clean; `dart format` clean.
  - Gates observed: `scripts/ep034-m2-tests.sh` -> `EP-034 M2: ok` (38 tests); `sh scripts/nodes/EP-034.sh M2` -> `EP-034 M2: ok` (exit 0).
  - Side gates: scope-audit EP-034 ok; reality-gate ok; security-check ok; license-gate ok; dependency-audit ok; expected-files EP-034 ok.
  - Committed as `[EP-034][M2] core behavior and deterministic invariants`; committed-tree reproduction green; tree clean.
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

- 2026-08-21: `requireString` with an empty allowlist incorrectly rejected all reads (empty set interpreted as deny-all). Fixed by reading values directly after `rejectUnknownKeys`; regression test retained for corrected semantics (no allowlist restriction vs empty set deny-all).
- 2026-08-21: Flutter scaffold `android/app/build.gradle.kts` ships a `// TODO: Add your own signing config` placeholder which fails the repo reality gate. Replaced with factual comment (release signing deferred to native release milestone; debug signing for dev); behavior unchanged.
- 2026-08-21: `expected-files.sh EP-034` fails while future-milestone paths (`packages/mobile-contracts/`, `tests/e2e/mobile/`, `tests/accessibility/mobile/`) are listed before they exist; trimmed to M1-owned paths, re-appended as milestones land (EP-033 incremental convention).
- 2026-08-21: Dart RegExp does not support the `(?i)` inline flag (FormatException: Invalid group); the analyzer `valid_regexps` lint caught it. Use the `caseSensitive: false` constructor parameter instead.
- 2026-08-21: Flutter analyzer `implementation_imports` lint rejects importing `package:nexus_mobile/src/contracts/*` from another package; behavior sources import the public barrel `package:nexus_mobile/nexus_mobile.dart` (which exports the full contract layer).

# 13. Decision Log

- 2026-08-21 | Mobile contracts reuse canonical schemas (SPEC-006 errors, SPEC-017 vocabulary); no mobile-only forked vocabulary. Evidence: schema-parity serialization tests in `apps/mobile/test/ep034_unit_serialization_test.dart`. Alternatives: hand-copying enums into Dart. Consequence: anti-drift regressions permanently retained. Reversal: schema update + ADR. Security/license/compat: none.
- 2026-08-21 | Native features intentionally deferred from M1: passkeys, biometrics, Bluetooth, push native modules, secure enclave/Keychain/Keystore, emulator/device runs, hardware certification - all NOT ASSERTED at M1. Evidence: M1 contract layer declares interfaces only; no native plugin implementations. Alternatives: implement native modules in M1. Consequence: honest certification boundary preserved (INTERFACE EXISTS != NATIVE PROVIDER IMPLEMENTED != DEVICE CERTIFIED). Reversal: later milestones. Security/license/compat: none.
- 2026-08-21 | Release signing for Android deferred to the native release milestone; debug signing retained for development builds. Evidence: `apps/mobile/android/app/build.gradle.kts` comment. Consequence: `flutter run --release` functional during development; no release artifact certified. Reversal: configure signing in M5/ship milestone. Security: no production signing keys exist.
- 2026-08-21 | M2 core behavior lives in `packages/mobile-contracts/` (fence) and path-depends on the `apps/mobile` contract barrel; no cycle within EP-034 because apps/mobile is closed after M1 and M3/M4 fences are tests-only. Evidence: pubspec path dependency + dependency-direction test. Alternatives: moving contracts into the behavior package (reopens M1) or forking vocabulary (forbidden). Consequence: single canonical vocabulary; behavior package is pure Dart. Reversal: package restructure ADR. Security/license/compat: none.
- 2026-08-21 | In-memory stores (InMemoryApprovalResolutionStore, InMemoryOfflinePolicyStore) are the real M2 layer for deterministic invariants; platform/durable storage and native providers are later milestones (NOT ASSERTED at M2). Evidence: port interfaces + in-memory implementations exercised by 38 tests. Alternatives: platform keychain in M2 (native milestone). Consequence: honest boundary (BEHAVIOR IMPLEMENTED != PLATFORM PERSISTED != DEVICE CERTIFIED). Reversal: later milestones. Security/license/compat: none.
- 2026-08-21 | Offline high-risk refusal and stale/unknown denials use SPEC-006 POLICY; divergent re-resolution uses CONFLICT; device/user binding violations use AUTHORIZATION. Evidence: typed-error assertions in M2 tests. Alternatives: UNAVAILABLE for offline refusals. Consequence: policy decisions are distinguishable from transport failures. Reversal: none without schema update. Security/license/compat: none.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
