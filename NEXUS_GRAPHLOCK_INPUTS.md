# NEXUS GRAPHLOCK INPUTS

## PROJECT_NAME

Nexus

## PROJECT_DESCRIPTION

A self-hosted-first personal and business intelligence operating system that presents one logical AI brain across cloud, home edge, desktops, mobile devices, networks, businesses, agents, communications, smart-home devices, security systems, and future robotics. Nexus assembles mature open-source systems behind stable provider contracts and adds the proprietary control plane, shared world model, memory fabric, policy, orchestration, user experience, onboarding, observability, and lifecycle automation that make the components behave as one product.

## PRODUCT_GOAL

Deliver a commercially sellable, secure, modular, one-package Life and Business OS that can be deployed to a user-owned VPS and home edge through an extremely simple wizard; defaults to local and open-source execution; minimizes recurring API costs; supports paid API fallbacks without architectural lock-in; safely coordinates people, models, agents, devices, software, communications, businesses, and infrastructure; and remains evolvable as AI and robotics capabilities change.

## TARGET_USERS

Primary: technically ambitious individuals, households, founders, multi-business owners, professionals, and small teams seeking one private control plane for home and work. Secondary: managed-service installers, privacy-conscious families, SMB operators, consultants, developers, and eventually enterprise or regulated deployments using stricter profiles.

## CORE_USER_OUTCOMES

1. LF-001 one-package-deployment: Deploy Nexus Core and a home edge from Nexus Setup using the local provider profile; assert owner login, health, private mesh, and fleet registration.
2. LF-002 restore-existing-nexus: Restore encrypted state onto a fresh deployment and prove identities, policies, memories, skills, and connectors reattach.
3. LF-003 owner-passkey-onboarding: Create an owner, enroll a passkey and recovery material, sign in, revoke the session, and verify audit records.
4. LF-004 multi-user-identity: Enroll two adults and one restricted user; prove separate context, permissions, preferences, and mobile devices.
5. LF-005 cross-device-continuity: Start an objective by voice, continue in the web dashboard, approve on mobile, and receive the final artifact in the same task graph.
6. LF-006 deterministic-home-control: Issue a known low-risk command; prove no model call occurred, Home Assistant changed state, Nexus verified state, and an audit event exists.
7. LF-007 conditional-home-workflow: Create a time and occupancy conditional command; prove Temporal persistence and correct execution or cancellation.
8. LF-008 visitor-response: Receive a camera person event, identify known or unknown, notify the right user, and play an approved response through two-way audio where certified.
9. LF-009 sentinel-quarantine: Detect a synthetic but real network-lab scan from an unknown device, correlate telemetry, request or apply policy-authorized quarantine, and verify isolation.
10. LF-010 network-diagnosis: Diagnose a controlled DNS or Wi-Fi fault from OPNsense or OpenWrt and AdGuard telemetry, explain evidence, and propose a reversible fix.
11. LF-011 email-lifecycle: Receive, search, summarize, draft, approve, send, and verify a real message through a certified mail provider.
12. LF-012 governed-phone-call: Place a real test call through Asterisk and a certified SIP provider, exchange speech with STT and TTS, honor disclosure, and store the governed transcript.
13. LF-013 fax-lifecycle: Send a real test fax through the certified profile, receive status callbacks, route inbound fax, and archive the artifact.
14. LF-014 social-campaign: Create platform-native variants, obtain approval, publish through a certified account, ingest engagement, and report attribution.
15. LF-015 hydra-cross-crm-command: Ask for hot leads across businesses, receive canonical Hydra context, propose a governed update, execute it, and consume the resulting Hydra event.
16. LF-016 coding-agent-cowork: Assign implementation to Codex, independent review to Claude Code, return an issue for correction, run tests, and produce a human-approved pull request artifact.
17. LF-017 durable-human-approval: Start a workflow, restart the worker while waiting, approve later from mobile, and prove exactly-once continuation.
18. LF-018 skill-install-and-run: Inspect, scan, approve, sign, install, discover, execute, and roll back a skill without granting undeclared capabilities.
19. LF-019 self-healing-fix-loop: Trigger a controlled defect, detect it through telemetry, reproduce, patch, test, review, request approval, canary, verify, and close or roll back.
20. LF-020 storage-backend-portability: Write versioned artifacts, migrate between local and one S3-compatible backend, verify hashes and metadata, and remove the old copy only after approval.
21. LF-021 model-provider-failover: Return a valid NexusControlObject through DeepSeek, disable the primary provider, fail over to a configured secondary, and preserve schemas, budgets, and trace IDs.
22. LF-022 mobile-step-up: Request a high-risk action by voice, refuse voice-only authorization, approve with mobile biometric and passkey, execute, and verify.
23. LF-023 legacy-sidecar-connector: Wrap a real local legacy protocol fixture outside production paths, discover capabilities, read state, issue an idempotent write, and receive a change event.
24. LF-024 offline-degraded-operation: Disconnect cloud AI and public internet while retaining local identity cache, low-risk home control, alerts, and queued synchronization.
25. LF-025 ceo-business-brief: Combine Hydra, social, communications, and finance connector data into a permission-filtered executive brief with source provenance.
26. LF-026 voice-endpoint-transfer: Start a conversation on a room satellite, move it to a Bluetooth headset or mobile endpoint, and maintain user, task, and privacy context.
27. LF-027 social-lead-to-crm: Classify a real certified social inquiry, create or link the canonical Hydra person and lead, draft a response, and record attribution.
28. LF-028 shared-room-private-response: Ask for sensitive personal information in an occupied room and prove Nexus routes the response privately rather than speaking it aloud.

## REPOSITORY_STATUS

Greenfield

## FRONTEND_STACK_OR_UNKNOWN

TypeScript 5.x, React 19.2.8, Vite, accessible component primitives, PWA; Tauri 2.11.2 desktop shell; Flutter 3.44.7 mobile applications with native Swift and Kotlin modules for device-security features.

## BACKEND_STACK_OR_UNKNOWN

Rust 1.97.1 for the control plane, edge runtime, security-critical services, connector runtime, APIs, eventing, and CLIs; TypeScript on Node 24 LTS for Temporal workers and web code; Python 3.14.6 with uv for speech, ML evaluation, Microbrain training, and model-specific workers.

## DATABASE_OR_UNKNOWN

PostgreSQL 18.4 with pgvector 0.8.6. The WorldGraphRepository interface begins with a PostgreSQL implementation and permits a dedicated graph engine later without changing domain callers.

## AUTH_OR_UNKNOWN

Keycloak 26.7.0 for OIDC, OAuth2, passkeys, federation, and service identities; OpenFGA 1.18.1 for relationship authorization; OPA 1.16.2 for contextual policy; device-bound credentials, mTLS, and step-up authentication for consequential actions.

## DEPLOYMENT_OR_UNKNOWN

OCI containers and Docker Compose for the default single-node and hybrid profiles; OpenTofu 1.12.1 plus cloud-init for BYOC provisioning; Headscale 0.28.0, WireGuard, and mTLS for the private mesh; optional enterprise Kubernetes target after the default deployment is proven.

## TESTING_TOOLS_OR_UNKNOWN

cargo test, nextest where justified, proptest, insta snapshots, Playwright, Vitest, Flutter test and integration_test, pytest, contract tests, Testcontainers, A2A TCK, MCP conformance, security scans, chaos tests, hardware certification tests, and stage-aware real live-fire proofs.

## PACKAGE_MANAGER_OR_UNKNOWN

Cargo with Cargo.lock; pnpm 11.17.0 with pnpm-lock.yaml; uv 0.12.0 with uv.lock; Flutter pub with pubspec.lock.

## CICD_OR_UNKNOWN

GitHub Actions with pinned action commit SHAs, separate required CI and informational nightly workflows, signed OCI artifacts, SBOMs, attestations, staged releases, human production promotion because auto-deploy is not authorized.

## OBSERVABILITY_OR_UNKNOWN

OpenTelemetry Collector 0.158.0, OpenTelemetry SDKs, GlitchTip 6.1.8, Prometheus-compatible metrics, Grafana dashboards, Loki-compatible logs, Tempo-compatible traces, and Nexus incident correlation.

## EXTERNAL_SERVICES_AND_CREDENTIALS

DeepSeek V4 Flash is the required V1 reflex provider. Optional providers include OpenAI, Anthropic, Google, xAI, Venice, ElevenLabs, Deepgram, Azure Speech, Telnyx, Twilio, Phaxio, Gmail, Microsoft Graph, Cloudflare, Contabo, Hetzner, DigitalOcean, AWS, social platforms, GitHub, app stores, and optional external storage backends. PREFLIGHT.md enumerates every credential and one-of group.

## AGENT_PLATFORMS

Claude Code, Codex CLI, Hermes, OpenClaw, GitHub Copilot, Cursor, Cline, and any terminal agent that can read, edit, and run commands.

## AUTO_DEPLOY_AUTHORIZED

no

## BUSINESS_CONSTRAINTS

Self-hosted first; open-source first; lowest practical recurring cost; mature existing software before custom implementation; commercially redistributable architecture; copyleft components isolated and compliance-reviewed; one installer and one coherent UX; managed SaaS may be offered without making self-hosting second class.

## TECHNICAL_CONSTRAINTS

One logical brain with physically distributed nodes; provider and connector contracts instead of vendor coupling; no direct model authority over consequential actions; no mandatory local LLM; DeepSeek V4 Flash V1 reflex path with a future Microbrain drop-in; Rust-first control plane; PostgreSQL-first; NATS JetStream; Temporal; Home Assistant; Frigate; Asterisk; identity, policy, secrets, observability, and deployment components named in COMPONENT_REGISTRY.yaml.

## SECURITY_COMPLIANCE_CONSTRAINTS

Zero-trust service identity; least privilege; tenant, person, business, and household isolation; passkey and biometric step-up; voice is identity evidence only; signed skills and connectors; software bill of materials; license policy; encrypted backups; append-only audit; no default TLS interception; jurisdiction-aware call and recording policy; no claim of HIPAA, PCI DSS, SOC 2, or other certification until the corresponding profile is independently validated.

## PERFORMANCE_REQUIREMENTS

Known local commands target sub-second perceived response on the home edge; ordinary interactive AI targets p50 time-to-first-token below 1.5 seconds and p95 below 3 seconds when provider conditions allow; API reads target p95 below 250 ms within a region; event propagation target p95 below 500 ms; action-gateway decisions target p99 below 20 ms excluding external checks; no household command waits behind long model generation; DeepSeek reflex cache token-hit SLO is at least 0.97 on cacheable traffic.

## ACCESSIBILITY_REQUIREMENTS

WCAG 2.2 AA for web and desktop; native platform accessibility semantics for mobile; keyboard-only operation; focus visibility; reduced-motion support; screen-reader labels; captions and transcripts for voice; non-color-only status; large-text resilience; no critical flow dependent only on speech.

## DATA_PRIVACY_REQUIREMENTS

Local processing by default; explicit data classification; purpose-limited context capsules; per-user and per-business namespaces; memory provenance, confidence, retention, supersession, and deletion; encryption in transit and at rest; export and deletion workflows; private response routing in shared spaces; API egress disclosed and policy-controlled; no training on user content without opt-in and scrubbing.

## THIRD_PARTY_INTEGRATIONS

Home Assistant, Frigate, go2rtc, Roku Home provider, Sonos and media providers, OPNsense, OpenWrt, AdGuard Home, Suricata, Zeek, CrowdSec, Wazuh or osquery, Asterisk, ICTFax and HylaFAX, Gmail, Microsoft Graph, IMAP and SMTP, Postiz, Hydra, Codex, Claude Code, Hermes, OpenClaw, MCP, A2A, Agent Skills, S3-compatible storage, MinIO compatibility, Cloudflare R2, Backblaze B2, Amazon S3, NAS, cloud provisioners, and future robots through the universal connector contract.

## KNOWN_NON_GOALS

Training a foundation model from scratch; granting models unrestricted physical or financial authority; replacing mature open-source projects merely to own more code; requiring Kubernetes for a household; requiring Nexus-operated cloud services for self-hosted installations; bypassing vendor authentication, DRM, platform terms, or device secure boot; promising universal Roku local streaming before verified; shipping noncommercial wake-word model weights; certifying every optional provider without real live-fire evidence; building a robotic body in V1.

## TIMELINE_OR_UNKNOWN

No calendar commitment is assumed. The deterministic 44-node graph is the milestone schedule. Each node ends in evidence, a green tag, or a terminal blocked report.

## DEPLOYMENT_TARGET_OR_UNKNOWN

Reference production profile: user-owned VPS plus home-edge node, with web, desktop, iOS, and Android clients. Supported profiles: managed cloud, BYOC, existing SSH server, hybrid self-hosted, and fully local. Provider adapters target Contabo, Hetzner, DigitalOcean, AWS, and generic SSH first.

## RUNTIME_BUDGET_NOTES

Default milestone maximum is six attempts under the Graphlock ladder. Control-plane services must fit a practical 4 to 8 vCPU and 16 GB RAM VPS without local generative inference. Home-edge sizing is profiled by workload. Optional vision and speech acceleration use available GPU, NPU, Coral, or desktop compute. Every worker declares CPU, RAM, storage, latency, locality, and trust constraints.

## SPECIAL_INSTRUCTIONS

The onboarding wizard, self-hosted defaults, open-source reuse, commercial license isolation, provider fallbacks, DeepSeek reflex contract, Microbrain seam, multi-user security, Nexus Connector Contract, stage-aware reality proofs, and the complete locked component matrix are architectural invariants. Modify the original Graphlock graph from 11 to 44 bounded nodes. Core-release and provider-certification gates are distinct; an optional connector is never advertised as operational before real live-fire certification.
