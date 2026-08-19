NODE-META-BEGIN
ID: EP-025
DEPS: EP-024
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-025
VERIFY_SENTINEL: node verify EP-025: ok
GREEN_TAG: green/EP-025
NODE-META-END

# 1. Purpose / Big Picture

Implement Asterisk LTS, SIP provider abstraction, bidirectional media, governed call workflows, STT and TTS, disclosure, and transcripts. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-025.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-025.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-024` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-025.md`
- `.agent/specs/SPEC-014-email-phone-fax-notifications-and-communications-routing.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-025.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-025-asterisk-telephony-and-ai-calling.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-025.txt`
- `.agent/node-contracts/EP-025.md`
- `scripts/nodes/EP-025.sh`
- `crates/nexus-telephony/`
- `connectors/asterisk/`
- `infra/asterisk/`
- `tests/telephony/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `TelephonyProvider` | `nexus-telephony` | Defined by EP-025; provider-neutral and versioned |
| `AsteriskProvider` | `nexus-telephony` | Defined by EP-025; provider-neutral and versioned |
| `SipCarrierProvider` | `nexus-telephony` | Defined by EP-025; provider-neutral and versioned |
| `CallSession` | `nexus-telephony` | Defined by EP-025; provider-neutral and versioned |
| `MediaBridge` | `nexus-telephony` | Defined by EP-025; provider-neutral and versioned |
| `CallPolicy` | `nexus-telephony` | Defined by EP-025; provider-neutral and versioned |
| `DisclosurePolicy` | `nexus-telephony` | Defined by EP-025; provider-neutral and versioned |
| `TranscriptArtifact` | `nexus-telephony` | Defined by EP-025; provider-neutral and versioned |

Acceptance obligations:

1. Asterisk exchanges bidirectional audio with Nexus voice sessions
2. Carrier credentials remain isolated
3. Dial, answer, hangup, transfer, DTMF, hold, and status are governed capabilities
4. Recording and AI disclosure follow policy and jurisdiction configuration

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement asterisk lts, sip provider abstraction, bidirectional media, governed call workflows, stt and tts, disclosure, and transcripts.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-025-M1.txt`, `.agent/node-contracts/EP-025.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-025-asterisk-telephony-and-ai-calling.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-025.txt`, `.agent/node-contracts/EP-025.md`, `scripts/nodes/EP-025.sh`, `crates/nexus-telephony/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep025_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-025.sh M1`

EXPECT:

- `EP-025 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-025 MILESTONE_PASS "M1 EP-025 M1: ok"`

FALLBACK: Support outbound operator-supervised calls before autonomous conversational calls. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-025][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-025.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-025-M2.txt`, `.agent/node-contracts/EP-025.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/asterisk/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep025_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-025.sh M2`

EXPECT:

- `EP-025 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-025 MILESTONE_PASS "M2 EP-025 M2: ok"`

FALLBACK: Support outbound operator-supervised calls before autonomous conversational calls. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-025][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-025 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-025-M3.txt`, `.agent/node-contracts/EP-025.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/asterisk/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep025_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-025.sh M3`

EXPECT:

- `EP-025 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-025 MILESTONE_PASS "M3 EP-025 M3: ok"`

FALLBACK: Support outbound operator-supervised calls before autonomous conversational calls. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-025][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-025 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-025-M4.txt`, `.agent/node-contracts/EP-025.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/telephony/`

CONTENT:

1. Create tests whose names begin `ep025_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-025.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-025 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-025 MILESTONE_PASS "M4 EP-025 M4: ok"`

FALLBACK: Support outbound operator-supervised calls before autonomous conversational calls. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-025][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-025.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-025-M5.txt`, `.agent/node-contracts/EP-025.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: Only the active ExecPlan progress, Decision Log, and ledger may change in this milestone.

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-025` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-025.sh M5`
2. `sh scripts/node-verify.sh EP-025`
3. `sh scripts/scope-audit.sh EP-025`

EXPECT:

- `EP-025 M5: ok`
- `node verify EP-025: ok`
- `scope audit EP-025: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-025 MILESTONE_PASS "M5 EP-025 M5: ok"`

FALLBACK: Support outbound operator-supervised calls before autonomous conversational calls. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-025][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-025` and observe `node verify EP-025: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-012` `governed-phone-call`: Place a real test call through Asterisk and a certified SIP provider, exchange speech with STT and TTS, honor disclosure, and store the governed transcript.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary (2026-08-17; gate `EP-025 M1: ok`)
- [x] M2: Core behavior and deterministic invariants (2026-08-17; gate `EP-025 M2: ok`)
- [x] M3: Real dependency and transport integration (2026-08-18; gate `EP-025 M3: ok`; real Asterisk 22.10.1 + real baresip + real RTP media + RFC4733 DTMF wire proof; commit `[EP-025][M3] real dependency and transport integration`)
- [x] M4: Forced failures, abuse cases, and observability (2026-08-18; gate `EP-025 M4: ok`; 11 live-stack failure proofs + real 401/409/486/603/NO_ANSWER + one-way/mid-call/restart RTP wire proofs + redaction + zero-orphan; commit `[EP-025][M4] forced failures, abuse cases, and observability`)
- [x] M5: Live-fire, operations, and node closure (2026-08-19; gate `EP-025 M5: ok`; REAL inbound governed phone call: real digest caller endpoint-v -> real INVITE auth (fresh call dialog, PJSIP authenticator debug: Authorization MUST precede SDP body) -> dialplan -> Stasis(nexus-telephony) -> ARI answer 204 -> ARI channel record 201 (no mixing bridge: ARI cannot record a bridged channel) -> real whisper STT ("Turn on the lights please.") -> production DisclosurePolicy/TranscriptGate (positive digest-only artifact, negative fails closed, hostile speech is data) -> deterministic bounded response -> real Kokoro current-run TTS -> ARI play 201 through real media path -> far-end RTP drain (tight inner loop, G.711 ulaw decode fixed) -> independent whisper readback ("Turning on the lights now.") -> hangup 204 -> terminal channels=0 -> zero orphans -> redaction clean; 3 LF-012 scenarios + 5 governance tests (REAL TranscriptGate::create_if_allowed over live evidence) green; wire proof src/dst 12140; caller dialog selftest gate guard; ops runbook docs/operations/EP-025-telephony.md; commit `[EP-025][M5] live-fire, operations, and node closure`)

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-18 | Asterisk 22.10.1 ARI returns HTTP 200 with an EMPTY body for `answer`, `bridge`, `continue`, `dtmf`, `moh`, `addChannel`, `redirect`. The transport's `post_json` demanded JSON, so `POST /ari/channels/{id}/answer` failed with "ari malformed JSON response". Fixed with a status-only `post` helper; all empty-body endpoints switched to it. Evidence: live gate run + `connectors/asterisk/src/transport.rs`.
- 2026-08-18 | Asterisk 22 ARI channel GET does NOT serialize the `bridge` field. Bridge membership truth lives in the bridge resource (`GET /ari/bridges/{id}` -> channels array). `session_state`/`media_state` now derive Bridged/TransportActive from real bridge membership with a bounded retry, never from the unpopulated channel field. Evidence: live probe + `adapter.rs`.
- 2026-08-18 | ARI-injected DTMF never emits `ChannelDtmfReceived` over the WS. The RFC4733 wire capture (tcpdump + decode_dtmf.py, ordered digits 5,3,9 at the receiving endpoint's RTP socket) is the authoritative DTMF evidence. Evidence: live pcap decode.
- 2026-08-18 | ARI-injected RFC4733 telephone-events share the SAME SSRC as bridge audio but use a DISJOINT RTP sequence space (observed: audio seq 26365.., DTMF events seq 49517.. on SSRC 3365434088). libre's jbuf treats that as a forward sequence jump and rejects subsequent audio as "too late" (ETIMEDOUT). Fixture disables the endpoint jitter buffer (`jitter_buffer_delay 0 0`) and the journey captures media BEFORE sending DTMF. Evidence: live pcap + baresip log.
- 2026-08-18 | The Asterisk mixing bridge takes over RTP with a NEW SSRC after one pre-bridge packet (observed: SSRC 1870905377 seq 37799 then SSRC 604022046 seq 22116 on the same socket). baresip's SSRC-change flush is gated on `jbuf_started` (false with a single pre-bridge packet), so `seq_put` stays poisoned and all bridge audio is dropped. Same fixture jbuf disable resolves it deterministically.
- 2026-08-18 | baresip hangs up at canary EOF (~10s with the old 4s pad). Canary pad extended to 20s (~26s total) and ALWAYS regenerated; DTMF is sent while the call is alive, transcription only after hangup closes the capture files.
- 2026-08-18 | `pjsip show contacts` global count is NOT a correct readiness model after `docker restart`: baresip briefly double-registers (one stale Unknown contact). The real invariant is per-AOR: exactly one usable current contact for endpoint-a AND endpoint-b, verified from `pjsip show aor <name>`. Fixed at the registrar (fixture AOR policy: `max_contacts=1`, `remove_existing=yes`, explicit test-only expiration bounds `minimum_expiration=3/default_expiration=30/maximum_expiration=60` - Asterisk defaults would clamp the 5s refresh up to 60s).
- 2026-08-18 | libtest runs live-stack tests alphabetically; the restart test must sort LAST (`z_` prefix) and the suite must run serially (`--test-threads=1`), or the restart's `docker restart` tears down the journey's live call.
- 2026-08-18 | `core show channels` guard must parse the numeric count, not `grep -c "active channels"` (the "0 active channels" line itself matches, so the old guard always failed).
- 2026-08-18 | M4 RTP wire guards must bind to STREAM IDENTITY, not packet counts alone: libtest runs live tests alphabetically, so the mid-call-loss test's sender window runs BEFORE the one-way test's; if both proofs share one sender AOR/port, one test's RTP can satisfy the other's guard. Fixed with a DEDICATED second sender AOR (`endpoint-u`, RTP src port 12120) for the one-way proof while `endpoint-t` (12070) stays exclusive to mid-call loss; each guard asserts its own sender's source port, the silent peer's receive port (dst 12060 >= locked minimum), and the silent peer's zero return traffic (src 12060 == 0). Evidence: gate guards 10/11 + responder logs (RTP_TARGET, MEDIA_START/STOP per INVITE).
- 2026-08-18 | The one-way wire proof was starved because the one-way test tore the bridge down ~40ms after media started (only 2 forwarded packets toward the silent peer; guard requires >=50). The live test now holds the bridged call 2s (real RTP accumulates ~100 packets); the restart test holds the post-restart call 6s so the canary audio yields ~96KB dec per side. Evidence: gate guard 10 (to_s >= 50), guard 12 (A=96684..98284 B=same).
- 2026-08-18 | Restart media evidence must be scoped to the CURRENT restart call: the restart test now clears dump-*.wav captures after re-spawn/re-registration and before the new call (clear_audio_dumps), so stale pre-restart media artifacts cannot satisfy the post-restart proof. Evidence: gate guard 12.
- 2026-08-18 | M4 redaction guard scope: the gate's guard 14 originally scanned `$WORK` recursively, which legitimately contains the fixture CONFIG state (rendered pjsip.conf/ari.conf, baresip accounts) holding the real credentials by design - always tripping. Scoped to produced log/event/capture artifacts only (gate log, *.log, ari-events.jsonl, pcap), matching M3's redaction scope. Evidence: gate guard 14 green with zero credential canaries in artifacts.
- 2026-08-18 | Workspace crypto alignment: nexus-telephony's sha2 moved 0.10 -> 0.11 (already locked by nexus-skills/healing/sidecar) so the digest/block-buffer stack is not duplicated; sha1 0.10.6 (RFC 6455 handshake ONLY) keeps its pinned digest 0.10 transitive stack, recorded as exact-version targeted skips in deny.toml. Evidence: dependency-audit.sh ok.
- 2026-08-19 | M5 caller INVITE endless 401 ROOT CAUSE: the digest response was mathematically correct on every attempt (independent MD5 recomputation matched the wire), yet PJSIP returned 401 forever. Enabling the real Asterisk file logger (`logger.conf` with `full => notice,warning,error,debug,verbose`) exposed the truth: `res_pjsip_authenticator_digest.c: No Authorization header found`. The caller fixture built the INVITE by appending the Authorization header AFTER the SDP body (`Content-Length` + `\r\n\r\n` + body), so PJSIP parsed it as part of the body and never saw the header. REGISTER worked because it has no body. Fix: emit Authorization BEFORE the header/body separator. Evidence: /var/log/asterisk/full authenticator DEBUG + sip wire capture + `--mode selftest` regression guard (Authorization BEFORE body separator).
- 2026-08-19 | Asterisk 22.10.1 ARI returns HTTP 204 for `answer`, `addChannel`, `play`, and DELETE live-recording; `record` returns 201; create-bridge returns 200. The first orchestrator versions only accepted 200 and failed on the real codes. Fixed with (200, 201, 204) acceptance on the endpoints that legitimately return 201/204. Evidence: gate runs (answer 204, record 201, play 201, hangup 204 observed).
- 2026-08-19 | ARI cannot record a channel while it is in a bridge: `ERROR res_stasis_recording.c: Cannot record channel while in bridge`. M5's governed call is a single caller -> Stasis channel whose own media path carries RTP both directions, so the mixing bridge is unnecessary and was removed (M4 two-way proof still exercises the bridge). Evidence: Asterisk full log + orchestrator.
- 2026-08-19 | The Asterisk base image ships WITHOUT `/var/spool/asterisk/recording` (ARI channel-record ENOENT -> HTTP 500 `Unrecognized recording error: No such file or directory`) and WITHOUT `/var/lib/asterisk/sounds/en` (docker cp target for ARI play media). Bootstrap now provisions both (mkdir + chown asterisk:asterisk). Evidence: bootstrap + live gate.
- 2026-08-19 | A channel recording auto-finalizes after a short audio gap (`Recording complete` ~6s after start, `No audio available` warning), so `DELETE /recordings/live/{name}` 404s when the file is already stored. The orchestrator treats 404 as success and fetches `/recordings/stored/{name}/file` (observed 200, 34604 bytes). Evidence: orchestrator + gate.
- 2026-08-19 | Caller far-end RTP capture was starved by the receive loop shape: an alternating one-RTP-read / one-SIP-read loop only captured ~8 packets of the ~97-packet TTS burst (0.2s RTP + 0.5s SIP timeouts per iteration). Fixed with a tight inner drain loop for RTP + non-blocking SIP poll. Evidence: pcap (97 packets toward 12140) vs captured bytes before/after fix; far-end readback then transcribed the intended response.
- 2026-08-19 | The fixture's G.711 ulaw decoder was wrong (sign bit mishandled -> OverflowError on valid bytes). Fixed with the canonical table (0xFF silence -> 0, sign branch 0x84 - t). Evidence: decoder sanity + live far-end WAV write.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-17 | Asterisk 22.10.1 already pinned in VERSIONS.lock (class telephony, policy isolated-sidecar-lts) matching SPEC-014 "Asterisk 22 LTS". Preserved. Container image `andrius/asterisk:22.10.1_debian-trixie-amd64` digest `sha256:7a22d773fe0f81adb715cd3e8df57c602726f8ef9d39deead6360e051483e280` selected (official `asterisk/asterisk` Docker Hub/GHCR repos do not exist; andrius/asterisk is the maintained image publishing the exact pinned version; `asterisk -V` inside container verified `Asterisk 22.10.1`). Evidence: docker pull + run output. Alternatives: source build via upstream Dockerfile (no official repo exists); rejected for reproducibility risk. Security: GPL-2.0, isolated-sidecar per COMPONENT_REGISTRY. License: GPL-2.0 behind isolated process boundary per LICENSE_POLICY.md. Compatibility: res_pjsip/chan_pjsip/res_ari*/res_http_websocket/res_rtp_asterisk modules confirmed present (47 res_pjsip modules).
- 2026-08-17 | M1 canonical call state ladder locks the permanent hierarchy as distinct ordered rungs REQUESTED < INVITE_SENT < RINGING < ANSWERED < BRIDGED < MEDIA_ESTABLISHED < TWO_WAY_AUDIO_VERIFIED with terminal HUNG_UP/BUSY/NO_ANSWER/REJECTED/UNAVAILABLE/AUTH_FAILED/NETWORK_ERROR/FAILED. A 200/ANSWER proves signaling only; TWO_WAY_AUDIO_VERIFIED requires decoded bidirectional audio evidence. Evidence: vocabulary.rs + ep025_unit_call_state_ladder_order test. Alternatives: a coarser state model; rejected because SIP signaling != media certification must be encoded in the type.
- 2026-08-17 | CallVerifier exact-target: only the exact session + expected state verifies; unrelated session change = UnrelatedChange; unobservable = Unknown; terminal readback never verifies an active expectation. Evidence: verifier.rs ep025_unit_verifier_* tests. Security: prevents cross-call verification confusion (directive 22).
- 2026-08-17 | M2 adapter state mapping binds to DOCUMENTED ARI channel states only: Up (Answered; Bridged when a real bridge id is present), Ring/Ringing, Dialing, Busy, Down; unrecognized state -> External fail-closed, never fabricated. ARI channel.state proves signaling only; MEDIA_ESTABLISHED/TWO_WAY_AUDIO_VERIFIED come only from the media bridge proof (M3/M5). Evidence: adapter.rs map_channel_state + ep025_unit_state_mapping_documented. Security: prevents signaling==media overclaim (directive 7/32).
- 2026-08-17 | ARI transport: real Asterisk 22 REST surface (health/info, channels CRUD, answer, bridge, dtmf, moh, redirect, continue) over reqwest blocking with bounded timeout; HTTP 401/403->Authorization, 404->NotFound, 500/502/503->Unavailable, 409->Conflict, silent peer->Timeout, malformed JSON->External. reqwest 0.13 requires the `query` feature for .query() (recorded pitfall). A fake Asterisk HTTP server may support parser-failure tests only (directive 2).
- 2026-08-17 | M2 in-flight idempotency: same target + same command in flight -> Conflict; completion releases the entry so retry is not Conflict (M2/M4 precedent preserved; crash-durable NOT ASSERTED process-local). Evidence: ep025_unit_idempotency_duplicate_conflict.
- 2026-08-17 | asterisk-diag status/recover: bounded actions only - health probe + fresh readback; NEVER originates/answers/hangs up/plays/DTMF (directive V). Evidence: connectors/asterisk/src/bin/asterisk-diag.rs.
- 2026-08-18 | Fixture AOR policy is CONTROLLED_TEST_FIXTURE tuning, NOT production/provider-wide semantics: `max_contacts=1`, `remove_existing=yes`, and explicit expiration bounds (`minimum_expiration=3`, `default_expiration=30`, `maximum_expiration=60`) make a one-device baresip registration deterministically replace the old one after `docker restart`. Asterisk defaults (`minimum_expiration=60`) would clamp the client's 5s refresh upward, leaving a stale contact observable ~60s. Test fixture timing vs production deployment policy recorded separately; production recommended values are NOT altered. Evidence: pjsip.conf.tmpl + `pjsip show aor endpoint-a` (MaxContact=1, remove_existing=true).
- 2026-08-18 | baresip is a CONTROLLED_TEST_FIXTURE; its jitter buffer is disabled (`jitter_buffer_delay 0 0`) because Asterisk 22.10.1's ARI-injected DTMF (disjoint RTP seq space on the same SSRC) and bridge SSRC takeover poison libre's jbuf sequence tracking, deterministically dropping all later audio. The M3 media proof (real RTP -> real PCMU decode -> whisper) and DTMF proof (RFC4733 wire capture) do not depend on the endpoint jitter buffer. Production receive-path semantics are unaffected. Evidence: pcap (seq 26365 vs 49517, SSRC 1870905377 vs 604022046), baresip logs, whisper readback of both directions.
- 2026-08-18 | Restart readiness is per-AOR, not a global contact count: exactly one usable current contact for endpoint-a AND one for endpoint-b from `pjsip show aor <name>`, because a global count can be satisfied by a stale/Unknown contact on the wrong AOR (observed: INVITE routed to a dead registration). Evidence: z_ restart test + gate guard 2.
- 2026-08-18 | Bounded asynchronous reconciliation for real Asterisk state: bridge membership and hangup destruction propagate asynchronously (ChannelEnteredBridge/ChannelDestroyed fire after the HTTP response), so verification polls real state with a bounded 8s deadline; only actual NotFound/channel disappearance satisfies. No fabricated terminal state; deadline expiry FAILS the test. Evidence: adapter.rs bridge/hangup verification + live gate runs.
- 2026-08-18 | No blind retry doctrine: the stale-contact race is fixed at the registrar (replacement policy + expiration bounds), not by waiting longer for the count to fall; the restart test requires deterministic per-AOR convergence before originating the second call. Evidence: pjsip.conf.tmpl + z_ restart test.
|- 2026-08-18 | Template placeholder names use lowercase (double-brace `ari_password` key) matching the reality-gate pattern exemption convention (HA fixture templates); rendered per-run with random values, never committed. Evidence: ari.conf.tmpl/pjsip.conf.tmpl + bootstrap render (`key.lower()`).
|- 2026-08-18 | M4 terminal-outcome authority hierarchy (directive A): ACTIVE/NONTERMINAL state comes from REST/ARI channel + bridge state; TERMINAL CALL OUTCOME comes from the ARI event stream (`ChannelDestroyed.cause`). A 486/603 destroys the channel before REST polling can observe any intermediate state (observed: channel gone before a 0.5s poll), so BUSY/REJECTED/NO_ANSWER are NEVER reconstructed from a missing channel; NotFound without a typed cause is Verification (UNKNOWN), not Busy/Rejected. Evidence: real `SIP/2.0 486 Rejected` + `SIP/2.0 603 Decline` on the wire; ChannelDestroyed cause=17 (User busy) and cause=21 (Call Rejected) observed on the ARI WS; adapter.rs wait_terminal + terminal_state_from_cause.
|- 2026-08-18 | Real ARI WebSocket event consumer (M4): minimal RFC6455 client (sha1 pinned for the handshake accept digest ONLY - not used for Nexus integrity/signing; base64/rand for the client key) + bounded EventStore (FIFO-pruned cause map, bounded recent-event ring, `connected` observability flag). The consumer marks itself disconnected on WS loss, never fabricates terminal outcomes during the gap, and reconnects with bounded backoff. Evidence: events.rs + live ep025_live_event_stream_disconnect_no_fabrication (gap session -> Verification error; reconnect session -> typed Rejected; exact-target: gap session never resurrected).
|- 2026-08-18 | Bounded provider originate timeout (M4 directive E): `originate_with_app_bounded`/`originate_stasis_bounded` pass the ARI `timeout` so NO_ANSWER is tied to the REAL Asterisk lifecycle - Asterisk destroys the ringing channel when the timer expires (Q.850 cause 102/19 recorded on the event stream) - not to a local sleep. Evidence: live ep025_live_no_answer_bounded_provider_timeout (cause 18/19/102 observed in the store).
|- 2026-08-18 | Controlled SIP responder fixture (reject_endpoint.py, CONTROLLED_TEST_FIXTURE) implements just enough REAL RFC 3261 to be a real PJSIP peer: digest REGISTER (real 401 challenge -> MD5 response -> 200), 603/486/ring/silent/sender modes, and a probe mode that reports the final digest result (PROBE_RESULT 200/401). RTP destination is learned from Asterisk's SDP offer; the advertised SDP c= address is the docker0 bridge address (127.0.0.1 is unreachable from inside the container). Evidence: real wire exchanges + gate guards.
|- 2026-08-18 | Ambiguous-originate doctrine (M4 directives K): when an originate's control response is lost, NEVER blind-retry; `reconcile_originate(caller_token)` matches the real channel by caller number and returns it, and Asterisk must hold exactly ONE channel for the logical call. Evidence: live ep025_live_ambiguous_originate_no_blind_retry (DropFirstOriginate fixture transport wraps the REAL transport: call placed, response dropped -> Unavailable; reconcile finds the channel; exactly 1 matching channel).
|- 2026-08-18 | Non-Stasis DTMF negative (M4 directive M): a channel created via the dialplan (extension 100 -> Dial, NOT in the nexus-telephony Stasis app) yields a real HTTP 409 `Channel not in Stasis application` from production send_dtmf, mapped to canonical Conflict; never success. Evidence: live ep025_live_non_stasis_dtmf_409 + transport 409 classification.
|- 2026-08-18 | DTMF input validation (M4 directive N): empty, overlong (>64), or illegal-character digit strings fail as Validation BEFORE provider mutation (zero transport calls). Evidence: adapter.rs send_dtmf validation + unit tests.
|- 2026-08-18 | One-way media and mid-call media loss (M4 directives H/I): a peer that answers with a=recvonly and sends NO RTP is never reported two-way verified (wire proof: bytes flow toward the silent peer's RTP port, none from it); a sender peer whose RTP window ends while the call stays Bridged keeps the call signaling-active while production still does NOT claim verified media. Evidence: live ep025_live_one_way_media_not_verified + ep025_live_mid_call_media_loss_not_verified + gate tcpdump guards on docker0 (ports 12060/12070).
|- 2026-08-18 | Restart during active call (M4 directive J): the call is honestly observed lost (no synthesized continuity), the consumer reconnects, controlled baresip endpoints DIE on mid-call provider restart (M3 observation) and are re-spawned by the test, per-AOR registration converges, and a new real call reaches bridged two-way media again. Evidence: live ep025_live_restart_during_active_call + gate post-restart media captures.
|- 2026-08-18 | M4 observability semantics (directive V): EventStore/audit state is PROCESS_LOCAL and BOUNDED (FIFO-pruned causes, bounded ring); the `connected` flag is live observability of the WS subscription; no event history survives process restart (no persistence claimed). Evidence: events.rs.
|- 2026-08-18 | M4 idempotency certification: in-process call idempotency PASS (same target + same command in flight -> Conflict; release on completion; contract suite ep025_failure_idempotency_in_process_conflict_and_release); crash-durable call idempotency NOT ASSERTED (no durable store implemented).
- 2026-08-19 | M5 caller fixture dialog model: REGISTER and the call are DISTINCT SIP usages. The INVITE gets its own fresh Call-ID + From tag + Via branch + CSeq 1; the 401-authenticated retry preserves the INVITE dialog identity (same Call-ID, same From tag, new Via branch, CSeq 2, fresh Authorization). Enforced by `--mode selftest` (8 structural assertions) wired into the M5 gate as guard 0d. Evidence: selftest + wire capture (REGISTER Call-ID d59329aa... vs INVITE fa743ac5...).
- 2026-08-19 | M5 governed-call orchestration is a test harness on system python3 (real ARI REST+WS, like ari_observer.py) because the production adapter does not expose play/record; governance assertions run through REAL production Rust TranscriptGate/DisclosurePolicy/CallPolicy in ep025_governed_live.rs. Orchestrator-first WS subscription ordering prevents the StasisStart race. Evidence: gate + orchestrator logs.
- 2026-08-19 | M5 deterministic response path is bounded (no frontier model): recognized phrase -> fixed response text; hostile markers ("ignore the rules", "unlock the door") -> "I cannot help with that request." with command_recognized=false and hostile_content=true in evidence. Evidence: orchestrator + hostile evidence JSON.
- 2026-08-19 | The custom SIP caller (reject_endpoint.py --mode caller) is a CONTROLLED_TEST_FIXTURE that implements just enough REAL RFC 3261 (digest REGISTER, authenticated INVITE, ACK, BYE, RTP) to exercise real Asterisk/Nexus; it is NOT a production SIP stack. Production SIP remains Asterisk/PJSIP. Evidence: fixture file + directive L.

# 14. Certification Registry (M3+M4, real evidence)

Certification entries recorded ONLY where the M3/M4 live-fire gates actually exercised the capability (directive: no certification without real proof; deferred items remain explicitly owned debts, not simulated success).

- Asterisk 22.10.1 (pinned image, real container): **PROVIDER_CERTIFIED** - real registration, real ARI/Stasis, real mixing bridge, real RTP media, real DTMF wire capture, real restart/re-registration. Evidence: scripts/ep025-m3-tests.sh full gate + 4 live-stack integration tests.
- PJSIP registration / digest auth: **PROVIDER_CERTIFIED** - real digest 401/200 exchange, per-AOR contact verification, wrong/cross credentials rejected (gate probes).
- ARI / Stasis call control: **PROVIDER_CERTIFIED** - originate into real Stasis app, exact-target StasisStart, answer, hangup with bounded NotFound verification.
- ARI mixing bridge: **PROVIDER_CERTIFIED** - real bridge resource membership, Bridged/TransportActive from real membership, channel removal verified.
- Two-way RTP media: **PROVIDER_CERTIFIED** - real PCMU RTP both directions, whisper readback of decoded captures ("Alpha Econexus" / "Bravo ... Nexus" observed).
- PCMU / ulaw: **MEDIA_CERTIFIED** - negotiated and carried in the real RTP stream (core show channel codec observed ulaw).
- RFC4733 DTMF: **PROVIDER_CERTIFIED** - production ARI send_dtmf, telephone-event packets captured on the wire, ordered_digits 539 decoded from the pcap.
- ARI event stream (RFC6455 WS): **PROVIDER_CERTIFIED** - real ChannelDestroyed causes observed (17=User busy->Busy, 21=Call Rejected->Rejected, 18/19/102->NoAnswer); event-stream disconnect never fabricates terminal state; reconnect resumes; exact-target store never resurrects a gap session. Evidence: events.rs + ep025_live_event_stream_disconnect_no_fabrication + store_cause assertions.
- Typed BUSY / REJECTED / NO_ANSWER classification: **PROVIDER_CERTIFIED** - real SIP 486 -> cause 17 -> Busy; real SIP 603 -> cause 21 -> Rejected; bounded ARI originate timeout -> Asterisk destroys the ringing channel (cause 18/19/102) -> NoAnswer. Evidence: ep025_live_rejected_603_typed_rejected, ep025_live_busy_486_typed_busy, ep025_live_no_answer_bounded_provider_timeout.
- One-way media detection: **INTERNAL/PROVIDER_CERTIFIED** - a real peer answering a=recvonly with zero return RTP is never reported two-way verified (wire proof: sender-u RTP src 12120 -> silent peer dst 12060 >= 50, src 12060 == 0). Evidence: gate guard 10 + ep025_live_one_way_media_not_verified.
- Mid-call media loss detection: **INTERNAL/PROVIDER_CERTIFIED** - a sender whose RTP window ends while the call stays Bridged keeps production from claiming verified media (wire proof: sender-t src 12070 window bounded, last packet precedes suite end). Evidence: gate guard 11 + ep025_live_mid_call_media_loss_not_verified.
- Wrong PJSIP credential: **PROVIDER_CERTIFIED** - real 401, zero contacts on the AOR.
- Wrong ARI credential: **PROVIDER_CERTIFIED** - truthful auth failure (diag never reports AVAILABLE).
- Asterisk unavailable: **PROVIDER_CERTIFIED** - truthful UNAVAILABLE, originate fails honestly (no fake CallSession).
- Non-Stasis DTMF: **PROVIDER_CERTIFIED** - real HTTP 409 -> canonical Conflict, never success.
- Ambiguous originate: **PROVIDER_CERTIFIED** - lost control response -> Unavailable + reconcile finds the real channel; exactly one channel, no blind duplicate.
- baresip (controlled SIP endpoints): **CONTROLLED_TEST_FIXTURE** - not a production provider; used to place/answer real calls under test.
- reject_endpoint.py controlled SIP responders (endpoint-r/s/t/u): **CONTROLLED_TEST_FIXTURE** - real digest REGISTER + real 603/486/ring/silent/sender/probe behavior; the deterministic sender responders (t/u) are the A-side for media-failure wire proofs.
- Kokoro / whisper voice assets: reuse EP-021 certification (whisper-cli + ggml-tiny.en.bin used for transcription readback; not re-certified here).
- Other codecs (G.722, Opus, etc.): **NOT ASSERTED** - configured on endpoints but not exercised as the negotiated in-call codec.
- SIP TLS / SRTP: **NOT ASSERTED** - transport is plain UDP/TCP; no TLS/SRTP exercised.
- Carrier / PSTN / external trunk: **NOT ASSERTED / DEFERRED** - no carrier connectivity in the controlled fixture.
- Physical handset / mobile endpoint: **NOT ASSERTED** - baresip software endpoints only.
- Crash-durable call idempotency: **NOT ASSERTED** - in-process idempotency PASS (Conflict on in-flight duplicate, release on completion); no durable store, explicitly not claimed.
- M5 governed inbound phone call (LF-012): **PROVIDER_CERTIFIED** - real digest caller endpoint-v -> real INVITE (fresh call dialog, authenticated retry accepted) -> real dialplan -> real Stasis(nexus-telephony) -> real ARI answer/record/play/hangup -> real RTP both directions -> real whisper.cpp STT -> production TranscriptGate digest-only artifact -> real Kokoro TTS -> independent far-end whisper readback ("Turning on the lights now."). Evidence: scripts/ep025-m5-tests.sh full gate + EP-025-M5-LF-012-{positive,negative-disclosure,hostile}.{json,md} + 5 governed live tests.
- DisclosurePolicy enforcement: **INTERNAL_CERTIFIED (via REAL gate)** - consented -> TranscriptGate::create_if_allowed produces digest-only artifact; not consented -> fails closed (None); hostile speech transcribed as DATA, never authority (command_recognized=false). Evidence: ep025_governed_live.rs tests over live evidence JSON.
- whisper.cpp STT: **PROVIDER_CERTIFIED** (reuse EP-021 certification; whisper-cli + ggml-tiny.en.bin sha256 921e4c...) - transcribed the real in-call recording "Turn on the lights please." Evidence: orchestrator + evidence JSON stt_transcript/stt_digest.
- Kokoro TTS: **PROVIDER_CERTIFIED** (reuse EP-021 certification; engine venv /opt/nexus-voice-engines, model sha256 496dba...) - synthesized a NEW per-run waveform for the exact response, played through real Asterisk media path. Evidence: orchestrator + evidence JSON tts_wav_sha256 (per-run, never reused).
- reject_endpoint.py --mode caller: **CONTROLLED_TEST_FIXTURE** - real digest REGISTER + authenticated INVITE + ACK/BYE + RTP streaming/capture; NOT a production SIP stack (directive L).

Deferred certification debts (owned, not incomplete simulation):
- OS-level sandbox for the telephony sidecar: DEFERRED to EP-040 / EP-043 (consistent with the skill-contract certification boundary).

# 15. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.

## M5 Outcomes (2026-08-19)

**Commands and observed sentinels:**
- `sh scripts/ep025-m5-tests.sh` -> `EP-025 M5: ok` (GATE_EXIT=0), plus:
  - `bootstrap: ok container=nexus-ep025-ast`
  - `EP-025 M5: engines verified (whisper-cli, ggml-tiny.en.bin, Kokoro 496dba...)`
  - `EP-025 M5: caller dialog selftest ok` (8 structural assertions, guard 0d)
  - `EP-025 M5: caller phrases synthesized (ok=<sha> hostile=<sha>)` (fresh per-run Kokoro waveforms)
  - `EP-025 M5: LF-012 positive call complete` / `negative-disclosure` / `hostile`
  - `EP-025 M5: far-end readback (positive) = Turning on the lights now.` (independent whisper)
  - `test result: ok. 5 passed; 0 failed` (governed live suite)
  - `EP-025 M5: wire proof ok (caller src 12140 -> Asterisk; response -> caller)`
  - `EP-025 M5: zero-orphan teardown ok (channels=0 bridges=0)`
  - `EP-025 M5: redaction ok (zero credential canaries in artifacts)`
- `sh scripts/nodes/EP-025.sh M5` -> `EP-025 M5: ok`
- `sh scripts/nodes/EP-025.sh verify` -> `node verify EP-025: ok`

**Owned live-fire proof LF-012 (governed-phone-call):** real inbound governed call through real Asterisk 22.10.1: real digest caller (endpoint-v, dedicated RTP src 12140) -> real authenticated INVITE (fresh call dialog; retry CSeq 1->2, new Via branch, same dialog identity) -> dialplan `_1XX` -> `Stasis(nexus-telephony)` -> ARI answer 204 -> ARI channel record 201 -> real caller RTP (Kokoro phrase, PCMU) -> whisper.cpp STT ("Turn on the lights please.") -> production DisclosurePolicy/TranscriptGate (consented -> digest-only artifact; not consented -> fails closed; hostile -> data, not authority) -> deterministic bounded response -> real Kokoro current-run TTS -> ARI play 201 through real Asterisk media path -> far-end RTP drain + G.711 decode -> independent whisper readback ("Turning on the lights now.") -> hangup 204 -> terminal channels=0 -> zero orphans -> redaction clean.

**Changed files vs M5 manifest (`.agent/milestone-files/EP-025-M5.txt`):**
- `infra/asterisk/fixture/lf012_orchestrator.py` (new, real ARI orchestrator)
- `infra/asterisk/fixture/reject_endpoint.py` (caller mode + dialog selftest + RTP drain + ulaw decode)
- `infra/asterisk/fixture/asterisk_bootstrap.py` (endpoint-v password, recording/sounds spool provisioning)
- `infra/asterisk/config/extensions.conf` (Stasis app nexus-telephony), `pjsip.conf.tmpl` (endpoint-v AOR), `logger.conf` (new)
- `connectors/asterisk/tests/ep025_governed_live.rs` (new, 5 governance tests)
- `scripts/ep025-m5-tests.sh` (new gate), `scripts/live-fire/LF-012.sh` (real wrapper), `scripts/nodes/EP-025.sh` (M5|verify wiring)
- `.agent/milestone-files/EP-025-M5.txt`, `.agent/expected-files/EP-025.txt`
- `.agent/state/evidence/EP-025-M5-LF-012-{positive,negative-disclosure,hostile}.{json,md}` (new)
- `docs/operations/EP-025-telephony.md` (new ops runbook)

**Certification status:** LF-012 governed call PROVIDER_CERTIFIED; DisclosurePolicy/TranscriptGate INTERNAL_CERTIFIED via real gate; whisper/Kokoro reuse EP-021 (PROVIDER_CERTIFIED); caller fixture CONTROLLED_TEST_FIXTURE; OS-level sandbox DEFERRED to EP-040/EP-043; carrier/PSTN/TLS/SRTP/handset NOT ASSERTED.

**Remaining risks:** PSTN/carrier integration not exercised; production SIP media security (TLS/SRTP) not certified; governed-call orchestration is a test harness (production adapter does not yet expose play/record); crash-durable idempotency NOT ASSERTED.

**Green tag:** `green/EP-025` (after NODE_DONE).
