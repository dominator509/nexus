NODE-META-BEGIN
ID: EP-024
DEPS: EP-023
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-024
VERIFY_SENTINEL: node verify EP-024: ok
GREEN_TAG: green/EP-024
NODE-META-END

# 1. Purpose / Big Picture

Implement Sonos, TV, media, lighting, HVAC, vacuum, irrigation, appliance, vehicle, and future robot provider contracts. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-024.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-024.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-023` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-024.md`
- `.agent/specs/SPEC-011-home-devices-media-appliances-irrigation-and-robotics-providers.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-024.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-024-media-appliances-irrigation-and-robotics-providers.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-024.txt`
- `.agent/node-contracts/EP-024.md`
- `scripts/nodes/EP-024.sh`
- `crates/nexus-devices/`
- `connectors/media/`
- `connectors/appliances/`
- `connectors/irrigation/`
- `connectors/vacuum/`
- `connectors/robotics/`
- `tests/devices/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `MediaProvider` | `nexus-devices` | Defined by EP-024; provider-neutral and versioned |
| `ApplianceProvider` | `nexus-devices` | Defined by EP-024; provider-neutral and versioned |
| `IrrigationProvider` | `nexus-devices` | Defined by EP-024; provider-neutral and versioned |
| `VacuumProvider` | `nexus-devices` | Defined by EP-024; provider-neutral and versioned |
| `RobotProvider` | `nexus-devices` | Defined by EP-024; provider-neutral and versioned |
| `DeviceCapabilityMapper` | `nexus-devices` | Defined by EP-024; provider-neutral and versioned |
| `DeviceCommandVerifier` | `nexus-devices` | Defined by EP-024; provider-neutral and versioned |

Acceptance obligations:

1. Home Assistant is preferred for commodity devices
2. Direct providers exist only for capability or reliability gaps
3. Commands are target-scoped and verified
4. Future robots receive no broader authority than declared capabilities

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement sonos, tv, media, lighting, hvac, vacuum, irrigation, appliance, vehicle, and future robot provider contracts.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-024-M1.txt`, `.agent/node-contracts/EP-024.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-024-media-appliances-irrigation-and-robotics-providers.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-024.txt`, `.agent/node-contracts/EP-024.md`, `scripts/nodes/EP-024.sh`, `crates/nexus-devices/`, `connectors/robotics/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep024_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-024.sh M1`

EXPECT:

- `EP-024 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-024 MILESTONE_PASS "M1 EP-024 M1: ok"`

FALLBACK: Expose Home Assistant-backed capabilities only and mark direct integrations unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-024][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-024.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-024-M2.txt`, `.agent/node-contracts/EP-024.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/media/`, `tests/devices/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep024_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-024.sh M2`

EXPECT:

- `EP-024 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-024 MILESTONE_PASS "M2 EP-024 M2: ok"`

FALLBACK: Expose Home Assistant-backed capabilities only and mark direct integrations unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-024][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-024 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-024-M3.txt`, `.agent/node-contracts/EP-024.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/appliances/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep024_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-024.sh M3`

EXPECT:

- `EP-024 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-024 MILESTONE_PASS "M3 EP-024 M3: ok"`

FALLBACK: Expose Home Assistant-backed capabilities only and mark direct integrations unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-024][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-024 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-024-M4.txt`, `.agent/node-contracts/EP-024.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/irrigation/`

CONTENT:

1. Create tests whose names begin `ep024_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-024.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-024 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-024 MILESTONE_PASS "M4 EP-024 M4: ok"`

FALLBACK: Expose Home Assistant-backed capabilities only and mark direct integrations unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-024][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-024.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-024-M5.txt`, `.agent/node-contracts/EP-024.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/vacuum/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-024` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-024.sh M5`
2. `sh scripts/node-verify.sh EP-024`
3. `sh scripts/scope-audit.sh EP-024`

EXPECT:

- `EP-024 M5: ok`
- `node verify EP-024: ok`
- `scope audit EP-024: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-024 MILESTONE_PASS "M5 EP-024 M5: ok"`

FALLBACK: Expose Home Assistant-backed capabilities only and mark direct integrations unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-024][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-024` and observe `node verify EP-024: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [x] M2: Core behavior and deterministic invariants
- [x] M3: Real dependency and transport integration
- [x] M4: Forced failures, abuse cases, and observability
- [x] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

- 2026-08-17 M1: Pre-created node script M1 gate was artifact-only (EP-001 masking class: `node-artifact-check.py` only, no test execution). Replaced with `scripts/ep024-m1-tests.sh` (real cargo suite + vacuity guards) per the EP-019..EP-023 precedent.
- 2026-08-17 M1: SPEC-011 canonical capability taxonomy differs from intuition - the domain vocabulary uses `CapabilityClass::Query/Command/Workflow/Stream/Administrative`, `Risk::R0..R4`, `ApprovalClass::None/Policy/Human/StrongHuman/FourEyes`, `Idempotency::NotApplicable/Optional/Required`. The mapper uses exactly these locked values (verified from nexus-domain sources), never invented classes.
- 2026-08-17 M2: Pre-created node script M2 gate ran the M1 contract suite (`cargo test -p nexus-devices ep024_unit`) - EP-001 masking class. Replaced with `scripts/ep024-m2-tests.sh` (nexus-media + nexus-devices-e2e real suites + vacuity guards).
- 2026-08-17 M2: The media adapter uses `Mutex` interior mutability (not `RefCell`) so the in-flight idempotency guard is provable under a real concurrent duplicate; the adapter is thread-safe when the transport is `Send + Sync`.
- 2026-08-17 M2: Exact-target verification fails closed with NotFound when the transport cannot observe the target - the adapter never invents a state or a Verified outcome for an unobservable device; availability truth table maps both Unavailable and NotFound to UNAVAILABLE (configured != reachable != streaming).
- 2026-08-17 M3: Pre-created node script M3 gate ran a nonexistent filter against the M1 crate (`cargo test -p nexus-devices ep024_integration`) - EP-001 masking class. Replaced with `scripts/ep024-m3-tests.sh` (real appliance suite against the REAL pinned HA container + vacuity guards).
- 2026-08-17 M3: This pinned HA build REJECTS the legacy `fan: - platform: template` layout ("must be configured under its own template key") and the `percentage_template` option; the modern layout is `template: - fan:` with a `percentage` template key (verified from the container's fan.py schema constants). The fixture binds to the observed API shape.
- 2026-08-17 M3: `fan.turn_on` without a percentage returns 400: the provider passes `percentage=None` into the turn_on action and Jinja `default(100)` does NOT apply to None (only undefined) - `input_number.set_value(None)` is rejected. The fixture action handles None explicitly (`percentage if percentage is not none else 100`); adapter PowerOn stays provider-neutral (no percentage injected by the adapter).
- 2026-08-17 M3: HA reports fan percentage as a JSON number (37.0); the adapter normalizes integral floats to their exact integer form ("37") for canonical readback/verification, preserving the exact canary value. The template fan reports state "unknown" until first actuation - mapped as present+usable (AVAILABLE) but never claimed as OFF.
- 2026-08-17 M3: The EP-020 `RestTransport` reports non-2xx as External (no 404 classification), so unknown-entity NotFound is proven by real /api/states registry membership, never by parsing HTTP status text.
- 2026-08-17 M4: The first M4 concurrency proof (thread::park with no guaranteed unpark) DEADLOCKED. The Condvar replacement ALSO deadlocked - because the PRODUCTION adapter held the observability lock across the entire provider dispatch: thread 1 blocked in invoke while holding it, so the duplicate blocked on `observability.lock()` BEFORE reaching the in-flight check. Fixed by scoping the observability lock to correlation minting + record calls (never across provider I/O). Root cause was in the adapter, not the test.
- 2026-08-17 M4: HA template binary_sensor NORMALIZES non-"on" template renders to "off" - an honest unknown-state fixture requires a template SENSOR (renders the literal template string; `sensor.nexus_zone_unknown` reports the real state "unknown").
- 2026-08-17 M4: A malformed-JSON fake peer that drops the socket immediately surfaces as a truncated-connection error (UNAVAILABLE) instead of a parse failure (EXTERNAL); holding the connection ~500ms after the body makes the malformed body fully readable and the fail-closed classification deterministic.
- 2026-08-17 M4: EP-020 `RestTransport` had NO request timeout and mapped every transport error to Unavailable - a silent/stalled provider would hang an irrigation command forever and TIMEOUT was unrepresentable. M4 added `RestTransport::with_timeout` + `is_timeout()` -> Timeout classification (additive; `new()` unchanged - appliance/M3 semantics preserved; M3 regression re-run green).
- 2026-08-17 M4: The first `EP-024 M4: ok` was FALSE-GREEN in the restore proof: `irrigation-diag status` treated `Ok(Unavailable)` as healthy (the poll could succeed while zones were UNAVAILABLE/NOT_FOUND mid-reload). Fixed: only AVAILABLE is healthy; capability errors also mark DEGRADED. Re-run requires genuine fresh-readback AVAILABLE before `EP-024 M4: ok`.
- 2026-08-17 M4: Real 401 (bad credential) surfaces as External at the EP-020 boundary (documented contract); the authorization gate is `auth_check() == Ok(false)`. Recorded honestly - never relabeled as a fabricated Authorization code.
- 2026-08-17 M5: The pinned HA build's vacuum `supported_features` bits differ from the classic table: the template vacuum (start+pause+return_to_base configured) publishes 12308 = START(4096) | STATE(8192) | PAUSE(4) | RETURN_HOME(16). The adapter's START bit was 2048 initially - the live capability probe caught the mismatch and the constants are bound to the OBSERVED provider value (never invented).
- 2026-08-17 M5: A template vacuum with start/pause/return_to_base configured is the honest controlled vacuum fixture: real state derives from an input_select helper, real actions (vacuum.start/pause/return_to_base) mutate it, and a real automation performs the RETURNING -> DOCKED transition after 5s - so RETURNING is observable and distinct from DOCKED (the fixture never jumps straight to docked).
- 2026-08-17 M5: MapReadback is NOT advertised by the fixture (no real map surface) and fails closed with Policy - the honest anti-slop boundary (REAL map data only). The map provider path is NOT CERTIFIED; physical SLAM is NOT ASSERTED.

# 13. Decision Log

- 2026-08-17 M1: Create `crates/nexus-devices` as the provider-neutral contract crate for MediaProvider/ApplianceProvider/IrrigationProvider/VacuumProvider/RobotProvider/DeviceCapabilityMapper/DeviceCommandVerifier. Evidence: SPEC-011 behaviors 5-7, node contract. Alternatives: fold into nexus-home (rejected: nexus-home owns the Home Assistant provider surface; devices are provider-neutral across media/appliance/irrigation/vacuum/robot classes). Consequence: callers import one device surface; later milestones add connectors behind ports. Reversal: rename crate under ADR. Security: no new authority; robot activation gated by safety declaration. License: MIT. Compatibility: workspace member addition only.
- 2026-08-17 M1: Create `connectors/robotics` as the fail-closed robotics connector with real safety-declaration gating but no fabricated hardware inventory (Reality rule; acceptance obligation 4). Evidence: SPEC-011 behavior 6, EP-023 roku-home precedent. Consequence: robot activation refuses until real hardware certification; the gating rule is proven now so a future bound robot cannot bypass it.
- 2026-08-17 M2: Create `connectors/media` (nexus-media) as the real media adapter core behind a `MediaTransport` port. Evidence: SPEC-011 behaviors 1-3/5, EP-023 frigate-adapter precedent. Alternatives: implement a concrete Sonos/HA transport now (rejected: no certified hardware/transport on this host; the adapter core is real, the transport binds later behind the same port). Consequence: media commands are target-scoped, idempotent, and verified through exact-target readback; unbound transports fail closed. Reversal: rename crate under ADR. Security: capability-gated commands, redacted transport errors. License: MIT. Compatibility: workspace member addition only.
- 2026-08-17 M2: Create `tests/devices` (nexus-devices-e2e) composing nexus-devices + nexus-media + nexus-robotics to prove all four acceptance obligations in one suite. Evidence: tests/vision precedent (EP-023). Consequence: cross-component composition is proven at M2; live-fire stays owned by later milestones.
- 2026-08-17 M3: Create `connectors/appliances` (nexus-appliances) as the real appliance adapter composed through the EP-020-certified Home Assistant boundary (`nexus-home-assistant::RestTransport`), NOT a second HA OAuth/REST client. Evidence: SPEC-011 behavior 5 (HA preferred for commodity devices), owner directive A/M (composition through the existing production provider surface; authentication stays EP-020-owned with a fresh token per run). Alternatives: implement an independent HA client (rejected: duplicates EP-020's certified transport; violates the ownership split - EP-020 owns HA transport/provider semantics, EP-024 owns appliance semantics). Consequence: the appliance transport is the narrowest provider-neutral port over the certified REST surface; unknown entities are NotFound by real registry membership; capability mapping derives from real entity features (never category defaults); commands are capability-gated (Policy before any provider call), exact-target verified, SUBMITTED never VERIFIED. Reversal: rename crate under ADR. Security: no credentials in the tree; token minted per run by the fixture bootstrap (EP-020-certified flow); zero secret leakage proven in the journey. License: MIT. Compatibility: workspace member addition only.
- 2026-08-17 M4: Create `connectors/irrigation` (nexus-irrigation) as the real irrigation adapter composed through the EP-020-certified Home Assistant boundary (`nexus-home-assistant::RestTransport`), NOT a second HA OAuth/REST client. Evidence: SPEC-011 behavior 5, owner directive A/G (composition through the existing production provider surface; EP-020 owns HA transport semantics, EP-024 owns irrigation semantics). Consequence: HaIrrigationTransport composes `RestTransport::with_timeout(10s)` (bounded request timeout - a silent/stalled provider -> TIMEOUT, closed endpoint -> UNAVAILABLE distinct, never a hang, never a fabricated outcome); unknown zones NotFound by real registry membership; capability mapping from real entity features (fixture zones ZoneControl only; schedule/moisture never advertised); commands capability-gated (Policy before any provider call), exact-target verified, SUBMITTED never VERIFIED; no blind retry of ambiguous watering ops (directive I - SUBMITTED/UNKNOWN preserved). Reversal: rename crate under ADR. Security: no credentials in the tree; token minted per run; observability redaction at insert; zero secret leakage proven. License: MIT. Compatibility: workspace member addition + additive EP-020 transport change (registered in scope fence).
- 2026-08-17 M4: Deterministic bounded concurrency proof (directive C): replaced the deadlocking thread::park with a Condvar state object (GateState { entered, blocked }) on the fixture transport - `invoke` signals entered before parking; the test waits with `wait_timeout` (bounded, fails on timeout) then submits the duplicate. The REAL production fix uncovered by the deadlock: the observability lock is never held across the provider dispatch (correlation minted under a short lock; records under a fresh lock). Evidence: ep024_unit_irrigation_duplicate_inflight_command_conflicts + bounded_recovery tests; both pass in ~0.01s.
- 2026-08-17 M4: In-process in-flight idempotency: PASS (real concurrent duplicate -> Conflict before any second provider mutation; completion releases the entry - retry after completion is NOT a Conflict; failures release the entry - recover() clears 0). Crash-durable idempotency: NOT ASSERTED (in-flight state is process-local; recorded honestly per directive D).
- 2026-08-17 M4: Parameter bounds (directive J): the irrigation contract owns binary zone commands (ZoneOn/ZoneOff); SetSchedule is capability-denied. No duration/percentage/flow/zone-count parameters exist in this contract, so parameter bounds are N/A; malformed parameters are rejected at the vocabulary boundary (unknown command -> Vocabulary) BEFORE any provider call. Any future parameterized irrigation command MUST validate bounds before dispatch (recorded obligation).
- 2026-08-17 M4: Observability (directive K/L): bounded redacted audit ring (256 entries, secrets redacted at insert), counters {op}:{outcome}, canonical correlation `irrigation-<nanos>-<seq>` minted per operation and preserved through command -> provider -> readback -> audit AND on every returned error path (Conflict/Policy/Verification/transport). No upstream correlation surface exists in the contract; local canonical id generated only when none exists.
- 2026-08-17 M4: `irrigation-diag` (directive M/N): status = auth check + zone discovery + per-zone availability/capabilities + counters + redacted audit tail; recover = bounded recovery (clears stuck in-flight entries) then fresh status. It NEVER opens valves, starts watering, resets authorization, or fabricates AVAILABLE/VERIFIED. Only AVAILABLE is healthy (a zone the provider does not report as present+usable is DEGRADED - the restore proof requires genuine fresh-readback availability; no stale cache can produce recovery success).
- 2026-08-17 M4: M4 gate replaced the masking-class EP-024.sh M4 command (ran a nonexistent `nexus-devices ep024_failure` filter - EP-001 class) with `scripts/ep024-m4-tests.sh`: real unit + failure probe + live journey + diag healthy/recover/redaction + offline->diag FAIL + restore->diag healthy (fresh readback only) + vacuity guards + teardown + orphan check.
- 2026-08-17 M4: Certification (directive O): nexus-irrigation adapter REAL_PRODUCTION_IMPLEMENTATION; HA provider dependency PROVIDER_CERTIFIED (EP-020 + M4 composition); controlled zone fixtures CONTROLLED_TEST_FIXTURE; physical irrigation controller/valve + actual water flow NOT ASSERTED / DEFERRED to its exact owner. Logical valve state is never called physical-flow certification.
- 2026-08-17 M5: Create `connectors/vacuum` (nexus-vacuum) as the real vacuum adapter composed through the EP-020-certified Home Assistant boundary (`nexus-home-assistant::RestTransport::with_timeout`), NOT a second HA OAuth/REST client. Evidence: SPEC-011 behavior 5, owner directive B/G (EP-020 owns HA transport/provider semantics, EP-024 owns vacuum semantics). Consequence: real capability discovery from the OBSERVED provider feature bits (template vacuum publishes START=4096/PAUSE=4/RETURN_HOME=16; never assumed); real StartClean -> CLEANING, Pause -> PAUSED, ReturnHome -> RETURNING -> DOCKED (real auto-dock automation; RETURNING distinct from DOCKED); Dock and ReturnHome are distinct Nexus capabilities mapping to the SAME provider action (vacuum.return_to_base) - recorded explicitly, not two fabricated behaviors; MapReadback REAL data only (safe metadata digest/dimensions/reference, never raw household imagery; no map surface on the fixture -> not advertised -> Policy fail closed); no blind retry of ambiguous physical commands (UNKNOWN OUTCOME -> VERIFY FIRST); in-flight idempotency (duplicate -> Conflict, completion releases entry, crash-durable NOT ASSERTED); bounded redacted audit ring + counters + canonical vacuum-<nanos>-<seq> correlation on every error path; vacuum-diag status/recover (never starts/stops a vacuum; only AVAILABLE healthy - fresh readback only). Reversal: rename crate under ADR. Security: no credentials in tree; redaction at insert; zero secret leakage proven; a valid HA credential is infrastructure access only, never cleaning/map/robot authority (EP-008 is final authorization authority). License: MIT. Compatibility: workspace member addition only.
- 2026-08-17 M5: Robot safety regression (directive Q): EP-024 M1 RobotSafetyDeclaration rerun green - vacuum support does NOT widen RobotProvider authority, caller permission, EP-008 authorization, or tenant scope; nexus-vacuum exposes no robot interface and never claims robot authority.
- 2026-08-17 M5: M5 gate replaced the masking-class EP-024.sh M5 command (reran the M1 contract suite `cargo test -p nexus-devices` - EP-001 class) with `scripts/ep024-m5-tests.sh`: real unit + failure probe + live journey (restart/offline/recovery phases) + diag healthy/recover/redaction + offline->diag FAIL + restore->diag healthy (fresh readback only) + vacuity guards + teardown + orphan check.
- 2026-08-17 M5: Certification (directive X/L/O): nexus-vacuum adapter INTERNAL_CERTIFIED after the real M5 live-fire; HA vacuum provider path PROVIDER_CERTIFIED (EP-020 composition + real vacuum proof); controlled template vacuum fixtures CONTROLLED_TEST_FIXTURE; vacuum map path NOT CERTIFIED (no real map exercised); physical robot vacuum / SLAM map NOT ASSERTED / DEFERRED; RobotProvider hardware NOT ASSERTED. Home Assistant state is never represented as physical robot motion.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
