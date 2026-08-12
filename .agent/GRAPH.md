# EXECUTION GRAPH

## Law

One node equals one self-contained ExecPlan, one lease, one green tag, and one bounded unit of evidence. The graph is immutable during a run. `scripts/graph-next.sh` is the only scheduling authority. A node is DONE only when all milestones pass, node verify prints its sentinel, the expected-file audit passes, NODE_DONE is appended, and `green/<node>` exists.

GRAPH-TABLE-BEGIN
NODE EP-000 DEPS -
NODE EP-001 DEPS EP-000
NODE EP-002 DEPS EP-001
NODE EP-003 DEPS EP-002
NODE EP-004 DEPS EP-003
NODE EP-005 DEPS EP-004
NODE EP-006 DEPS EP-005
NODE EP-007 DEPS EP-006
NODE EP-008 DEPS EP-007
NODE EP-009 DEPS EP-008
NODE EP-010 DEPS EP-009
NODE EP-011 DEPS EP-010
NODE EP-012 DEPS EP-011
NODE EP-013 DEPS EP-012
NODE EP-014 DEPS EP-013
NODE EP-015 DEPS EP-014
NODE EP-016 DEPS EP-015
NODE EP-017 DEPS EP-016
NODE EP-018 DEPS EP-017
NODE EP-019 DEPS EP-018
NODE EP-020 DEPS EP-019
NODE EP-021 DEPS EP-020
NODE EP-022 DEPS EP-021
NODE EP-023 DEPS EP-022
NODE EP-024 DEPS EP-023
NODE EP-025 DEPS EP-024
NODE EP-026 DEPS EP-025
NODE EP-027 DEPS EP-026
NODE EP-028 DEPS EP-027
NODE EP-029 DEPS EP-028
NODE EP-030 DEPS EP-029
NODE EP-031 DEPS EP-030
NODE EP-032 DEPS EP-031
NODE EP-033 DEPS EP-032
NODE EP-034 DEPS EP-033
NODE EP-035 DEPS EP-034
NODE EP-036 DEPS EP-035
NODE EP-037 DEPS EP-036
NODE EP-038 DEPS EP-037
NODE EP-039 DEPS EP-038
NODE EP-040 DEPS EP-039
NODE EP-041 DEPS EP-040
NODE EP-042 DEPS EP-041
NODE EP-043 DEPS EP-042
GRAPH-TABLE-END

## Dispatch

- `NEXT <id>`: append LEASE, open the named ExecPlan, and execute its first unchecked milestone.
- `RESUME <id>`: continue only if the lease is yours. A different lease may be taken over only after ninety minutes without a ledger event; append LEASE_TAKEOVER first.
- `BLOCKED <id>`: terminal halt. Read the structured report and make only its named human decision.
- `STALL <id>`: graph defect. Append NODE_BLOCKED with `GRAPH_STALL` and halt.
- `ALL_DONE`: run the ship gate, produce the signed release, print the manual deployment command, and append RUN_COMPLETE.

## Checkpoints

Commit every milestone as `[EP-NNN][Mk] <imperative summary>`. Tag a finished node `green/EP-NNN`. Rollback never crosses a completed green tag. State is derived from the append-only ledger; no parallel status file exists.

## Multi-agent cohesion

Git plus the ledger are the complete coordination bus. Run the scheduler fresh before leasing. Append HEARTBEAT every fifteen minutes and after each milestone. Release a lease before handing off. Platform memory and chat history are never authoritative.

## Build arc

The graph first establishes truthful toolchain, domain, data, eventing, workflows, identity, policy, trust, and universal contracts. It then adds model intelligence, memory, agents, skills, self-healing, home and communication providers, business and social control, Sentinel, user clients, onboarding, provisioning, storage, observability, supply-chain controls, Microbrain R&D, release lifecycle, and final live-fire certification.

| Node | Dependencies | Purpose | Specs | ExecPlan |
| --- | --- | --- | --- | --- |
| EP-000 | - | Discovery, source verification, toolchain lock, license baseline, and truthful command surface | SPEC-000, SPEC-019 | .agent/execplans/EP-000-discovery-and-toolchain.md |
| EP-001 | EP-000 | Create the polyglot monorepo, generated-contract pipeline, stage-aware gates, and CI skeleton | SPEC-000, SPEC-006 | .agent/execplans/EP-001-foundation-and-monorepo.md |
| EP-002 | EP-001 | Implement canonical IDs, vocabularies, schemas, component registry, and provider-neutral contracts | SPEC-001, SPEC-003, SPEC-022 | .agent/execplans/EP-002-domain-contracts-and-vocabulary.md |
| EP-003 | EP-002 | Implement people, households, businesses, devices, sessions, presence evidence, and tenant boundaries | SPEC-001, SPEC-005 | .agent/execplans/EP-003-identity-people-devices-and-tenancy.md |
| EP-004 | EP-003 | Implement PostgreSQL, pgvector, repositories, memory records, world graph abstraction, and migrations | SPEC-002 | .agent/execplans/EP-004-data-memory-and-world-graph.md |
| EP-005 | EP-004 | Implement NATS JetStream, canonical events, outbox, replay, correlation, and durable consumers | SPEC-023 | .agent/execplans/EP-005-event-nervous-system.md |
| EP-006 | EP-005 | Implement Temporal namespaces, workers, workflow contracts, approvals, retries, signals, and cancellation | SPEC-023 | .agent/execplans/EP-006-durable-workflows.md |
| EP-007 | EP-006 | Deploy Keycloak and implement OIDC, passkeys, service identities, sessions, device enrollment, and step-up | SPEC-005 | .agent/execplans/EP-007-authentication-and-passkeys.md |
| EP-008 | EP-007 | Implement OpenFGA, OPA, risk classes, short-lived grants, deterministic Action Gateway, verification, and receipts | SPEC-005, SPEC-006 | .agent/execplans/EP-008-authorization-policy-and-action-gateway.md |
| EP-009 | EP-008 | Implement OpenBao, SOPS and age bootstrap, device stores, certificate authority, Headscale, WireGuard, and mTLS | SPEC-005, SPEC-020 | .agent/execplans/EP-009-secrets-trust-and-private-mesh.md |
| EP-010 | EP-009 | Implement capability discovery, health, command, query, event, and connector-tier contracts | SPEC-003, SPEC-022 | .agent/execplans/EP-010-capability-registry-and-connector-contract.md |
| EP-011 | EP-010 | Build Rust, Python, and TypeScript connector SDKs plus a sandboxed legacy Connector Sidecar | SPEC-022 | .agent/execplans/EP-011-connector-sdks-and-sidecar-runtime.md |
| EP-012 | EP-011 | Implement REST, WebSocket, MCP Streamable HTTP, A2A, artifact exchange, and scoped context capsules | SPEC-003 | .agent/execplans/EP-012-api-mcp-and-a2a-fabric.md |
| EP-013 | EP-012 | Implement the model provider registry, Bifrost-preferred gateway adapter, budgets, fallbacks, and provider health | SPEC-009 | .agent/execplans/EP-013-model-gateway-and-provider-registry.md |
| EP-014 | EP-013 | Implement DeepSeek V4 Flash ReflexProvider, effort tiers, deterministic prompt segments, cache accounting, and schema validation | SPEC-009 | .agent/execplans/EP-014-deepseek-reflex-and-cache.md |
| EP-015 | EP-014 | Implement the Nexus Model Router Contract, policy routing, RouteLLM-compatible scoring, escalation, and Microbrain interface | SPEC-009, SPEC-025 | .agent/execplans/EP-015-model-router-and-microbrain-seam.md |
| EP-016 | EP-015 | Implement hybrid retrieval, context capsules, memory consolidation, retention, privacy, and graph-aware context construction | SPEC-002 | .agent/execplans/EP-016-context-engine-and-memory-consolidation.md |
| EP-017 | EP-016 | Implement objectives, task graph, agent registry, A2A adapters, Codex, Claude Code, Hermes, OpenClaw, budgets, and artifacts | SPEC-010 | .agent/execplans/EP-017-agent-orchestrator-and-harness-adapters.md |
| EP-018 | EP-017 | Implement signed Agent Skills packages, trust levels, permissions, evals, promotion, composition, and versioning | SPEC-010 | .agent/execplans/EP-018-skill-registry-and-skill-factory.md |
| EP-019 | EP-018 | Implement incident correlation, diagnosis, patching, independent review, HITL approval, canary, verification, and rollback | SPEC-018 | .agent/execplans/EP-019-self-healing-engineering-loop.md |
| EP-020 | EP-019 | Implement Home Assistant provider, discovery, canonical device mapping, local fast path, verification, and automation handoff | SPEC-011 | .agent/execplans/EP-020-home-assistant-and-device-control.md |
| EP-021 | EP-020 | Implement audio ingest, VAD, custom wake word, local STT, local TTS, speaker evidence, cloud fallbacks, and privacy controls | SPEC-012 | .agent/execplans/EP-021-voice-core-stt-tts-wake-and-speaker-id.md |
| EP-022 | EP-021 | Implement Assist and Wyoming satellites, top-ten hardware matrix, Bluetooth endpoints, AEC, endpoint transfer, and room routing | SPEC-012 | .agent/execplans/EP-022-voice-satellites-bluetooth-and-audio-routing.md |
| EP-023 | EP-022 | Implement Frigate, go2rtc, camera capability provider, Roku discovery and fallback ladder, visitor events, and two-way audio where verified | SPEC-021 | .agent/execplans/EP-023-frigate-vision-and-roku-home-provider.md |
| EP-024 | EP-023 | Implement Sonos, TV, media, lighting, HVAC, vacuum, irrigation, appliance, vehicle, and future robot provider contracts | SPEC-011 | .agent/execplans/EP-024-media-appliances-irrigation-and-robotics-providers.md |
| EP-025 | EP-024 | Implement Asterisk LTS, SIP provider abstraction, bidirectional media, governed call workflows, STT and TTS, disclosure, and transcripts | SPEC-014 | .agent/execplans/EP-025-asterisk-telephony-and-ai-calling.md |
| EP-026 | EP-025 | Implement universal mailboxes, Gmail, Microsoft Graph, IMAP and SMTP, self-hosted mail option, attachments, drafts, sends, and audit | SPEC-014 | .agent/execplans/EP-026-email-fabric.md |
| EP-027 | EP-026 | Implement ICTFax, HylaFAX compatibility, fax documents, inbound routing, outbound status, T.38 or carrier fallback, and audit | SPEC-014 | .agent/execplans/EP-027-fax-fabric.md |
| EP-028 | EP-027 | Implement the authenticated Nexus-to-Hydra capability, context, action, event, identity, and business binding seam | SPEC-015 | .agent/execplans/EP-028-hydra-business-control-plane.md |
| EP-029 | EP-028 | Implement Postiz-isolated connector, direct official APIs, content, community, analytics, approvals, CRM lead handoff, and attribution | SPEC-015 | .agent/execplans/EP-029-social-command-center.md |
| EP-030 | EP-029 | Implement OPNsense and OpenWrt adapters, AdGuard Home, inventory, segmentation, baselines, anomaly scoring, and quarantine proposals | SPEC-013 | .agent/execplans/EP-030-sentinel-core-network-and-dns.md |
| EP-031 | EP-030 | Implement optional Suricata, Zeek, CrowdSec, Wazuh or osquery profiles, honeypots, triage, investigation, response, and verification | SPEC-013 | .agent/execplans/EP-031-sentinel-advanced-detection-and-endpoints.md |
| EP-032 | EP-031 | Implement person-aware push, desktop, speaker, SMS, email, phone, watch, car, privacy, urgency, quiet hours, and escalation routing | SPEC-014 | .agent/execplans/EP-032-notification-and-communications-router.md |
| EP-033 | EP-032 | Implement accessible React PWA, cloud dashboard, chat, operations center, approvals, settings, security console, and Tauri desktop | SPEC-004, SPEC-017 | .agent/execplans/EP-033-web-dashboard-and-desktop.md |
| EP-034 | EP-033 | Implement Flutter iOS and Android apps, passkeys, biometrics, voice, push, Bluetooth, approvals, remote controls, and secure local storage | SPEC-017 | .agent/execplans/EP-034-ios-and-android-mobile.md |
| EP-035 | EP-034 | Implement Nexus Setup, owner recovery, deployment choice, hardware profiling, secure bootstrap, home-edge QR enrollment, discovery, people, and integration cards | SPEC-004, SPEC-016 | .agent/execplans/EP-035-setup-wizard-and-onboarding.md |
| EP-036 | EP-035 | Implement node registry, workload placement, OpenTofu modules, cloud-init, Contabo, Hetzner, DigitalOcean, AWS, generic SSH, and private mesh | SPEC-016 | .agent/execplans/EP-036-compute-fabric-and-cloud-provisioning.md |
| EP-037 | EP-036 | Implement ArtifactStore with local, NAS, SeaweedFS, MinIO compatibility, R2, B2, and S3 plus encrypted backup, restore, and migration | SPEC-024 | .agent/execplans/EP-037-artifact-storage-backup-and-disaster-recovery.md |
| EP-038 | EP-037 | Implement OpenTelemetry, GlitchTip, metrics, logs, traces, dashboards, alerts, SLOs, fleet health, and incident operations | SPEC-007 | .agent/execplans/EP-038-observability-and-operations.md |
| EP-039 | EP-038 | Implement license policy, sidecar boundaries, SBOM, provenance, signed artifacts, image scanning, dependency policy, and advisory monitoring | SPEC-019 | .agent/execplans/EP-039-license-sbom-and-supply-chain.md |
| EP-040 | EP-039 | Complete contract, integration, E2E, security, accessibility, performance, chaos, provider certification, hardware lab, and flaky-test elimination | SPEC-008 | .agent/execplans/EP-040-testing-hardening-and-chaos.md |
| EP-041 | EP-040 | Implement the separate Microbrain dataset, frozen evals, teacher consensus, QLoRA pipeline, GGUF export, shadow comparison, and canary tooling | SPEC-025 | .agent/execplans/EP-041-microbrain-training-factory.md |
| EP-042 | EP-041 | Implement signed releases, installers, offline bundle, transactional updates, staged rollout, backup-before-update, provider migration, and rollback drills | SPEC-016, SPEC-024 | .agent/execplans/EP-042-deployment-release-update-and-rollback.md |
| EP-043 | EP-042 | Execute all live-fire proofs, security and privacy review, load and hardware certification, restore and rollback drills, docs audit, release tag, and manual deploy handoff | SPEC-008 | .agent/execplans/EP-043-production-readiness-and-ship.md |
