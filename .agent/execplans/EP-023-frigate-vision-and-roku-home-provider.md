NODE-META-BEGIN
ID: EP-023
DEPS: EP-022
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-023
VERIFY_SENTINEL: node verify EP-023: ok
GREEN_TAG: green/EP-023
NODE-META-END

# 1. Purpose / Big Picture

Implement Frigate, go2rtc, camera capability provider, Roku discovery and fallback ladder, visitor events, and two-way audio where verified. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-023.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-023.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-022` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-023.md`
- `.agent/specs/SPEC-021-cameras-frigate-go2rtc-roku-home-visitor-identity-and-two-way-audio.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-023.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-023-frigate-vision-and-roku-home-provider.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-023.txt`
- `.agent/node-contracts/EP-023.md`
- `scripts/nodes/EP-023.sh`
- `crates/nexus-vision/`
- `connectors/frigate/`
- `connectors/roku-home/`
- `infra/frigate/`
- `tests/vision/`
- `hardware/cameras/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `CameraProvider` | `nexus-vision` | Defined by EP-023; provider-neutral and versioned |
| `FrigateProvider` | `nexus-vision` | Defined by EP-023; provider-neutral and versioned |
| `RokuHomeProvider` | `nexus-vision` | Defined by EP-023; provider-neutral and versioned |
| `StreamSource` | `nexus-vision` | Defined by EP-023; provider-neutral and versioned |
| `CameraEvent` | `nexus-vision` | Defined by EP-023; provider-neutral and versioned |
| `VisitorIdentity` | `nexus-vision` | Defined by EP-023; provider-neutral and versioned |
| `TwoWayAudioCapability` | `nexus-vision` | Defined by EP-023; provider-neutral and versioned |
| `CameraFallbackPlan` | `nexus-vision` | Defined by EP-023; provider-neutral and versioned |

Acceptance obligations:

1. Frigate events and streams enter canonical vision contracts
2. Roku capabilities use verified local, Roku cloud or web, Google Home, then browser fallback in that order
3. No unverified RTSP or ONVIF claim is made
4. Two-way audio is enabled only after live certification

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement frigate, go2rtc, camera capability provider, roku discovery and fallback ladder, visitor events, and two-way audio where verified.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-023-M1.txt`, `.agent/node-contracts/EP-023.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-023-frigate-vision-and-roku-home-provider.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-023.txt`, `.agent/node-contracts/EP-023.md`, `scripts/nodes/EP-023.sh`, `crates/nexus-vision/`, `hardware/cameras/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep023_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-023.sh M1`

EXPECT:

- `EP-023 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-023 MILESTONE_PASS "M1 EP-023 M1: ok"`

FALLBACK: Use Roku official web or Google Home bridge for available operations and retain microSD as independent recording, without claiming network retrieval. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-023][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-023.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-023-M2.txt`, `.agent/node-contracts/EP-023.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/frigate/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep023_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-023.sh M2`

EXPECT:

- `EP-023 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-023 MILESTONE_PASS "M2 EP-023 M2: ok"`

FALLBACK: Use Roku official web or Google Home bridge for available operations and retain microSD as independent recording, without claiming network retrieval. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-023][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-023 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-023-M3.txt`, `.agent/node-contracts/EP-023.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/frigate/` (owner directive supersedes the pre-wired `connectors/roku-home/` line: M3 proves the REAL Frigate/go2rtc media chain; Roku stays a layered provider ladder with no hardware)

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep023_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-023.sh M3`

EXPECT:

- `EP-023 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-023 MILESTONE_PASS "M3 EP-023 M3: ok"`

FALLBACK: Use Roku official web or Google Home bridge for available operations and retain microSD as independent recording, without claiming network retrieval. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-023][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-023 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-023-M4.txt`, `.agent/node-contracts/EP-023.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/frigate/`

CONTENT:

1. Create tests whose names begin `ep023_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-023.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-023 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-023 MILESTONE_PASS "M4 EP-023 M4: ok"`

FALLBACK: Use Roku official web or Google Home bridge for available operations and retain microSD as independent recording, without claiming network retrieval. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-023][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-023.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-023-M5.txt`, `.agent/node-contracts/EP-023.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/vision/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-023` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-023.sh M5`
2. `sh scripts/node-verify.sh EP-023`
3. `sh scripts/scope-audit.sh EP-023`

EXPECT:

- `EP-023 M5: ok`
- `node verify EP-023: ok`
- `scope audit EP-023: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-023 MILESTONE_PASS "M5 EP-023 M5: ok"`

FALLBACK: Use Roku official web or Google Home bridge for available operations and retain microSD as independent recording, without claiming network retrieval. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-023][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-023` and observe `node verify EP-023: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-008` `visitor-response`: Receive a camera person event, identify known or unknown, notify the right user, and play an approved response through two-way audio where certified.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary (2026-08-17)
- [x] M2: Core behavior and deterministic invariants (2026-08-17)
- [x] M3: Real dependency and transport integration (2026-08-17)
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

M1 evidence: `EP-023 M1: ok` (13 ep023_unit tests via
scripts/ep023-m1-tests.sh vacuity guard): crates/nexus-vision
contract crate (SPEC-021 canonical terms): CameraId typed bounded
identity; CameraCapability vocabulary lock (OBJECT_DETECTION /
RECORDING / LIVE_STREAM / TWO_WAY_AUDIO / VISITOR_EVENTS / ROKU_CONTROL,
unknown rejected at parse, serde roundtrip); PrivacyClass;
RokuCapabilityTier with fixed ladder order (LOCAL_VERIFIED <
VENDOR_AUTHENTICATED < GOOGLE_HOME_BRIDGE < BROWSER_AUTOMATION <
UNAVAILABLE, SPEC-021 behavior 3); StreamRef with VerificationStatus
(no unverified RTSP/ONVIF claim - verified() requires real evidence
ref, acceptance obligation 3); CameraEvent (camera/time/object/zones/
confidence/media_refs/retention/privacy_class, SPEC-021 behavior 5)
+ ReviewItem + VisitorEvent with validation; VisitorIdentity
Known/Unknown with advisory_only enforced at construction (behavior 6,
never unlocks/disarms); TwoWayAudioCapability certified only when
verified speaker path + approval + disclosure + echo handling all
hold (behavior 7, acceptance obligation 4); CameraFallbackPlan
deterministic ladder selection + BrowserAutomationPolicy
(isolated/monitored/rate-limited/never stable API, behavior 4);
CameraProvider/FrigateProvider/RokuHomeProvider ports fail closed;
VisionError/VisionErrorCode SPEC-006 codes + redacted surface.
hardware/cameras/profiles.yaml: 6 camera classes (FRIGATE_NVR,
RTSP_CAMERA, ONVIF_CAMERA, ROKU_DEVICE, GOOGLE_HOME_BRIDGE,
BROWSER_AUTOMATION) all conformance DEFINED / physical certified
NOT_ASSERTED. clippy -D warnings clean; workspace check green; side
gates ok (scope audit EP-023: ok, reality gate: ok, security check:
ok, license gate: ok, blueprint validation: ok, format check: ok,
ok, dependency audit: ok). M1 gate vacuity fixed (pre-created gate was
artifact-only, EP-001 masking class).

M2 evidence: `EP-023 M2: ok` (28 ep023_unit tests via
scripts/ep023-m2-tests.sh vacuity guard): connectors/frigate
(nexus-frigate crate, SPEC-021; M2 fence): real production adapter
against the documented Frigate HTTP API and embedded go2rtc API -
FrigateTransport port + real RestTransport (health GET /api/, config
GET /api/config, events GET /api/events with after/limit, go2rtc
streams GET /api/go2rtc/streams, latest frame GET /api/{camera}/latest.jpg);
DTOs bound to the REAL provider shapes verified from upstream source
(FrigateConfig cameras map; CameraConfig enabled/detect/record/
snapshots/live/ffmpeg-inputs/audio; EventResponse id/label/camera/
start_time/zones/has_clip/has_snapshot/data with score in data.score;
go2rtc streams map name -> producers[{url}]/consumers). FrigateAdapter
implements CameraProvider + FrigateProvider via RefCell interior
mutability: camera discovery from config keys (stable identity, never
display name/list index - directive H), capability metadata (object
detection/visitor events from detect.enabled, recording, live stream;
TwoWayAudio NEVER advertised from config - directive M), stream refs
always Unverified (directive F/G/Q, acceptance obligation 3), event
mapping (data.score -> confidence, zones, absolute media refs from
provider base URL, PRIVATE default privacy - directive L), availability
mapping configured != reachable != streaming (DISCOVERED/AVAILABLE/
STREAMING/DEGRADED/UNAVAILABLE truth table - directive I/Q), snapshot
refs URL-only (no raw frames in events), redaction rtsp://user:pass@ ->
rtsp://***@ + query-secret masking (directive S). 28 tests prove:
response mapping, stable identity, availability mapping, exact stream
mapping, error handling (unavailable/timeout/malformed/not-found),
privacy boundaries, advisory visitor identity, two-way-audio gating,
Roku ladder ordering, no unverified RTSP/ONVIF claim, no secret
leakage. clippy -D warnings clean; workspace check green; M1 13 green;
side gates ok (scope audit EP-023: ok, reality gate: ok, security
check: ok, license gate: ok, blueprint validation: ok, format check:
ok, dependency audit: ok). M2 gate vacuity fixed (pre-created gate ran
the M1 contract suite ep023_unit, EP-001 masking class).

M3 evidence: `EP-023 M3: ok` (scripts/ep023-m3-tests.sh, real
provider/media chain; observed run /tmp/ep023-m3-gate-run5.log and
evidence /tmp/ep023-m3-evidence.json; node gate re-run also `EP-023 M3:
ok`). Pinned providers verified from real output: Frigate 0.17.2 at
sha256:d4351369984d4a9e2a49ac59736f6490856a7ea11f7790040746d21496967010
(API version `0.17.2-3d4dd3a`), embedded go2rtc v1.9.10 (df95ce3),
mediamtx v1.20.0 binary sha256
25947caac403f37ec881c9be213af2cad67e344a6c7098905b0d31c17f40e336
(CONTROLLED_TEST_FIXTURE transport). Real chain: host FFmpeg canary
(testsrc2 1280x360, unique NX3-<hex> token + localtime, fontsize 64)
-> mediamtx RTSP 8554 -> Frigate go2rtc producer -> detect pipeline ->
/api/nexus_front/latest.jpg readback. Phase A (source up): 9
ep023_integration_frigate tests green (version prefix match, discovery
stable identity, capabilities, availability STREAMING with live
producer, snapshot JPEG, events API, restart identity, redaction;
source-dead excluded via libtest --skip, canonical `--` boundary).
Live-fire proof: snapshots real (51868/43557 bytes, FFD8FF, sha256
differ over time), independent PIL decode 1920x1080, canary OCR ratio
0.917 (tesseract crop+2x upscale+normalized fuzzy match), go2rtc
producer evidence real (format_name rtsp, protocol rtsp+tcp, remote
172.17.0.1:8554, H264 SDP, user_agent go2rtc/1.9.10, bytes_recv
1401130), independent RTSP restream client decoded 8 real frames and
canary visible at ratio 0.917, no m3secret in /api/config or
/api/go2rtc/streams surfaces. Phase B (source killed): observed go2rtc
lose live evidence at +24s (bounded poll, real transition) ->
availability_source_dead green (never STREAMING, DEGRADED). Phase C
(source restarted): producer reattached +12s -> availability_recovered
green (STREAMING). Phase D (docker restart Frigate): producer reattached
+60s -> restart_same_identity green. Cross-phase accounting: 10/10
required integration tests executed (directive D). Zero-orphan teardown
verified. Sysctl hygiene (directive G): fs.inotify.max_user_watches
found 200000 (already sufficient), recorded, no change needed, verified
in teardown; mediamtx now runs from $WORK so auto.crt/auto.key certs
stay out of the repository. Adapter production fix: go2rtc bare-url
producer (dead source) is DEGRADED never STREAMING; live-producer
evidence (format_name/bytes_recv) required for STREAMING; capability
LiveStream requires live.streams non-empty OR an ffmpeg input with
roles (Frigate normalizes roles [] -> [record,detect] and auto-populates
live.streams, so nexus_secure truthfully reports Available + LiveStream
but never ObjectDetection/Recording/TwoWayAudio). M2 regression green:
50 unit tests (28 ep023_unit + 22 in-crate); clippy clean; format/lint/
security/license/dependency/reality gates ok; scope audit EP-023: ok
(after removing mediamtx-generated auto.crt/auto.key scratch).

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-17: The pre-created M2 gate in scripts/nodes/EP-023.sh ran
  `cargo test --locked -p nexus-vision ep023_unit` - the M1 contract
  suite - EP-001 gate-masking class. Replaced with
  scripts/ep023-m2-tests.sh which runs the real
  `cargo test -p nexus-frigate ep023_unit` suite (vacuity guarded) and
  a `test -s connectors/frigate/tests/ep023_unit_frigate.rs` artifact
  check, same correction pattern as every prior M2 gate.
- 2026-08-17: Frigate EventResponse has NO top-level score field; the
  real detection score lives in `data.score` (verified in Frigate
  source: min_score/max_score filters query Event.data["score"]). The
  adapter maps confidence from data.score and rejects events without a
  valid score rather than fabricating confidence.
- 2026-08-17: go2rtc `/api/streams` (as mounted by Frigate at
  /api/go2rtc/streams) returns a map of stream name -> {producers,
  consumers}; Producer.MarshalJSON emits `{"url": ...}`. A producer
  entry means a source is attached - provider metadata, NOT media-level
  proof (directive F/G/Q); the adapter keeps StreamRef Unverified.
- 2026-08-17: The nexus-vision provider ports take `&self`; the real
  REST transport is stateful. The FrigateAdapter uses RefCell interior
  mutability (single-threaded adapter) so the port methods drive the
  real transport without test-mode branches or duplicated _mut
  entry points.
- 2026-08-17: Structs carrying f32 confidence cannot derive Eq; the
  contract types use PartialEq + Serialize only.
- 2026-08-17: The scope audit fence for EP-023 did not authorize
  Cargo.toml/Cargo.lock; adding the workspace member required the
  fence to list them (EP-022 precedent).
- 2026-08-17: Frigate 0.17.2 NORMALIZES the config surface: `roles: []`
  becomes `["record","detect"]` and `live.streams` auto-populates for
  cameras with ffmpeg inputs. The M3 integration test initially
  asserted `caps.is_empty()` for the never-connecting nexus_secure
  camera; the real /api/config proves LiveStream IS declared. The
  truthful assertion: Available (configured + healthy + no go2rtc
  stream declared), LiveStream declared, but never ObjectDetection /
  Recording / TwoWayAudio (directive H/M).
- 2026-08-17: go2rtc does NOT drop dead-source producer evidence
  quickly: observed ~24-29s after killing the FFmpeg source on mediamtx
  1.20.0 before format_name/bytes_recv disappear. The gate now polls
  /api/go2rtc/streams (bounded 90s) for the REAL transition instead of
  a fixed short sleep; restart reattachment is likewise polled (observed
  +12s source restart, +60s Frigate container restart).
- 2026-08-17: cargo libtest prints "running 1 test" (singular); the
  vacuity regex `running [1-9][0-9]* tests` missed single-test phases.
  Fixed to `running [1-9][0-9]* test`.
- 2026-08-17: mediamtx generates auto.crt/auto.key in its CWD; running
  it from the repository root left cert scratch files that the scope
  audit flagged. The gate now launches mediamtx from $WORK (removed in
  teardown).
- 2026-08-17: tesseract confuses glyphs (S/5) on rendered video frames
  even at fontsize 64; the live-fire proof now crop+2x-upscales the
  canary region and fuzzy-matches (difflib >= 0.75; observed 0.917).
  A wrong/absent canary scores far lower, so the canary readback still
  defeats a canned camera-error image.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-17 | Contract-first vision crate with fail-closed ports |
  crates/nexus-vision owns the provider-neutral vision contracts;
  unbound CameraProvider/FrigateProvider/RokuHomeProvider fail closed
  and never fabricate cameras, events, or streams. Alternatives:
  provider code in the contract crate (rejected: EP-022/EP-020
  pattern separates contracts from connectors). Security: fail-closed
  ports, redacted errors.
- 2026-08-17 | No unverified RTSP/ONVIF claim | StreamRef::verified()
  requires a real evidence reference; unverified streams stay
  UNVERIFIED (acceptance obligation 3, SPEC-021 behavior 3).
  Evidence: ep023_unit_stream_ref_unverified_no_claim.
- 2026-08-17 | Known-person matching is advisory-only enforced at
  construction | KnownVisitor.advisory_only is always true; identity
  can never unlock or disarm by itself (SPEC-021 behavior 6).
  Evidence: ep023_unit_known_visitor_advisory_only +
  ep023_unit_visitor_identity_never_authorizes.
- 2026-08-17 | Two-way audio certified only after all gates | Verified
  speaker path, approval, disclosure, and echo handling are each
  mandatory; certify() fails closed otherwise (behavior 7,
  acceptance obligation 4). Evidence:
  ep023_unit_two_way_audio_never_without_certification.
- 2026-08-17 | Frigate adapter DTOs bound to real provider shapes |
  DTO field names and semantics copied from the Frigate source
  (frigate/api/defs/response/event_response.py, config/camera/*.py,
  go2rtc streams.go Producer.MarshalJSON) and proven by unit parsing
  of real-shaped JSON. No invented endpoints. Alternatives: guessed
  API (rejected: anti-hallucination law). Evidence:
  ep023_unit_frigate_config_camera_defaults,
  ep023_unit_frigate_event_score_reads_data_score.
- 2026-08-17 | Event confidence from data.score only | Frigate
  EventResponse has no top-level score; the real score lives in
  data.score. Events without a valid score are rejected External
  (malformed provider response) rather than fabricating confidence
  (directive J). Evidence:
  ep023_unit_frigate_events_reject_missing_score_no_fabrication.
- 2026-08-17 | Availability truth table, never collapse | configured
  != reachable != streaming; disabled camera is DEGRADED, provider
  down is UNAVAILABLE, producer attached is STREAMING metadata only
  (directive I/Q). Evidence:
  ep023_unit_frigate_availability_disabled_never_online,
  ep023_unit_frigate_availability_provider_down_is_unavailable.
- 2026-08-17 | RefCell interior mutability for &self ports | The
  nexus-vision ports take &self; the real transport is stateful. The
  adapter uses RefCell (documented single-threaded) so port methods
  drive the real transport; no test-mode branches, no _mut
  duplicates. Evidence: adapter.rs with_transport.
- 2026-08-17 | STREAMING requires live-producer evidence | go2rtc keeps
  a bare {"url":...} producer entry for a DEAD source; only real
  Connection evidence (format_name/protocol/remote_addr/bytes_recv)
  permits STREAMING. A declared-but-dead stream is DEGRADED, never
  AVAILABLE or STREAMING (directive I/Q). Evidence: M3 phase B
  availability_source_dead_never_streaming (observed go2rtc lose live
  evidence at +24s) + unit
  ep023_unit_frigate_availability_dead_producer_never_streaming.
- 2026-08-17 | LiveStream capability requires an actual live path |
  live.streams non-empty OR an ffmpeg input with roles; an empty-roles
  input does not grant LiveStream. Frigate normalizes roles [] ->
  [record,detect] and auto-populates live.streams, so the
  never-connecting nexus_secure truthfully reports Available + declared
  LiveStream but never ObjectDetection/Recording/TwoWayAudio (directive
  H/M). Evidence: M3 integration redaction test + real /api/config.
- 2026-08-17 | M3 media chain certified with controlled fixture only |
  Frigate 0.17.2 (pinned digest) + embedded go2rtc v1.9.10 +
  mediamtx v1.20.0 (pinned sha256) proven REAL via FFmpeg canary
  CONTROLLED_TEST_FIXTURE (media INPUT only; transport/processing real):
  live producer evidence, independent RTSP decode, canary OCR. Physical
  camera NOT_ASSERTED; Roku NOT_ASSERTED/DEFERRED (no hardware). No
  certification upgraded from partial runs.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
