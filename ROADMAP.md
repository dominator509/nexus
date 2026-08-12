Do not implement from this file. Implementation happens only through the graph: run `sh scripts/graph-next.sh`.

# ROADMAP

## EP-000: Discovery And Toolchain

Purpose: Discovery, source verification, toolchain lock, license baseline, and truthful command surface.

Dependencies: -.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-000` exists.

Specifications: SPEC-000, SPEC-019. ExecPlan: `.agent/execplans/EP-000-discovery-and-toolchain.md`.

## EP-001: Foundation And Monorepo

Purpose: Create the polyglot monorepo, generated-contract pipeline, stage-aware gates, and CI skeleton.

Dependencies: EP-000.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-001` exists.

Specifications: SPEC-000, SPEC-006. ExecPlan: `.agent/execplans/EP-001-foundation-and-monorepo.md`.

## EP-002: Domain Contracts And Vocabulary

Purpose: Implement canonical IDs, vocabularies, schemas, component registry, and provider-neutral contracts.

Dependencies: EP-001.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-002` exists.

Specifications: SPEC-001, SPEC-003, SPEC-022. ExecPlan: `.agent/execplans/EP-002-domain-contracts-and-vocabulary.md`.

## EP-003: Identity People Devices And Tenancy

Purpose: Implement people, households, businesses, devices, sessions, presence evidence, and tenant boundaries.

Dependencies: EP-002.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-003` exists.

Specifications: SPEC-001, SPEC-005. ExecPlan: `.agent/execplans/EP-003-identity-people-devices-and-tenancy.md`.

## EP-004: Data Memory And World Graph

Purpose: Implement PostgreSQL, pgvector, repositories, memory records, world graph abstraction, and migrations.

Dependencies: EP-003.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-004` exists.

Specifications: SPEC-002. ExecPlan: `.agent/execplans/EP-004-data-memory-and-world-graph.md`.

## EP-005: Event Nervous System

Purpose: Implement NATS JetStream, canonical events, outbox, replay, correlation, and durable consumers.

Dependencies: EP-004.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-005` exists.

Specifications: SPEC-023. ExecPlan: `.agent/execplans/EP-005-event-nervous-system.md`.

## EP-006: Durable Workflows

Purpose: Implement Temporal namespaces, workers, workflow contracts, approvals, retries, signals, and cancellation.

Dependencies: EP-005.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-006` exists.

Specifications: SPEC-023. ExecPlan: `.agent/execplans/EP-006-durable-workflows.md`.

## EP-007: Authentication And Passkeys

Purpose: Deploy Keycloak and implement OIDC, passkeys, service identities, sessions, device enrollment, and step-up.

Dependencies: EP-006.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-007` exists.

Specifications: SPEC-005. ExecPlan: `.agent/execplans/EP-007-authentication-and-passkeys.md`.

## EP-008: Authorization Policy And Action Gateway

Purpose: Implement OpenFGA, OPA, risk classes, short-lived grants, deterministic Action Gateway, verification, and receipts.

Dependencies: EP-007.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-008` exists.

Specifications: SPEC-005, SPEC-006. ExecPlan: `.agent/execplans/EP-008-authorization-policy-and-action-gateway.md`.

## EP-009: Secrets Trust And Private Mesh

Purpose: Implement OpenBao, SOPS and age bootstrap, device stores, certificate authority, Headscale, WireGuard, and mTLS.

Dependencies: EP-008.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-009` exists.

Specifications: SPEC-005, SPEC-020. ExecPlan: `.agent/execplans/EP-009-secrets-trust-and-private-mesh.md`.

## EP-010: Capability Registry And Connector Contract

Purpose: Implement capability discovery, health, command, query, event, and connector-tier contracts.

Dependencies: EP-009.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-010` exists.

Specifications: SPEC-003, SPEC-022. ExecPlan: `.agent/execplans/EP-010-capability-registry-and-connector-contract.md`.

## EP-011: Connector Sdks And Sidecar Runtime

Purpose: Build Rust, Python, and TypeScript connector SDKs plus a sandboxed legacy Connector Sidecar.

Dependencies: EP-010.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-011` exists.

Specifications: SPEC-022. ExecPlan: `.agent/execplans/EP-011-connector-sdks-and-sidecar-runtime.md`.

## EP-012: Api Mcp And A2A Fabric

Purpose: Implement REST, WebSocket, MCP Streamable HTTP, A2A, artifact exchange, and scoped context capsules.

Dependencies: EP-011.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-012` exists.

Specifications: SPEC-003. ExecPlan: `.agent/execplans/EP-012-api-mcp-and-a2a-fabric.md`.

## EP-013: Model Gateway And Provider Registry

Purpose: Implement the model provider registry, Bifrost-preferred gateway adapter, budgets, fallbacks, and provider health.

Dependencies: EP-012.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-013` exists.

Specifications: SPEC-009. ExecPlan: `.agent/execplans/EP-013-model-gateway-and-provider-registry.md`.

## EP-014: Deepseek Reflex And Cache

Purpose: Implement DeepSeek V4 Flash ReflexProvider, effort tiers, deterministic prompt segments, cache accounting, and schema validation.

Dependencies: EP-013.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-014` exists.

Specifications: SPEC-009. ExecPlan: `.agent/execplans/EP-014-deepseek-reflex-and-cache.md`.

## EP-015: Model Router And Microbrain Seam

Purpose: Implement the Nexus Model Router Contract, policy routing, RouteLLM-compatible scoring, escalation, and Microbrain interface.

Dependencies: EP-014.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-015` exists.

Specifications: SPEC-009, SPEC-025. ExecPlan: `.agent/execplans/EP-015-model-router-and-microbrain-seam.md`.

## EP-016: Context Engine And Memory Consolidation

Purpose: Implement hybrid retrieval, context capsules, memory consolidation, retention, privacy, and graph-aware context construction.

Dependencies: EP-015.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-016` exists.

Specifications: SPEC-002. ExecPlan: `.agent/execplans/EP-016-context-engine-and-memory-consolidation.md`.

## EP-017: Agent Orchestrator And Harness Adapters

Purpose: Implement objectives, task graph, agent registry, A2A adapters, Codex, Claude Code, Hermes, OpenClaw, budgets, and artifacts.

Dependencies: EP-016.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-017` exists.

Specifications: SPEC-010. ExecPlan: `.agent/execplans/EP-017-agent-orchestrator-and-harness-adapters.md`.

## EP-018: Skill Registry And Skill Factory

Purpose: Implement signed Agent Skills packages, trust levels, permissions, evals, promotion, composition, and versioning.

Dependencies: EP-017.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-018` exists.

Specifications: SPEC-010. ExecPlan: `.agent/execplans/EP-018-skill-registry-and-skill-factory.md`.

## EP-019: Self Healing Engineering Loop

Purpose: Implement incident correlation, diagnosis, patching, independent review, HITL approval, canary, verification, and rollback.

Dependencies: EP-018.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-019` exists.

Specifications: SPEC-018. ExecPlan: `.agent/execplans/EP-019-self-healing-engineering-loop.md`.

## EP-020: Home Assistant And Device Control

Purpose: Implement Home Assistant provider, discovery, canonical device mapping, local fast path, verification, and automation handoff.

Dependencies: EP-019.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-020` exists.

Specifications: SPEC-011. ExecPlan: `.agent/execplans/EP-020-home-assistant-and-device-control.md`.

## EP-021: Voice Core Stt Tts Wake And Speaker Id

Purpose: Implement audio ingest, VAD, custom wake word, local STT, local TTS, speaker evidence, cloud fallbacks, and privacy controls.

Dependencies: EP-020.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-021` exists.

Specifications: SPEC-012. ExecPlan: `.agent/execplans/EP-021-voice-core-stt-tts-wake-and-speaker-id.md`.

## EP-022: Voice Satellites Bluetooth And Audio Routing

Purpose: Implement Assist and Wyoming satellites, top-ten hardware matrix, Bluetooth endpoints, AEC, endpoint transfer, and room routing.

Dependencies: EP-021.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-022` exists.

Specifications: SPEC-012. ExecPlan: `.agent/execplans/EP-022-voice-satellites-bluetooth-and-audio-routing.md`.

## EP-023: Frigate Vision And Roku Home Provider

Purpose: Implement Frigate, go2rtc, camera capability provider, Roku discovery and fallback ladder, visitor events, and two-way audio where verified.

Dependencies: EP-022.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-023` exists.

Specifications: SPEC-021. ExecPlan: `.agent/execplans/EP-023-frigate-vision-and-roku-home-provider.md`.

## EP-024: Media Appliances Irrigation And Robotics Providers

Purpose: Implement Sonos, TV, media, lighting, HVAC, vacuum, irrigation, appliance, vehicle, and future robot provider contracts.

Dependencies: EP-023.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-024` exists.

Specifications: SPEC-011. ExecPlan: `.agent/execplans/EP-024-media-appliances-irrigation-and-robotics-providers.md`.

## EP-025: Asterisk Telephony And Ai Calling

Purpose: Implement Asterisk LTS, SIP provider abstraction, bidirectional media, governed call workflows, STT and TTS, disclosure, and transcripts.

Dependencies: EP-024.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-025` exists.

Specifications: SPEC-014. ExecPlan: `.agent/execplans/EP-025-asterisk-telephony-and-ai-calling.md`.

## EP-026: Email Fabric

Purpose: Implement universal mailboxes, Gmail, Microsoft Graph, IMAP and SMTP, self-hosted mail option, attachments, drafts, sends, and audit.

Dependencies: EP-025.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-026` exists.

Specifications: SPEC-014. ExecPlan: `.agent/execplans/EP-026-email-fabric.md`.

## EP-027: Fax Fabric

Purpose: Implement ICTFax, HylaFAX compatibility, fax documents, inbound routing, outbound status, T.38 or carrier fallback, and audit.

Dependencies: EP-026.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-027` exists.

Specifications: SPEC-014. ExecPlan: `.agent/execplans/EP-027-fax-fabric.md`.

## EP-028: Hydra Business Control Plane

Purpose: Implement the authenticated Nexus-to-Hydra capability, context, action, event, identity, and business binding seam.

Dependencies: EP-027.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-028` exists.

Specifications: SPEC-015. ExecPlan: `.agent/execplans/EP-028-hydra-business-control-plane.md`.

## EP-029: Social Command Center

Purpose: Implement Postiz-isolated connector, direct official APIs, content, community, analytics, approvals, CRM lead handoff, and attribution.

Dependencies: EP-028.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-029` exists.

Specifications: SPEC-015. ExecPlan: `.agent/execplans/EP-029-social-command-center.md`.

## EP-030: Sentinel Core Network And Dns

Purpose: Implement OPNsense and OpenWrt adapters, AdGuard Home, inventory, segmentation, baselines, anomaly scoring, and quarantine proposals.

Dependencies: EP-029.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-030` exists.

Specifications: SPEC-013. ExecPlan: `.agent/execplans/EP-030-sentinel-core-network-and-dns.md`.

## EP-031: Sentinel Advanced Detection And Endpoints

Purpose: Implement optional Suricata, Zeek, CrowdSec, Wazuh or osquery profiles, honeypots, triage, investigation, response, and verification.

Dependencies: EP-030.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-031` exists.

Specifications: SPEC-013. ExecPlan: `.agent/execplans/EP-031-sentinel-advanced-detection-and-endpoints.md`.

## EP-032: Notification And Communications Router

Purpose: Implement person-aware push, desktop, speaker, SMS, email, phone, watch, car, privacy, urgency, quiet hours, and escalation routing.

Dependencies: EP-031.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-032` exists.

Specifications: SPEC-014. ExecPlan: `.agent/execplans/EP-032-notification-and-communications-router.md`.

## EP-033: Web Dashboard And Desktop

Purpose: Implement accessible React PWA, cloud dashboard, chat, operations center, approvals, settings, security console, and Tauri desktop.

Dependencies: EP-032.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-033` exists.

Specifications: SPEC-004, SPEC-017. ExecPlan: `.agent/execplans/EP-033-web-dashboard-and-desktop.md`.

## EP-034: Ios And Android Mobile

Purpose: Implement Flutter iOS and Android apps, passkeys, biometrics, voice, push, Bluetooth, approvals, remote controls, and secure local storage.

Dependencies: EP-033.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-034` exists.

Specifications: SPEC-017. ExecPlan: `.agent/execplans/EP-034-ios-and-android-mobile.md`.

## EP-035: Setup Wizard And Onboarding

Purpose: Implement Nexus Setup, owner recovery, deployment choice, hardware profiling, secure bootstrap, home-edge QR enrollment, discovery, people, and integration cards.

Dependencies: EP-034.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-035` exists.

Specifications: SPEC-004, SPEC-016. ExecPlan: `.agent/execplans/EP-035-setup-wizard-and-onboarding.md`.

## EP-036: Compute Fabric And Cloud Provisioning

Purpose: Implement node registry, workload placement, OpenTofu modules, cloud-init, Contabo, Hetzner, DigitalOcean, AWS, generic SSH, and private mesh.

Dependencies: EP-035.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-036` exists.

Specifications: SPEC-016. ExecPlan: `.agent/execplans/EP-036-compute-fabric-and-cloud-provisioning.md`.

## EP-037: Artifact Storage Backup And Disaster Recovery

Purpose: Implement ArtifactStore with local, NAS, SeaweedFS, MinIO compatibility, R2, B2, and S3 plus encrypted backup, restore, and migration.

Dependencies: EP-036.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-037` exists.

Specifications: SPEC-024. ExecPlan: `.agent/execplans/EP-037-artifact-storage-backup-and-disaster-recovery.md`.

## EP-038: Observability And Operations

Purpose: Implement OpenTelemetry, GlitchTip, metrics, logs, traces, dashboards, alerts, SLOs, fleet health, and incident operations.

Dependencies: EP-037.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-038` exists.

Specifications: SPEC-007. ExecPlan: `.agent/execplans/EP-038-observability-and-operations.md`.

## EP-039: License Sbom And Supply Chain

Purpose: Implement license policy, sidecar boundaries, SBOM, provenance, signed artifacts, image scanning, dependency policy, and advisory monitoring.

Dependencies: EP-038.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-039` exists.

Specifications: SPEC-019. ExecPlan: `.agent/execplans/EP-039-license-sbom-and-supply-chain.md`.

## EP-040: Testing Hardening And Chaos

Purpose: Complete contract, integration, E2E, security, accessibility, performance, chaos, provider certification, hardware lab, and flaky-test elimination.

Dependencies: EP-039.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-040` exists.

Specifications: SPEC-008. ExecPlan: `.agent/execplans/EP-040-testing-hardening-and-chaos.md`.

## EP-041: Microbrain Training Factory

Purpose: Implement the separate Microbrain dataset, frozen evals, teacher consensus, QLoRA pipeline, GGUF export, shadow comparison, and canary tooling.

Dependencies: EP-040.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-041` exists.

Specifications: SPEC-025. ExecPlan: `.agent/execplans/EP-041-microbrain-training-factory.md`.

## EP-042: Deployment Release Update And Rollback

Purpose: Implement signed releases, installers, offline bundle, transactional updates, staged rollout, backup-before-update, provider migration, and rollback drills.

Dependencies: EP-041.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-042` exists.

Specifications: SPEC-016, SPEC-024. ExecPlan: `.agent/execplans/EP-042-deployment-release-update-and-rollback.md`.

## EP-043: Production Readiness And Ship

Purpose: Execute all live-fire proofs, security and privacy review, load and hardware certification, restore and rollback drills, docs audit, release tag, and manual deploy handoff.

Dependencies: EP-042.

Exit: the node verify command passes, expected files match, live-fire proofs active at this stage pass, a NODE_DONE event is appended, and tag `green/EP-043` exists.

Specifications: SPEC-008. ExecPlan: `.agent/execplans/EP-043-production-readiness-and-ship.md`.
