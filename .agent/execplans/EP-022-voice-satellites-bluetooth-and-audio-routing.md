NODE-META-BEGIN
ID: EP-022
DEPS: EP-021
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-022
VERIFY_SENTINEL: node verify EP-022: ok
GREEN_TAG: green/EP-022
NODE-META-END

# 1. Purpose / Big Picture

Implement Assist and Wyoming satellites, top-ten hardware matrix, Bluetooth endpoints, AEC, endpoint transfer, and room routing. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-022.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-022.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-021` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-022.md`
- `.agent/specs/SPEC-012-voice-speech-wake-word-speaker-evidence-satellites-bluetooth-and-audio-routing.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-022.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-022-voice-satellites-bluetooth-and-audio-routing.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-022.txt`
- `.agent/node-contracts/EP-022.md`
- `scripts/nodes/EP-022.sh`
- `crates/nexus-audio/`
- `connectors/assist-satellite/`
- `connectors/wyoming/`
- `connectors/bluetooth-audio/`
- `tests/audio/`
- `hardware/voice/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `AudioEndpoint` | `nexus-audio` | Defined by EP-022; provider-neutral and versioned |
| `VoiceSatellite` | `nexus-audio` | Defined by EP-022; provider-neutral and versioned |
| `AssistSatelliteProvider` | `nexus-audio` | Defined by EP-022; provider-neutral and versioned |
| `WyomingProvider` | `nexus-audio` | Defined by EP-022; provider-neutral and versioned |
| `BluetoothEndpointProvider` | `nexus-audio` | Defined by EP-022; provider-neutral and versioned |
| `EndpointRouter` | `nexus-audio` | Defined by EP-022; provider-neutral and versioned |
| `ConversationTransfer` | `nexus-audio` | Defined by EP-022; provider-neutral and versioned |
| `EchoCancellationProfile` | `nexus-audio` | Defined by EP-022; provider-neutral and versioned |

Acceptance obligations:

1. Top ten hardware classes have conformance profiles
2. Bluetooth reconnect and endpoint transfer preserve conversation context
3. Room satellites remain locally functional
4. Input and output endpoints are selected by person, room, privacy, and availability

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement assist and wyoming satellites, top-ten hardware matrix, bluetooth endpoints, aec, endpoint transfer, and room routing.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-022-M1.txt`, `.agent/node-contracts/EP-022.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-022-voice-satellites-bluetooth-and-audio-routing.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-022.txt`, `.agent/node-contracts/EP-022.md`, `scripts/nodes/EP-022.sh`, `crates/nexus-audio/`, `hardware/voice/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep022_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-022.sh M1`

EXPECT:

- `EP-022 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-022 MILESTONE_PASS "M1 EP-022 M1: ok"`

FALLBACK: Certify Home Assistant Voice Preview Edition, Linux satellite, Android, and iOS first while other classes remain unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-022][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-022.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-022-M2.txt`, `.agent/node-contracts/EP-022.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/assist-satellite/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep022_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-022.sh M2`

EXPECT:

- `EP-022 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-022 MILESTONE_PASS "M2 EP-022 M2: ok"`

FALLBACK: Certify Home Assistant Voice Preview Edition, Linux satellite, Android, and iOS first while other classes remain unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-022][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-022 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-022-M3.txt`, `.agent/node-contracts/EP-022.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/wyoming/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep022_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-022.sh M3`

EXPECT:

- `EP-022 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-022 MILESTONE_PASS "M3 EP-022 M3: ok"`

FALLBACK: Certify Home Assistant Voice Preview Edition, Linux satellite, Android, and iOS first while other classes remain unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-022][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-022 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-022-M4.txt`, `.agent/node-contracts/EP-022.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/bluetooth-audio/`

CONTENT:

1. Create tests whose names begin `ep022_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-022.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-022 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-022 MILESTONE_PASS "M4 EP-022 M4: ok"`

FALLBACK: Certify Home Assistant Voice Preview Edition, Linux satellite, Android, and iOS first while other classes remain unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-022][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-022.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-022-M5.txt`, `.agent/node-contracts/EP-022.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/audio/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-022` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-022.sh M5`
2. `sh scripts/node-verify.sh EP-022`
3. `sh scripts/scope-audit.sh EP-022`

EXPECT:

- `EP-022 M5: ok`
- `node verify EP-022: ok`
- `scope audit EP-022: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-022 MILESTONE_PASS "M5 EP-022 M5: ok"`

FALLBACK: Certify Home Assistant Voice Preview Edition, Linux satellite, Android, and iOS first while other classes remain unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-022][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-022` and observe `node verify EP-022: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-026` `voice-endpoint-transfer`: Start a conversation on a room satellite, move it to a Bluetooth headset or mobile endpoint, and maintain user, task, and privacy context.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary (2026-08-16)
- [x] M2: Core behavior and deterministic invariants (2026-08-16)
- [x] M3: Real dependency and transport integration (2026-08-16)
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

M1 evidence: `EP-022 M1: ok` (16 ep022_unit tests; gate vacuity-guarded via
scripts/ep022-m1-tests.sh, EP-001 masking class correction); clippy -D
warnings clean; cargo fmt clean; scope audit EP-022: ok; reality gate: ok;
security check: ok; license gate: ok; blueprint validation: ok; format
check: ok; dependency audit: ok.

M2 evidence: `EP-022 M2: ok` (11 ep022_unit adapter integration tests via
scripts/ep022-m2-tests.sh vacuity guard + 5 in-crate unit tests, 3 suites
16 total); clippy -D warnings clean; workspace check 95 crates green; M1
regression 16 green; side gates ok (scope/reality/security/license/
blueprint/format/dependency).

M3 evidence: `EP-022 M3: ok` (4 ep022_integration tests via
scripts/ep022-m3-tests.sh vacuity guard against the REAL
rhasspy/wyoming-openwakeword container, digest
sha256:52cb1168731a1849fc28cf339c935fde58746bbabc94226668a40ef6ddf5d42b):
canonical describe/info handshake returns the real server advertising
openwakeword wake program with installed models; real
Kokoro-generated hey-jarvis.wav streamed through the real Wyoming
protocol (wyoming==1.10.0 client, MIT) produces a real Detection event
(hey_jarvis at 1000ms); real silence produces NotDetected (real
negative); dead server fails closed fast. Container lifecycle managed by
the gate (start -> protocol readiness -> suite -> zero-orphan teardown).
COMPONENT_REGISTRY.yaml row wyoming-openwakeword added (EP-020 M3
precedent). M1 16 green; M2 16 green; clippy clean; workspace check 95
crates green; side gates ok (scope/reality/security/license/blueprint/
format/dependency).

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-16: The pre-created M1 gate in scripts/nodes/EP-022.sh was
  artifact-only (`node-artifact-check.py` plus nothing) - EP-001
  gate-masking class. Replaced with scripts/ep022-m1-tests.sh which runs
  the real `cargo test -p nexus-audio ep022_unit` suite and fails closed
  when no test ran (vacuity guard), same correction pattern as
  EP-018/EP-019/EP-020/EP-021 M1 gates.
- 2026-08-16: The initial DeterministicRouter privacy branch failed open:
  with `policy.sensitive=true` and only shared-room (room-bound) output
  candidates, the router fell through to the room-based selection and
  returned the shared-room speaker. Test
  `ep022_unit_router_sensitive_never_shared_room_output` caught it at
  first run (13 passed / 1 failed). Fixed: sensitive output requires a
  person-bound endpoint; if none exists the router returns
  AudioErrorCode::NotFound (fail closed, LF-028 precedent, SPEC-012
  behavior 9). Availability or convenience never outranks privacy.
- 2026-08-16: The pre-created M2 gate in scripts/nodes/EP-022.sh ran
  `cargo test -p nexus-audio ep022_unit` against the M1 contract crate,
  not the M2 changed-files fence (connectors/assist-satellite/) - EP-001
  gate-masking class. Replaced with scripts/ep022-m2-tests.sh running the
  real nexus-assist-satellite ep022_unit suite with a vacuity guard.
- 2026-08-16: Assist satellite adapter core needs explicit I/O-agnostic
  ports (AudioSource/AudioFrameSink) and a local wake gate port; an
  unbound gate fails closed (UNAVAILABLE) and a satellite cannot start
  listening without a bound wake gate (SPEC-012 behavior 3 requires local
  wake; a satellite without local wake is not locally functional).
- 2026-08-16: The real rhasspy/wyoming-openwakeword container (latest,
  digest 52cb1168...d42b) is a tflite-lineage openwakeword 2.1.0 server;
  its bundled wake models (okay_nabu, hey_jarvis, etc.) are upstream
  fixtures. The M3 transport proof streams REAL Kokoro-generated audio
  and observes a REAL Detection event (hey_jarvis at 1000ms) through the
  REAL protocol. The Nexus-owned wake model is NOT swapped into the
  container; production wake-model certification remains DEFERRED per
  SPEC-019 (no bundled noncommercial weights; Nexus model stays
  Nexus-owned).
- 2026-08-16: Readiness probing for the Wyoming container must do a real
  protocol handshake, not a bare TCP connect - the port accepts at the
  kernel level before the app is ready (tests errored with empty events
  on first gate run). The gate now waits for the describe/info handshake.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-16 | Sensitive-never-shared routing is a hard invariant |
  DeterministicRouter refuses shared-room (room-bound, non-person) output
  for sensitive content and fails closed with NOT_FOUND when no
  person-bound output exists. Evidence:
  ep022_unit_router_sensitive_never_shared_room_output (private selected
  when available; NOT_FOUND when only shared). Alternatives: allow shared
  when no private exists (rejected: LF-028 precedent, SPEC-012 behavior
  9, directive section C). Consequence: privacy outranks availability.
  Reversal: only by spec change. Security: fail-closed.
- 2026-08-16 | Endpoint identity is the canonical ref, never the display
  name | AudioEndpointId is the authoritative identity; `name` is mutable
  metadata only. Router tie-breaks on stable endpoint id, not name.
  Evidence: ep022_unit_endpoint_identity_is_canonical_ref_not_display_name
  (two endpoints sharing a display name route by id deterministically).
  Alternatives: name-keyed routing (rejected: mutable, not unique).
  Security: stable identity prevents spoofing via renamed endpoints.
- 2026-08-16 | Conversation transfer preserves context and never
  implicitly upgrades privacy | DeterministicTransfer copies the
  conversation context exactly (session, principal, objective, privacy
  policy id, room, transcript, correlation id); privacy class is never
  mutated by transfer. A move to a more public endpoint requires the
  canonical router privacy decision (shared speaker never selected for
  sensitive content). Evidence:
  ep022_unit_transfer_preserves_conversation_context +
  ep022_unit_transfer_never_implicitly_upgrades_privacy. Security:
  no implicit privacy downgrade.
- 2026-08-16 | BluetoothDeviceRef is a provider-neutral contract, not
  transport certification | M1 defines stable device reference,
  state vocabulary (DISCONNECTED/CONNECTING/CONNECTED/RECONNECTING), and
  the BluetoothEndpointProvider port with fail-closed defaults. Real
  Bluetooth pairing/connection/audio transport is owned by the later
  milestone/node that implements it (connectors/bluetooth-audio). No
  claim of Bluetooth connectivity is made from contracts.
- 2026-08-16 | AEC profile is not AEC performance certification |
  EchoCancellationProfile defines endpoint class, aggressiveness bounds
  (0..=2), and noise-suppression flag; validation is real. Profile
  existence does not prove an AEC engine works on real hardware.
  hardware/voice/profiles.yaml records every hardware class as
  conformance DEFINED / physical certified NOT_ASSERTED; physical
  certification is later-owned and never upgraded from a YAML profile.
- 2026-08-16 | M1 gate vacuity correction | The pre-created M1 gate was
  artifact-only (EP-001 masking class); scripts/ep022-m1-tests.sh now
  runs the real cargo suite and requires at least one ep022_unit test
  passed. Evidence: `EP-022 M1: ok` observed from the real test output.
- 2026-08-16 | nexus-audio owns its typed identity | AudioRoomId and the
  audio endpoint surface are defined in-crate; nexus-domain PersonId is
  reused for person binding (serde helper serializes PersonId as its
  UUIDv7 string since nexus-domain has no serde derives). No wire binding
  ripple into nexus-domain.
- 2026-08-16 | Assist satellite adapter is I/O-agnostic with fail-closed
  ports | connectors/assist-satellite (nexus-assist-satellite) implements
  the M2 core: AssistSatelliteAdapter with visible SatelliteState
  (STOPPED/LISTENING/CAPTURING/HARDWARE_MUTED), local WakeGate port
  (Armed/Triggered), AudioSource/AudioFrameSink transport ports, and
  conversation context survival across stop and transfer. Unbound ports
  fail closed (UNAVAILABLE); start_listening refuses without a bound wake
  gate; hardware mute is authoritative (Policy error on listen, capture
  ignored, never auto-resumes on unmute). Transport certification (real
  mic/Bluetooth/Wyoming) is NOT claimed here - owned by M3/M4/M5.
  Evidence: 11 ep022_unit adapter integration tests + 5 in-crate tests.
  Alternatives: hard-wiring a microphone transport in M2 (rejected:
  no real hardware in this environment; Reality rule). Security:
  fail-closed gates, visible mute state (SPEC-012 behavior 9).
- 2026-08-16 | Wyoming transport integration uses the REAL protocol
  server | connectors/wyoming (Python connector + unittest suite) talks
  the canonical Wyoming protocol to the REAL
  rhasspy/wyoming-openwakeword container (digest
  52cb1168...d42b, Apache-2.0 classifier/MIT LICENSE text) over TCP.
  Real client library wyoming==1.10.0 (MIT) in the engine venv. Real
  Kokoro-generated audio -> real Describe handshake -> real Detection
  event; silence -> real NotDetected; dead server fails closed fast.
  The container wake models are upstream fixtures (tflite lineage); the
  Nexus-owned wake model is never placed inside the container and its
  production certification remains DEFERRED (SPEC-019; EP-021 M3 graph
  gap). COMPONENT_REGISTRY.yaml row added (EP-020 M3 precedent).
  Evidence: 4 ep022_integration tests; gate manages container lifecycle
  with zero-orphan teardown. Alternatives: an in-memory protocol stub
  (rejected: Reality rule - would prove nothing about the wire);
  swapping the Nexus model into the container (rejected: SPEC-019).
  Security: real wire behavior, fail-closed timeouts, no fabricated
  detections.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
