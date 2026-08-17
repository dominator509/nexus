# NEXUS CERTIFICATION REGISTRY

Machine-readable certification registry (OWNER ARCHITECTURE DIRECTIVE
section 7). One component per block, `key: value` lines, ASCII only.

## Status vocabulary

- NOT_IMPLEMENTED
- IMPLEMENTED
- INTERNAL_CERTIFIED
- PROVIDER_CERTIFIED
- HARDWARE_CERTIFIED
- PRODUCTION_CERTIFIED
- DEFERRED

Implementation, integration, and certification are tracked separately.
A component may be IMPLEMENTED before its external certification exists;
that is not simulation. DEFERRED rows must name a certification_owner.
At the ship gate (SPEC-008; EP-040/EP-043) every required capability
must reach its required level: PROVIDER_CERTIFIED for required external
providers, HARDWARE_CERTIFIED for required physical hardware,
PRODUCTION_CERTIFIED for the core runtime. IMPLEMENTED or DEFERRED rows
that are blocking_for_ship=true fail the ship gate.

Update rule: append or edit rows only with ledger evidence. This file
lives under .agent/state/evidence so every node may maintain its own
rows without fence churn.

## Components

## Component: nexus-context
component_id: nexus-context
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: none (provider-neutral ports by design)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-context; EP-016 M1 gate (24 ep016_unit tests + 1 dependency-direction); scope audit EP-016: ok

## Component: nexus-memory-workers
component_id: nexus-memory-workers
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: none (candidate/source/graph/semantic I/O injected through ports by design)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-memory-workers; EP-016 M2 gate (58 unit tests + 1 dependency-direction, 3 suites); clippy -D warnings clean; lint: ok

## Component: memory-workflow-contracts
component_id: memory-workflow-contracts
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: none (Temporal workflow contracts; real engine integration owned by the Temporal runtime node EP-006)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: packages/workflows/src/memory/; EP-016 M3 gate (14 ep016_integration tests via real vitest + vacuity guard + tsc --noEmit clean)

## Component: memory-plane-real-composition
component_id: memory-plane-real-composition
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: postgresql, pgvector, temporal (open-source infrastructure)
provider_certification: INTERNAL_CERTIFIED (real containers/services proved by owning nodes EP-004/EP-006)
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: EP-004 (PostgreSQL, pgvector, repositories, memory records, world graph); EP-006 (Temporal durable workflows); EP-016 worker ports consume these at the composition boundary
graph_gap_note: No node contract explicitly names composing the EP-016 context workers with the real EP-004 repositories and EP-006 Temporal runtime; the earliest consuming node per graph direction (agent orchestration and downstream) is the natural owner. EP-040/EP-043 must confirm an explicit integration owner at ship-gate review or add a certification/integration node (directive section 6).

## Component: agent-workflow-contracts
component_id: agent-workflow-contracts
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: none (Temporal workflow contracts; real engine integration owned by the Temporal runtime node EP-006)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship); real Temporal execution owned by EP-006 substrate, explicit agent-workflow composition owner to be confirmed at ship-gate review (EP-040/EP-043)
blocking_for_ship: false
evidence_reference: packages/workflows/src/agents/; EP-017 M3 gate (10 ep017_integration tests via real vitest + vacuity guard + tsc --noEmit clean); TypeScript workflow/state logic executed under real Vitest, NOT against a real Temporal server (no fake Temporal client; real engine integration deferred per EP-006 ownership)

## Component: agent-harness-adapters
component_id: agent-harness-adapters
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (real process boundary: production ProcessRunner spawns real subprocesses; deterministic registry + orchestrator + CliHarnessAdapter proven by 20 ep017_unit + 35 ep017_failure + 5 lf016 tests)
provider: codex, claude-code, hermes, openclaw (external coding-agent CLIs)
provider_certification: DEFERRED (real Codex/Claude Code/Hermes/OpenClaw CLIs NOT installed in this environment; no provider credential present; LF-016 proves the real process boundary through a CONTROLLED_TEST_FIXTURE only)
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship; external coding-agent provider certification owner to be confirmed at ship-gate review per directive section 6)
blocking_for_ship: false
evidence_reference: crates/nexus-harness-adapters; tests/agents/fixtures/coding-agent-fixture.sh (CONTROLLED_TEST_FIXTURE); LF-016-ep017-m5.md (real subprocess spawn, exit-status mapping, cancellation, fail-closed nonzero exit); EP-017 M2/M4 gates

## Component: control-plane-runtime
component_id: control-plane-runtime
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: none (self-hosted runtime)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: apps/control-plane; infra/compose/core.yaml; Dockerfile; /healthz /readyz /v1/capabilities real handlers; EP-044-M5-live-fire.md (real container, HTTP 200 bodies, local stop, no orphan, LF-029 regression ok)

## Component: deepseek-reflex
component_id: deepseek-reflex
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: deepseek (deepseek-v4-flash)
provider_certification: PROVIDER_CERTIFIED
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-reflex; EP-014-M5-live-fire.md (real provider route deepseek-v4-flash, 8 canonical requests, mandatory runtime smoke real container PASS)

## Component: model-gateway-provider-registry
component_id: model-gateway-provider-registry
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: bifrost (internal gateway 127.0.0.1:8000), deepseek fallback
provider_certification: INTERNAL_CERTIFIED for bifrost (internal infrastructure); deepseek-v4-flash fallback PROVIDER_CERTIFIED via EP-014
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-model-gateway, nexus-model-transport, nexus-bifrost; ep013-m5-live-fire.json (allow/deny paths, budget, rate limit, usage accounting, real transport)

## Component: model-router-microbrain-seam
component_id: model-router-microbrain-seam
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED
provider: reflex providers (DeepSeek, Microbrain via ReflexProvider contract)
provider_certification: INTERNAL_CERTIFIED (real transport attempt, connection-refused -> UNAVAILABLE, failover typed lock; real provider certification deferred to provider owner nodes)
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-router seam; LF-021-ep015-m5.md (real transport attempt; only UNAVAILABLE/TIMEOUT failover-eligible; typed lock)

## Component: skill-plane-contracts
component_id: skill-plane-contracts
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (vocabulary + manifest + signature structure + package identity + proposal lifecycle + permission authority; 58 ep018_unit M1 tests, clippy clean, schema parity with canonical schemas)
provider: none (provider-neutral contract crate)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-skills (vocabulary.rs, manifest.rs, signature.rs, proposal.rs, composer.rs); ADR-025; 58 ep018_unit contract tests

## Component: skill-plane-bundle-registry
component_id: skill-plane-bundle-registry
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (real bundle loader with real SHA-256 scan-before-install content hashing, real JSON-file durable registry store, install/remove/revoke lifecycle with terminal revoked flag, immutable-by-version conflict rejection, rollback on persistence failure, durable revocation across store reload, no resurrection of revoked identity)
provider: none (self-hosted skill plane)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-skills (bundle.rs, store.rs, registry.rs); skills/ real bundles; 17 ep018_unit M2 tests; 19 ep018_failure M4 tests; real on-disk tamper test (one-byte flip -> digest change -> CONFLICT, never executable)

## Component: skill-plane-schema-validation
component_id: skill-plane-schema-validation
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (canonical JSON Schema 2020-12 documents validated by the real jsonschema 0.49.9 crate; Rust serde surface and on-disk bundles conform; unknown permissions/non-canonical ids/invalid semvers rejected by schema)
provider: none
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: schemas/skills/; crates/nexus-skills/tests/ep018_integration_schema.rs (5 tests)

## Component: skill-plane-signature-crypto
component_id: skill-plane-signature-crypto
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (real ring 0.17.14 Ed25519 keypair generation, signing, and verification over the canonical package identity digest; tampered content FAILS, wrong signer FAILS, bad signature FAILS; structural hex validation remains distinct from cryptographic certification)
provider: none (ring is a vetted pinned internal crypto implementation, locked workspace dep via rustls/rcgen)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-skills/src/signature.rs; LF-018-ep018-m5.md (real keypair -> sign canonical digest -> verify PASS; tamper/wrong-signer/bad-signature FAIL)

## Component: skill-plane-process-execution
component_id: skill-plane-process-execution
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (REAL_INTERNAL_PROCESS: SkillExecutor spawns the installed skill payload as a real subprocess with a scrubbed environment, capped output, real exit-status mapping, fail-closed on spawn failure; declared permissions must be within the caller grant; WRITE at runtime denied when not granted)
provider: none (self-hosted execution boundary; payload is CONTROLLED_TEST_FIXTURE)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-skills/src/executor.rs; tests/skills/fixtures/livefire-transform.sh (CONTROLLED_TEST_FIXTURE); LF-018-ep018-m5.md (input -> transformation -> output artifact; runtime WRITE denied exit 3; revoke -> execution denied)

## Component: skill-plane-sandbox
component_id: skill-plane-sandbox
implementation_status: IMPLEMENTED (enforcement decision PASS: permission ceiling + caller-grant intersection + fail-closed execution boundary deny undeclared capabilities at runtime)
internal_proof: INTERNAL_CERTIFIED (permission/trust enforcement proven by M1/M4 tests and LF-018 runtime WRITE denial; real process boundary proven by SkillExecutor)
provider: none
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: LF-018 runtime permission denial; permission ceiling tests; real sandbox execution certification DEFERRED (no OS-level isolation proof in EP-018; sandbox requirement/enforcement decision PASS, real sandbox execution certification deferred to EP-043/EP-040)

## Component: skill-plane-external-registry
component_id: skill-plane-external-registry
implementation_status: NOT ASSERTED (no external/public skill registry is claimed, owned, or exercised by EP-018)
internal_proof: N/A
provider: none
provider_certification: NOT ASSERTED
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: EP-018 non-goals; directive section T (manifest network_rules are REQUESTS, never firewall configuration; real network enforcement deferred to Sentinel/EP-030 class owners)


## Component: self-healing-engine
component_id: self-healing-engine
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (deterministic self-healing lifecycle: IncidentState 18-state vocabulary with explicit terminals, DiagnosisConfidence escalation, incident memory dedup/idempotency, approval digest binding, canary/rollback state machines; 19 ep019_unit + 13 ep019_failure + 12 ep019_integration + 1 LF-019 tests green)
provider: none (self-hosted engineering loop; controlled failing fixture CONTROLLED_TEST_FIXTURE)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-healing/; packages/workflows/src/incidents/; tests/healing/; LF-019-ep019-m5.md (real failing fixture -> real subprocess incident -> reproduce -> patch -> review -> approval -> canary -> verify -> close/rollback)

## Component: self-healing-sandbox
component_id: self-healing-sandbox
implementation_status: IMPLEMENTED (enforcement decision PASS: isolated working copy + fail-closed execution boundary + scope preservation + sandbox/security verdicts)
internal_proof: INTERNAL_CERTIFIED (patch applies to isolated copy, reproduction FAIL->PASS, fail-closed preserved after patch, scope remains allowed; security gate failures rejected)
provider: none
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: EP-040/EP-043 (real OS-level sandbox isolation certification deferred)
blocking_for_ship: false
evidence_reference: LF-019 isolated working copy; ep019_m3 integration; runbooks/self-healing/README.md; real sandbox execution certification DEFERRED (no OS-level isolation proof in EP-019)

## Component: self-healing-production-deployment
component_id: self-healing-production-deployment
implementation_status: IMPLEMENTED (deterministic canary/rollback state machine; staged contract recorded)
internal_proof: INTERNAL_CERTIFIED (CanaryState + RollbackState machines, auto_rollback_on_regression, rollback bound to known previous artifact; LF-019 rollback proof restores failing behavior = health restored to known previous state)
provider: none (production deployment substrate later-owned)
provider_certification: N/A
hardware_certification: N/A
production_certification: DEFERRED
certification_owner: deployment-owning node (EP-042/EP-043 real production canary certification)
blocking_for_ship: false
evidence_reference: crates/nexus-healing/src/canary.rs + rollback.rs; LF-019-ep019-m5.md rollback section; real production canary deployment certification DEFERRED to the deployment-owning node (no simulated canary)

## Component: home-assistant-provider
component_id: home-assistant-provider
implementation_status: IMPLEMENTED (real HA provider adapter + contract crate)
internal_proof: INTERNAL_CERTIFIED (ep020_unit contract + adapter suites; dependency direction nexus-domain+serde only)
provider: ghcr.io/home-assistant/home-assistant:stable@sha256:56690a89c79a0de98035e1719f8324a92d5859c1192ff45adb0230ea81cb42a5 (Apache-2.0; running version 2026.8.2)
provider_certification: PROVIDER_CERTIFIED (real container live-fire: real auth/OAuth login_flow, discovery + canonical mapping, real service execution, exact-target readback, WebSocket state_changed, programmatic automation creation via config API, automation persistence + conditional behavior, offline/reconnect queue; M3 19 integration + M4 9 failure + LF-006/LF-007/LF-024 real proofs)
hardware_certification: NOT ASSERTED (template-light entity light.nexus_test_light = CONTROLLED_TEST_FIXTURE; physical light hardware certification DEFERRED to its certification owner; no NODE_BLOCK)
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-home; connectors/home-assistant; infra/home-assistant; tests/home; .agent/state/evidence/EP-020-M3-real-ha-provider.md; EP-020-M4-forced-failures.md; EP-020-M5-real-provider-livefire.md

## Component: nexus-audio (EP-022 contract crate)
component_id: nexus-audio
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (16 ep022_unit tests; vocabulary/endpoint/router/transfer/satellite/bluetooth/AEC contracts; M1 gate green)
provider: none (provider-neutral ports by design)
provider_certification: N/A
hardware_certification: NOT ASSERTED (hardware/voice/profiles.yaml conformance DEFINED only; physical classes never upgraded from YAML)
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-audio; EP-022 M1 gate; hardware/voice/profiles.yaml

## Component: nexus-assist-satellite (EP-022 M2 adapter core)
component_id: nexus-assist-satellite
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (16 ep022_unit tests; local wake gating, hardware mute authority, context survival; M2 gate green)
provider: none (I/O-agnostic ports; real transports owned by M3/M4/M5)
provider_certification: N/A
hardware_certification: NOT ASSERTED
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: connectors/assist-satellite; EP-022 M2 gate; LF-026 e2e (tests/audio)

## Component: wyoming-connector (EP-022 M3 transport)
component_id: wyoming-connector
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (4 ep022_integration tests vs REAL rhasspy/wyoming-openwakeword container digest 52cb1168...d42b; real Describe/Detection/NotDetected wire events; M3 gate green)
provider: rhasspy/wyoming-openwakeword (Apache-2.0 classifier, MIT LICENSE text)
provider_certification: INTERNAL_CERTIFIED (real container + real protocol; container wake models are upstream fixtures per SPEC-019)
hardware_certification: NOT ASSERTED
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship); Nexus wake model certification DEFERRED per SPEC-019 (EP-021 M3 graph gap)
blocking_for_ship: false
evidence_reference: connectors/wyoming; COMPONENT_REGISTRY.yaml wyoming-openwakeword row; EP-022 M3 gate

## Component: nexus-bluetooth-audio (EP-022 M4 connector)
component_id: nexus-bluetooth-audio
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (13 ep022_failure tests; real D-Bus wire client verified byte-for-byte vs live dbus-send capture; 3 real system-bus tests; real peer fault injection; M4 gate green)
provider: host system D-Bus daemon (real bus; org.bluez absent on this host)
provider_certification: NOT ASSERTED (BlueZ absence proven by real GetNameOwner NameHasNoOwner; connector fails closed UNAVAILABLE)
hardware_certification: NOT ASSERTED (Bluetooth/A2DP transport certification DEFERRED to hardware ownership)
production_certification: DEFERRED
certification_owner: EP-040/EP-043 (real Bluetooth hardware transport certification; never claimed from the connector)
blocking_for_ship: false
evidence_reference: connectors/bluetooth-audio; COMPONENT_REGISTRY.yaml bluez row; EP-022 M4 gate; LF-026 bluetooth leg

## Component: nexus-audio-e2e (EP-022 M5 cross-node proof)
component_id: nexus-audio-e2e
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (5 ep022_e2e tests composing real nexus-audio + nexus-assist-satellite + nexus-bluetooth-audio; LF-026 live-fire green with machine-readable evidence)
provider: none (composition proof)
provider_certification: N/A
hardware_certification: NOT ASSERTED
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: tests/audio; .agent/state/evidence/EP-022-M5-LF-026-voice-endpoint-transfer.json

## Component: nexus-vision (EP-023 M1 contract crate)
component_id: nexus-vision
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (13 ep023_unit tests; camera/stream/identity/two-way/roku contracts; StreamRef no-unverified-claim, two-way fails closed, advisory identity; M1 gate green)
provider: none (provider-neutral ports by design)
provider_certification: N/A
hardware_certification: NOT ASSERTED (hardware/cameras/profiles.yaml conformance DEFINED only; physical camera classes never upgraded from YAML)
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: crates/nexus-vision; EP-023 M1 gate; hardware/cameras/profiles.yaml

## Component: nexus-frigate (EP-023 M2/M3/M4 connector)
component_id: nexus-frigate
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (28 ep023_unit + 10 ep023_integration + 18 ep023_failure tests; real adapter against documented Frigate HTTP/go2rtc API; real media chain; forced-failure suite; observability + frigate-diag; M2/M3/M4 gates green)
provider: ghcr.io/blakeblackshear/frigate:0.17.2@sha256:d4351369984d4a9e2a49ac59736f6490856a7ea11f7790040746d21496967010 (embedded go2rtc v1.9.10 df95ce3); mediamtx v1.20.0 sha256 25947caac403f37ec881c9be213af2cad67e344a6c7098905b0d31c17f40e336 CONTROLLED_TEST_FIXTURE transport
provider_certification: INTERNAL_CERTIFIED (real pinned container + real RTSP/media chain + real person detection in LF-008)
hardware_certification: NOT ASSERTED (no physical camera; stream refs stay Unverified)
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship); Roku/physical-camera hardware DEFERRED to EP-040/EP-043
blocking_for_ship: false
evidence_reference: connectors/frigate; COMPONENT_REGISTRY.yaml frigate/go2rtc/mediamtx rows; EP-023 M2/M3/M4 gates; LF-008

## Component: nexus-roku-home (EP-023 M5 provider ladder)
component_id: nexus-roku-home
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (5 ep023_unit_roku tests; real fail-closed RokuHomeProviderHost: empty inventory, tier UNAVAILABLE, canonical ladder select_tier; M5 gate green)
provider: none (no Roku hardware/credentials bound on this host)
provider_certification: N/A
hardware_certification: NOT ASSERTED (Roku HARDWARE_CERTIFICATION DEFERRED to EP-040/EP-043; no physical device; never fabricated)
production_certification: DEFERRED
certification_owner: EP-040/EP-043 (real Roku hardware transport certification; never claimed from the connector)
blocking_for_ship: false
evidence_reference: connectors/roku-home; EP-023 M5 gate; .agent/state/evidence/EP-023-M5-LF-008-visitor-response.json (roku_tier UNAVAILABLE)

## Component: nexus-vision-e2e (EP-023 M5 cross-node proof)
component_id: nexus-vision-e2e
implementation_status: IMPLEMENTED
internal_proof: INTERNAL_CERTIFIED (4 ep023_e2e pure-contract tests + LF-008 journey composing real nexus-vision + nexus-frigate + nexus-roku-home; real person event -> VisitorEvent -> notification decision -> two-way NOT certified; M5 gate green)
provider: none (composition proof)
provider_certification: N/A
hardware_certification: NOT ASSERTED (two-way audio live certification NOT ASSERTED; requires real speaker/media path)
production_certification: DEFERRED
certification_owner: EP-043 (production readiness and ship)
blocking_for_ship: false
evidence_reference: tests/vision; .agent/state/evidence/EP-023-M5-LF-008-visitor-response.json
