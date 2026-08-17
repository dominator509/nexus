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
- [x] M4: Forced failures, abuse cases, and observability (2026-08-17)
- [x] M5: Live-fire, operations, and node closure (2026-08-17)

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

M4 evidence: `EP-023 M4: ok` (scripts/ep023-m4-tests.sh; observed run
/tmp/ep023-m4-tests.log + evidence /tmp/ep023-m4-evidence.json; node
gate re-run also `EP-023 M4: ok`). Production changes: RestTransport
now carries a bounded per-request timeout (with_timeout; production
wiring sets a small bound) and classifies transport failures into
Timeout (is_timeout) vs Unavailable (connect/other) - the M4 directive
G/H distinction; 401/403 -> Authorization, 404 -> NotFound,
500/502/503 -> Unavailable, other non-success -> External
(classify_status, directive K); per-request provider-boundary
correlation id (frigate-<nanos>-<seq>, unique + safe) is threaded
through get/get_json AND preserved on status/parse error paths
(directive B: the outer provider correlation is never replaced);
`malformed_count()` on the transport trait counts real malformed
responses at the boundary, surfaced in adapter metrics. New
observability module: FrigateObservability (operation counters:
operations/failures/timeouts/auth_failures + bounded redacted audit
ring with correlation ids, poison-safe lock so telemetry can never
alter provider semantics, directive C). Adapter with_transport records
every operation (canonical names: health/version/availability/
stream_ref/list_cameras/capabilities/events_since) with redacted
detail (code only) + correlation; adapter.metrics() merges the
transport malformed counter. New frigate-diag binary
(connectors/frigate/src/bin/frigate-diag.rs, directive N/O): status
(provider_reachable, frigate_version, camera_count, streams_live/
degraded/unavailable, go2rtc_available/stream_count, metrics) and
recover (bounded: fresh observation only, never infrastructure
restart); all output passes redact_url. Failure suite
connectors/frigate/tests/ep023_failure_frigate.rs (18 tests, real
mechanisms only): closed port -> Unavailable; REAL silent TCP peer
(accepts, never responds) + with_timeout(700ms) -> Timeout in <6s
(directive G, the timeout classifier proven against a real socket
read deadline); real HTTP responders: 401/403 -> Authorization, 404 ->
NotFound, 500 -> Unavailable, invalid JSON -> External +
malformed_total >= 1, schema-invalid {"cameras": 42} -> External fail
closed (directive J); redaction canaries (rtsp://user:
EP023_SECRET_CANARY@... and ?token=EP023_TOKEN_CANARY) ZERO occurrence
in VisionError/audit/metrics/diag stdout/stderr (directive E);
correlation present + stable per operation (audit record matches
error correlation, directive P.8); audit ring bounded at capacity with
oldest eviction (directive D); counters increment (timeouts on silent
peer, auth_failures on 401); phase B (REAL Frigate container stopped):
the SAME production operation -> VisionErrorCode::Unavailable, failure
counter increments, audit records the failure, fresh availability
observation never STREAMING (directive F/M, no stale cache);
diagnostic status against healthy provider (phase A) and unavailable
provider (phase B) + diagnostic redaction (directive O/P.12-14);
phase C (container restarted): recovery observed (list_cameras +
nexus_front discovered). Cross-phase accounting 18/18 (directive D).
Frigate restart cold boot observed ~175-179s on this host; the phase C
wait bound is 300s (initial boot bound 240s; first gate run failed at
120s restart bound - genuine timing, not a defect). M2 regression
green (29 ep023_unit + 23 in-crate); M3 regression green (re-run after
RestTransport changes); clippy -D warnings clean; fmt/format/lint/
security/license/dependency/reality gates ok; scope audit EP-023: ok
(scripts/ep023-m4-tests.sh registered in .agent/expected-files/
EP-023.txt); zero-orphan teardown; sysctl hygiene recorded (directive
G: 200000, sufficient, no change).

M5 evidence: `EP-023 M5: ok` (scripts/ep023-m5-tests.sh; observed run
/tmp/ep023-m5-node-gate5.log + LF-008 standalone /tmp/lf008-standalone.log;
node gate re-run also `EP-023 M5: ok`). Pure-contract E2E phase (no
stack): 4 nexus-vision-e2e ep023_e2e tests green - stream refs never
claim verified without evidence (evidence_ref None, verified("") fails
closed, verified("probe-1") only with a real ref), two-way audio fails
closed without certification (state NotCertified, certify() ->
VisionErrorCode::Verification even with other gates), identity
advisory-only (KnownVisitor.advisory_only always true), Roku ladder
fails closed truthfully (host inventory empty, tier UNAVAILABLE,
select_tier picks best available, empty -> UNAVAILABLE); 5
nexus-roku-home ep023_unit_roku tests green (host fail-closed,
ladder order, never fabricates higher tier, canonical vocabulary,
fixture provider port). LF-008 live-fire (scripts/live-fire/LF-008.sh,
real stack): pinned Frigate 0.17.2 digest + mediamtx v1.20.0 sha +
REAL person photograph (infra/frigate/fixtures/person-einstein.jpg,
3250x4333 JPEG) streamed through mediamtx RTSP -> go2rtc -> Frigate cpu
detector with slow horizontal pan (walking pace motion, probe-verified
detection_fps 13.5; static/slow-zoom streams do NOT open Frigate's
motion gate - genuine host behavior, recorded in Surprises); the gate
POLLS /api/events until a real person detection appears (observed at
poll 1, label person, no canned fixture, honest failure otherwise);
E2E journey test ep023_e2e_visitor_response_lf008 (run FOR REAL with
--ignored): maps the REAL person event through the production adapter
into CameraEvent (object person, confidence 0.734, camera nexus_front)
-> VisitorEvent -> advisory UNKNOWN identity -> deterministic
notification-target decision (PRIVATE -> owner only) -> two-way audio
certify() fails closed NOT_CERTIFIED (no verified speaker path, never
fabricated) -> capabilities never advertise TwoWayAudio from metadata
-> stream ref stays Unverified -> real provider availability Streaming;
machine-readable evidence written to
.agent/state/evidence/EP-023-M5-LF-008-visitor-response.json (real
observed values only: real_person_event confidence 0.734, identity
UNKNOWN, two_way NOT_CERTIFIED, roku UNAVAILABLE, stream UNVERIFIED);
zero-orphan teardown verified; LF-008 placeholder (proof-runner.sh
nexusctl/nexus-cli - no such crate) replaced with the real live-fire
script + vacuity guards (EP-001 class). tests/vision/OPS.md ops doc
(health/readiness/backup-restore/upgrade/disable/rollback +
certification boundary). Workspace battery with the live-stack
#[ignore] convention: 1553 passed, 15 ignored (142 suites) - the 14
live-stack integration/failure tests + LF-008 journey are #[ignore]d
for ambient battery and run FOR REAL via --ignored in the M3/M4/M5
gates. Certification registry rows appended (nexus-vision,
nexus-frigate, nexus-roku-home, nexus-vision-e2e INTERNAL_CERTIFIED;
Frigate provider INTERNAL_CERTIFIED pinned digest; Roku hardware
DEFERRED EP-040/EP-043; physical camera NOT ASSERTED). clippy -D
warnings clean; fmt/format/lint/security/license/dependency/reality
gates ok; scope audit EP-023: ok (scripts/ep023-m5-tests.sh +
scripts/live-fire/LF-008.sh registered in
.agent/expected-files/EP-023.txt); M2 regression 52 green; M3
regression ok; M4 regression ok.

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
- 2026-08-17: Frigate 0.17.2 cold boot is SLOW on this host: observed
  ~175s first boot and ~179s after `docker stop`/`docker start`. The
  first M4 gate run failed phase C with a 120s restart bound; the
  container itself was healthy (probe proved restart up at +179s).
  The gate now bounds restart at 300s and dumps container logs on
  failure. This is a genuine host timing fact, not a provider defect.
- 2026-08-17: A "non-routable address" timeout test is flaky across
  platforms (may give refused/unreachable instead of a read timeout);
  the M4 suite instead runs a REAL local TCP peer that accepts the
  connection and stays silent longer than the transport bound. The
  peer is a CONTROLLED_TEST_FIXTURE; the timeout mechanism is the real
  reqwest client read deadline (directive G).
- 2026-08-17: RestTransport `get_json` used to drop the correlation id
  on HTTP-status and JSON-parse error paths (None), which broke audit
  correlation for exactly the failure classes M4 certifies. The
  correlation is now generated once per request and preserved across
  send/status/parse errors; per-request ids include a monotonic
  sequence suffix (time alone could collide).
- 2026-08-17: The pre-created M4 gate in scripts/nodes/EP-023.sh ran
  `cargo test --locked -p nexus-vision ep023_failure` - the M1
  contract crate, EP-001 gate-masking class (same defect class as the
  M2 gate). Replaced with scripts/ep023-m4-tests.sh running the real
  nexus-frigate ep023_failure_frigate suite (vacuity guarded,
  cross-phase accounting, skip verification).
- 2026-08-17: malformed_total cannot be incremented in the adapter
  observability alone: the adapter sees only the canonical External
  code. The malformed counter lives at the transport boundary where
  JSON/DTO parsing actually fails (trait default 0, RestTransport
  counts real parse failures) and adapter.metrics() merges it.
- 2026-08-17: The pre-created LF-008.sh was a placeholder calling
  scripts/proof-runner.sh LF-008 which invokes `nexusctl`/`nexus-cli`
  - NO such crate exists in the workspace (verified: target/release/
  nexusctl absent, no nexus-cli package). Replaced with a real
  live-fire script (LF-026 pattern) that starts the pinned stack,
  polls for a REAL person detection, runs the journey test with
  --ignored, and writes machine-readable evidence.
- 2026-08-17: Frigate's motion gate does NOT open on a static or
  slow-zoom person image: probes with a still loop (zoompan 0.0005)
  and a slow zoom (0.0004) both yielded detection_fps=0.1 and zero
  person events after 240s; a slow HORIZONTAL PAN across the portrait
  (walking pace, crop x advancing ~90px/s) immediately opened the
  gate (detection_fps=13.5) and produced a real person event at the
  first poll. The LF-008 gate uses the pan; the events poll is the
  honest detector (no canned fixture, failure if no person appears).
- 2026-08-17: cargo libtest prints "running 1 test" (singular) for a
  single-test filter; two LF-008 vacuity regexes (`running [1-9][0-9]*
  tests` and the double-escaped `\\\\.\\\\.\\\\.` journey pattern)
  missed single-test phases. Fixed to `running [1-9][0-9]* test` and
  `\.\.\.` (single-escape, verified against the real log line).
- 2026-08-17: The pre-created M5/verify gate in scripts/nodes/
  EP-023.sh ran `cargo test --locked -p nexus-vision` - the M1
  contract crate, EP-001 gate-masking class (same defect class as M2/
  M4). Replaced with scripts/ep023-m5-tests.sh (vacuity-guarded
  pure-contract E2E + Roku units) + the real LF-008 live-fire.
- 2026-08-17: EP-023 is the first node whose integration tests need a
  live stack; the ambient `verify.sh` battery (`cargo test --workspace
  --tests`) panicked on the 14 live-stack tests without
  FRIGATE_BASE_URL. The 10 integration + 4 failure live-stack tests
  are now `#[ignore]`d for the ambient battery and run FOR REAL with
  `--ignored` inside the M3/M4 gates (and the LF-008 journey in the
  M5 gate). Workspace battery: 1553 passed, 15 ignored.

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
- 2026-08-17 | Bounded RestTransport timeout required in production |
  The default reqwest client has no timeout; a blackholed provider
  would hang callers forever. RestTransport::with_timeout sets the
  per-request bound (production wiring MUST call it; the failure suite
  proves a silent peer fails closed with Timeout in <6s). Evidence:
  ep023_failure_frigate_silent_peer_times_out.
- 2026-08-17 | Timeout vs Unavailable are distinct | is_timeout ->
  Timeout; connect/DNS/other transport failures -> Unavailable. A
  closed port (connection refused) is Unavailable; an accepted-but-
  silent peer is Timeout. Evidence:
  ep023_failure_frigate_closed_port_connection_failure_unavailable +
  ep023_failure_frigate_silent_peer_times_out (directive H).
- 2026-08-17 | Real silent-peer timeout mechanism | The timeout proof
  uses a REAL local TCP peer that accepts and never responds
  (CONTROLLED_TEST_FIXTURE); the timeout itself is the real reqwest
  read deadline. No non-routable-address flakiness. Evidence: silent
  peer test elapsed <6s at 700ms bound.
- 2026-08-17 | HTTP authorization classification | 401/403 ->
  Authorization on every provider path (classify_status); no fallback
  to unauthenticated success; auth_failures counter increments.
  Evidence: ep023_failure_frigate_http_401_authorization +
  counters test.
- 2026-08-17 | Provider operation correlation | One provider-boundary
  correlation id per request (frigate-<nanos>-<seq>), preserved across
  status/parse error paths and recorded in audit + VisionError. The
  nexus-vision provider ports do NOT carry caller correlation context
  (contract unchanged); when one arrives in input/error context later,
  with_transport preserves it exactly (directive B). Evidence:
  ep023_failure_frigate_correlation_present_and_stable.
- 2026-08-17 | Process-local observability semantics | Counters are
  monotonic within process lifetime; the audit ring is PROCESS_LOCAL
  and BOUNDED (capacity, oldest eviction). No durability across
  restart is implied; EP-045 owns metrics shipping. Evidence:
  ep023_failure_frigate_audit_ring_bounded.
- 2026-08-17 | Redaction tested against actual secret values | The M4
  suite uses recognizable canaries (EP023_SECRET_CANARY in userinfo,
  EP023_TOKEN_CANARY as token) and requires ZERO occurrence across
  VisionError/audit/metrics/diag stdout/stderr - not a
  "password replaced" substring check. Evidence:
  ep023_failure_frigate_redaction_canaries_absent +
  ep023_failure_frigate_diag_redaction.
- 2026-08-17 | Real provider death behavior | The SAME production
  operation that succeeds in phase A returns
  VisionErrorCode::Unavailable after the real Frigate container is
  stopped; no stale success, no STREAMING from cache, failure counter
  increments, audit records the failure; restart proves recovery.
  Evidence: provider_stopped_unavailable + never_streaming_without_
  fresh_evidence + recovery_after_provider_restart (directive F/M).
- 2026-08-17 | Stream liveness requires current provider evidence |
  The adapter keeps no stale state cache; every availability call
  re-probes the real provider. After provider death a fresh
  observation is Unavailable/Degraded, never STREAMING (directive M).
- 2026-08-17 | frigate-diag status/recovery ownership | status reports
  provider reachability, version, camera counts, live/degraded/
  unavailable streams, go2rtc availability, metrics - all via the real
  production adapter paths, all redacted. recover performs a bounded
  fresh observation only; it NEVER restarts host infrastructure or
  fabricates stream health (directive N/O). Evidence:
  ep023_failure_frigate_diag_status_healthy + _unavailable +
  _redaction.
- 2026-08-17 | M4 gate masking/vacuity defect fixed | The scaffold M4
  line ran the M1/nexus-vision suite (EP-001 class). Replaced with a
  vacuity-guarded ep023-frigate failure suite gate with cross-phase
  accounting (18/18) and per-phase skip verification.
- 2026-08-17 | Live-stack tests #[ignore]d for ambient battery | EP-023
  is the first node whose integration/failure tests need a live stack;
  the ambient verify battery panicked without FRIGATE_BASE_URL. The 14
  live-stack tests (10 integration + 4 failure) and the LF-008 journey
  carry #[ignore] with an explicit reason; the M3/M4/M5 gates run them
  FOR REAL with --ignored against the live pinned stack. The gate
  evidence is unchanged (same tests, same assertions); only the ambient
  battery skips them. Evidence: workspace battery 1553 passed / 15
  ignored, M3/M4/M5 gates green.
- 2026-08-17 | LF-008 real person event, not a canned fixture | The
  gate streams the REAL person photograph (person-einstein.jpg) with
  slow-pan motion and POLLS /api/events until Frigate's cpu detector
  emits a genuine person event (observed at poll 1, confidence 0.734);
  failure to detect within 240s fails the gate. The journey test maps
  that real event through the production adapter; evidence JSON
  records real observed values only. Alternatives: injecting a fake
  event (rejected: fabrication).
- 2026-08-17 | Roku stays a fail-closed ladder, hardware DEFERRED |
  connectors/roku-home binds RokuHomeProviderHost honestly (empty
  inventory, tier UNAVAILABLE, never fabricates a capability or a
  higher ladder tier); HARDWARE_CERTIFICATION DEFERRED to EP-040/EP-043
  with no physical device. The crate exists so the provider port is
  bound to a real implementation instead of an unbound default.
  Evidence: ep023_unit_roku_* tests + EP-023-M5 evidence JSON
  roku_tier UNAVAILABLE.
- 2026-08-17 | TwoWayAudio stays NOT certified on this node | LF-008
  proves certify() fails closed (no verified speaker path) and
  capabilities never advertise TwoWayAudio from config metadata; the
  approved-response playback leg is proven as NOT certified, never
  fabricated. Certification owner: EP-043 with real media hardware.
  Evidence: ep023_e2e_two_way_audio_fails_closed_without_certification
  + EP-023-M5 evidence JSON two_way NOT_CERTIFIED.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.

## Changed files vs machine fence (M5)

- `tests/vision/` (fence: tests/vision/): Cargo.toml + tests/
  ep023_e2e_visitor_response.rs (nexus-vision-e2e, 5 tests: 4 pure
  contract + 1 #[ignore]d LF-008 journey) + OPS.md ops doc.
- `connectors/roku-home/` (fence: connectors/roku-home/): Cargo.toml +
  src/lib.rs (nexus-roku-home, real fail-closed RokuHomeProviderHost,
  5 ep023_unit_roku tests).
- `Cargo.toml` + `Cargo.lock`: +2 workspace members (nexus-roku-home,
  nexus-vision-e2e).
- `connectors/frigate/tests/ep023_integration_frigate.rs`: 10 tests
  #[ignore]d (live stack; run via M3 gate --ignored).
- `connectors/frigate/tests/ep023_failure_frigate.rs`: 4 live-stack
  tests #[ignore]d (run via M4 gate --ignored).
- `scripts/ep023-m3-tests.sh` + `scripts/ep023-m4-tests.sh`:
  run_cargo now passes --ignored to the live-stack suites.
- `scripts/ep023-m5-tests.sh` (new, registered in expected-files):
  M5 gate - vacuity-guarded pure-contract E2E + Roku units + LF-008.
- `scripts/live-fire/LF-008.sh` (registered): real live-fire (was a
  broken proof-runner placeholder invoking nonexistent nexusctl).
- `scripts/nodes/EP-023.sh`: M5/verify line de-masked (was
  `cargo test -p nexus-vision` EP-001 class) -> ep023-m5-tests.sh.
- `.agent/expected-files/EP-023.txt`: + scripts/ep023-m5-tests.sh +
  scripts/live-fire/LF-008.sh.
- `.agent/state/evidence/EP-023-M5-LF-008-visitor-response.json` (new,
  machine-readable real observed values).
- `.agent/state/evidence/CERTIFICATION_REGISTRY.md`: + EP-023 rows.

## Commands and observed sentinels (M5)

- `sh scripts/nodes/EP-023.sh M5` -> `EP-023 M5: ok` (GATE_EXIT=0;
  phase 1: 4 E2E + 5 Roku pure tests green; phase 2: LF-008 live-fire
  green with real person event at poll 1).
- `sh scripts/live-fire/LF-008.sh` -> `LF-008: ok` (standalone run;
  real person detection observed; journey test ok; zero-orphan
  teardown).
- `sh scripts/node-verify.sh EP-023` -> `node verify EP-023: ok`
  (verify battery + M5 gate).
- `sh scripts/scope-audit.sh EP-023` -> `scope audit EP-023: ok`.
- `sh scripts/expected-files.sh EP-023` -> `expected files EP-023: ok`.
- `cargo test --workspace --tests --locked` -> 1553 passed, 15 ignored
  (142 suites).
- `cargo test --locked -p nexus-roku-home -p nexus-vision-e2e` ->
  9 passed, 1 ignored.
- `cargo clippy --locked -p nexus-roku-home -p nexus-vision-e2e
  --all-targets -- -D warnings` -> clean.
- `sh scripts/reality-gate.sh` -> `reality gate: ok`.

## Provider and hardware status

- Frigate 0.17.2 (pinned digest) provider: INTERNAL_CERTIFIED (real
  container, real media chain, real person detection, real events).
- mediamtx v1.20.0 (pinned sha): CONTROLLED_TEST_FIXTURE transport.
- Physical camera hardware: NOT ASSERTED (no hardware exercised).
- Roku hardware: NOT ASSERTED / HARDWARE_CERTIFICATION DEFERRED to
  EP-040/EP-043 (no physical device).
- Two-way audio live certification: NOT ASSERTED (requires real
  speaker/media path; LF-008 proves the honest fail-closed leg).
- WebRTC/RTSP media-level certification: NOT ASSERTED; stream refs
  stay Unverified.

## Remaining risks

- Frigate cold boot ~175-179s on this host: gates use 240s/300s bounds
  (genuine host timing, not a defect).
- Live-stack proofs depend on the host Docker + mediamtx binary; they
  are gated (#[ignore]) in the ambient battery and run FOR REAL in the
  M3/M4/M5 gates.
- Roku and physical-camera certification are explicitly deferred
  debts with owners (EP-040/EP-043); not incomplete simulated code.

## Green tag

- `green/EP-023` created at the M5 implementation commit (the node
  closure commit follows as the ledger closure commit, EP-022
  convention preserved).

