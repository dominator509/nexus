NODE-META-BEGIN
ID: EP-021
DEPS: EP-020
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-021
VERIFY_SENTINEL: node verify EP-021: ok
GREEN_TAG: green/EP-021
NODE-META-END

# 1. Purpose / Big Picture

Implement audio ingest, VAD, custom wake word, local STT, local TTS, speaker evidence, cloud fallbacks, and privacy controls. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-021.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-021.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-020` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-021.md`
- `.agent/specs/SPEC-012-voice-speech-wake-word-speaker-evidence-satellites-bluetooth-and-audio-routing.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-021.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-021-voice-core-stt-tts-wake-and-speaker-id.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-021.txt`
- `.agent/node-contracts/EP-021.md`
- `scripts/nodes/EP-021.sh`
- `python/nexus_voice/`
- `models/wake/`
- `infra/voice/`
- `tests/voice/core/`
- `benchmarks/voice/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `AudioFrame` | `tests/voice/core` | Defined by EP-021; provider-neutral and versioned |
| `VadProvider` | `tests/voice/core` | Defined by EP-021; provider-neutral and versioned |
| `WakeWordProvider` | `tests/voice/core` | Defined by EP-021; provider-neutral and versioned |
| `SttProvider` | `tests/voice/core` | Defined by EP-021; provider-neutral and versioned |
| `TtsProvider` | `tests/voice/core` | Defined by EP-021; provider-neutral and versioned |
| `SpeakerEvidenceProvider` | `tests/voice/core` | Defined by EP-021; provider-neutral and versioned |
| `VoiceSession` | `tests/voice/core` | Defined by EP-021; provider-neutral and versioned |
| `AudioPrivacyPolicy` | `tests/voice/core` | Defined by EP-021; provider-neutral and versioned |

Acceptance obligations:

1. Silero VAD, custom commercial-safe openWakeWord weights, whisper.cpp, and Kokoro work locally
2. Deepgram and OpenAI STT plus ElevenLabs and Azure TTS fit the same contracts
3. Speaker recognition is evidence only
4. Hardware mute and shared-room privacy states propagate to policy

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement audio ingest, vad, custom wake word, local stt, local tts, speaker evidence, cloud fallbacks, and privacy controls.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-021-M1.txt`, `.agent/node-contracts/EP-021.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-021-voice-core-stt-tts-wake-and-speaker-id.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-021.txt`, `.agent/node-contracts/EP-021.md`, `scripts/nodes/EP-021.sh`, `python/nexus_voice/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep021_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-021.sh M1`

EXPECT:

- `EP-021 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-021 MILESTONE_PASS "M1 EP-021 M1: ok"`

FALLBACK: Use push-to-talk with local STT and TTS when wake-word accuracy or AEC is not certified. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-021][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-021.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-021-M2.txt`, `.agent/node-contracts/EP-021.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `models/wake/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep021_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-021.sh M2`

EXPECT:

- `EP-021 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-021 MILESTONE_PASS "M2 EP-021 M2: ok"`

FALLBACK: Use push-to-talk with local STT and TTS when wake-word accuracy or AEC is not certified. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-021][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-021 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-021-M3.txt`, `.agent/node-contracts/EP-021.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/voice/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep021_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-021.sh M3`

EXPECT:

- `EP-021 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-021 MILESTONE_PASS "M3 EP-021 M3: ok"`

FALLBACK: Use push-to-talk with local STT and TTS when wake-word accuracy or AEC is not certified. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-021][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-021 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-021-M4.txt`, `.agent/node-contracts/EP-021.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/voice/core/`

CONTENT:

1. Create tests whose names begin `ep021_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-021.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-021 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-021 MILESTONE_PASS "M4 EP-021 M4: ok"`

FALLBACK: Use push-to-talk with local STT and TTS when wake-word accuracy or AEC is not certified. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-021][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-021.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-021-M5.txt`, `.agent/node-contracts/EP-021.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `benchmarks/voice/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-021` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-021.sh M5`
2. `sh scripts/node-verify.sh EP-021`
3. `sh scripts/scope-audit.sh EP-021`

EXPECT:

- `EP-021 M5: ok`
- `node verify EP-021: ok`
- `scope audit EP-021: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-021 MILESTONE_PASS "M5 EP-021 M5: ok"`

FALLBACK: Use push-to-talk with local STT and TTS when wake-word accuracy or AEC is not certified. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-021][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-021` and observe `node verify EP-021: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-028` `shared-room-private-response`: Ask for sensitive personal information in an occupied room and prove Nexus routes the response privately rather than speaking it aloud.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [x] M2: Core behavior and deterministic invariants
- [x] M3: Real dependency and transport integration
- [x] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.

## 2026-08-16 -- M2 wake model core (evidence: models/wake/tests; SPEC-012/SPEC-019)

- DECISION: models/wake/ owns the deterministic wake machinery as the
  python package `nexus_wake`: model manifests, license safety, digest
  verification, idempotent registry, and the armed/triggered/disarmed/
  uncertified decision state machine. The real openWakeWord runtime
  inference plugs in behind `WakeModelScore` at M3 (infra/voice); the
  core never fabricates a trigger and never ships noncommercial weights
  (SPEC-019 required behavior 2; SPEC-012 non-goal).
- DECISION: registry `register()` requires real weights digest
  verification before a model becomes usable; identical re-registration
  is idempotent; same id with different digest is `WakeModelConflict`
  (never silent overwrite).
- DECISION: M2 gate rewritten from the pre-created artifact-only node
  script entry to vacuity-guarded `scripts/ep021-m2-tests.sh`
  (EP-001 gate-masking class), same pattern as M1 and LF-023.
- SECURITY: no credentials/weights in repo; digest verification is
  mandatory; raw audio never retained or logged.
- REVERSAL: revert to M1 commit. No public-surface break: nexus_voice
  contracts unchanged; nexus_wake is a new package under models/wake.

## 2026-08-16 -- M3 real engine integration (evidence: infra/voice/, tests/voice/core ep021_integration; SPEC-012/SPEC-019; EP-021 owner directive)

- DECISION: four real engines integrated as an isolated sidecar venv
  (/opt/nexus-voice-engines, Python 3.12.3) with subprocess workers
  (EP-021 directive G): Silero VAD v5.1 ONNX, openwakeword 0.4.0,
  whisper.cpp v1.7.4 (commit 8a9ad784), Kokoro 0.9.4 on torch 2.13.0+cpu
  (official CPU index, directive I). The main project interpreter (3.14)
  stays frozen; adapters under infra/voice/ are stdlib-only and map worker
  JSON onto the nexus_voice contracts. Disk: 41.3 GiB reclaimed via
  owner-authorized `cargo clean` of the regenerable workspace target/
  (directive B); engines/models consume ~2.5 GiB; ~40 GiB remains free.
- DECISION: openwakeword production wake-model certification is DEFERRED
  (recorded in infra/voice/manifests/certification.yaml with the graph
  gap); the bundled noncommercial pretrained weights are never used
  (SPEC-019). A real controlled-test wake model (nexus_wake_hey_nexus_v1)
  was trained in-repo on Kokoro-synthesized fixtures through the real
  openwakeword feature frontend (LR on 96-dim embeddings, ONNX export
  matching the engine contract [1,16,96] -> [1,1]): 18/18 positives at
  1.000, negatives <= 0.078. Weights are Nexus-owned (Apache-2.0);
  retraining is functionally deterministic (identical separation stats),
  byte digest varies with BLAS float scheduling (documented in manifest).
- DECISION: M1 abstract contract ports converted from `raise
  NotImplementedError` to typed fail-closed `VoiceError(UNAVAILABLE)`
  (repo convention per python/nexus_connector_sdk) to satisfy the reality
  gate honestly; contracts otherwise unchanged (M1 suite still green).
- DECISION: wake worker streams int16 1280-sample frames through the real
  engine predict loop (openwakeword buffer zeroes the first 5 frames;
  float32 input collapses to silence); whisper worker resamples to 16 kHz
  (whisper-cli requires 16 kHz input).
- SECURITY: no credentials or weights in the repository; models and
  binaries live outside the repo (/opt) with digests recorded in
  infra/voice/manifests/models.yaml + engines.yaml; raw audio is
  ephemeral and never logged.
- REVERSAL: revert to M2 commit. No public-surface break: nexus_voice
  contracts unchanged (only default port bodies); infra/voice is new.

## 2026-08-16 -- M4 forced failures and observability (evidence: tests/voice/core ep021_failure; SPEC-006; EP-021 directive O)

- DECISION: adapters fail closed with typed SPEC-006 VoiceErrors. A new
  `run_engine` wrapper (infra/voice/adapters/__init__.py) maps real
  engine failures to UNAVAILABLE and real subprocess timeouts to TIMEOUT;
  no raw audio ever reaches an error surface (as_dict redacts payload).
- DECISION: M4 suite exercises only real failure mechanisms (no mocks):
  missing silero/wake/whisper model files, a genuinely corrupt WAV
  rejected by the worker, a permission-denied model (chmod 0), a missing
  sidecar venv, and a real 1s subprocess timeout against a ~3.3s whisper
  transcription. Unsupported compressed frames are refused at the adapter
  boundary (typed validation) before any engine is invoked.
- DECISION: SttProviderWhisperCpp gained model/binary/timeout constructor
  parameters so real failure injection does not require mocks.
- SECURITY: error surfaces verified redacted in tests; no credentials or
  raw audio in failures.
- REVERSAL: revert to M3 commit. No public-surface break: adapter
  behavior on healthy paths unchanged (M3 suite regression green).
