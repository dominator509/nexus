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
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

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
- 2026-08-18 | Template placeholder names use lowercase (`{{ari_password}}`) matching the reality-gate pattern exemption convention (HA fixture templates); rendered per-run with random values, never committed. Evidence: ari.conf.tmpl/pjsip.conf.tmpl + bootstrap render (`key.lower()`).

# 14. Certification Registry (M3, real evidence)

Certification entries recorded ONLY where the M3 live-fire gate actually exercised the capability (directive: no certification without real proof; deferred items remain explicitly owned debts, not simulated success).

- Asterisk 22.10.1 (pinned image, real container): **PROVIDER_CERTIFIED** - real registration, real ARI/Stasis, real mixing bridge, real RTP media, real DTMF wire capture, real restart/re-registration. Evidence: scripts/ep025-m3-tests.sh full gate + 4 live-stack integration tests.
- PJSIP registration / digest auth: **PROVIDER_CERTIFIED** - real digest 401/200 exchange, per-AOR contact verification, wrong/cross credentials rejected (gate probes).
- ARI / Stasis call control: **PROVIDER_CERTIFIED** - originate into real Stasis app, exact-target StasisStart, answer, hangup with bounded NotFound verification.
- ARI mixing bridge: **PROVIDER_CERTIFIED** - real bridge resource membership, Bridged/TransportActive from real membership, channel removal verified.
- Two-way RTP media: **PROVIDER_CERTIFIED** - real PCMU RTP both directions, whisper readback of decoded captures ("Alpha Econexus" / "Bravo ... Nexus" observed).
- PCMU / ulaw: **MEDIA_CERTIFIED** - negotiated and carried in the real RTP stream (core show channel codec observed ulaw).
- RFC4733 DTMF: **PROVIDER_CERTIFIED** - production ARI send_dtmf, telephone-event packets captured on the wire, ordered_digits 539 decoded from the pcap.
- baresip (controlled SIP endpoints): **CONTROLLED_TEST_FIXTURE** - not a production provider; used to place/answer real calls under test.
- Kokoro / whisper voice assets: reuse EP-021 certification (whisper-cli + ggml-tiny.en.bin used for transcription readback; not re-certified here).
- Other codecs (G.722, Opus, etc.): **NOT ASSERTED** - configured on endpoints but not exercised as the negotiated in-call codec.
- SIP TLS / SRTP: **NOT ASSERTED** - transport is plain UDP/TCP; no TLS/SRTP exercised.
- Carrier / PSTN / external trunk: **NOT ASSERTED / DEFERRED** - no carrier connectivity in the controlled fixture.
- Physical handset / mobile endpoint: **NOT ASSERTED** - baresip software endpoints only.

Deferred certification debts (owned, not incomplete simulation):
- OS-level sandbox for the telephony sidecar: DEFERRED to EP-040 / EP-043 (consistent with the skill-contract certification boundary).

# 15. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
