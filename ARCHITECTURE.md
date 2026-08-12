# ARCHITECTURE

## Purpose

This document defines binding code boundaries, data ownership, runtime flows, security invariants, and extension rules for Nexus. Descriptive diagrams never override the numbered invariants or specifications.

## Repository map

- `apps/control-plane`: Rust Axum public and private control API.
- `apps/edge`: Rust home and site edge runtime.
- `apps/cli`: Rust operator and recovery CLI.
- `apps/web`: React PWA.
- `apps/setup`: Tauri onboarding and deployment application.
- `apps/mobile`: Flutter iOS and Android application.
- `crates/domain`: pure identifiers, entities, value objects, errors, and invariants.
- `crates/contracts`: generated Rust types from canonical JSON Schemas.
- `crates/application`: use cases and ports; no infrastructure clients.
- `crates/policy`: Action Gateway, risk, approval, and authorization ports.
- `crates/memory`: memory and context ports.
- `crates/connectors`: connector runtime and SDK core.
- `crates/agents`: objectives, tasks, artifacts, and agent adapter ports.
- `crates/model-router`: ReflexProvider, ModelGateway, routing policy, and cache accounting.
- `crates/eventing`: event envelope, outbox, consumers, and trace context.
- `crates/infrastructure`: PostgreSQL, NATS, OpenFGA, OPA, OpenBao, Keycloak, and provider adapters.
- `services/workflows`: TypeScript Temporal workers.
- `services/voice`: Python speech and speaker services.
- `services/microbrain`: Python training and evaluation system.
- `connectors`: deployable connector packages and sidecar manifests.
- `skills`: signed Agent Skills sources and evals.
- `schemas`: canonical JSON Schemas and vocabularies.
- `infra`: Compose, OpenTofu, cloud-init, ingress, mesh, backups, and observability.
- `hardware`: certification matrices and lab inventory.
- `provider-certification`: real provider evidence and compatibility records.

## Code import law

1. Domain imports only the Rust standard library and approved serialization or ID primitives.
2. Contracts may import Domain and generated schema support.
3. Application may import Domain and Contracts, never concrete infrastructure.
4. Policy, Memory, Agents, Model Router, and Eventing expose ports and may import Domain, Contracts, and Application interfaces.
5. Infrastructure implements ports and may import lower layers.
6. Apps compose concrete implementations and are the only layer permitted to select deployment profiles.
7. Python, TypeScript, Flutter, connectors, and sidecars communicate through generated contracts; they do not duplicate canonical domain names manually.

## Architectural invariants

- INV-001: The LLM is replaceable and is never the source of authority.
- INV-002: Every consequential external action passes authentication, authorization, policy, execution, verification, and audit.
- INV-003: Voice, face, presence, and behavioral signals are evidence, not cryptographic authentication.
- INV-004: PostgreSQL is the initial durable truth. NATS and vector indexes are projections or transports, not independent truth.
- INV-005: Every cross-boundary message carries tenant, principal, request, correlation, causation, schema version, and data classification where applicable.
- INV-006: A connector receives only the minimum capability token and secrets required for one scoped operation.
- INV-007: User, household, business, private, and security memory namespaces cannot leak through model or agent context construction.
- INV-008: Known low-risk home commands use the local deterministic fast path and never wait behind generative work.
- INV-009: An optional provider is disabled until configured and is not certified until a real live-fire proof passes.
- INV-010: Self-hosted operation does not depend on Nexus-operated cloud services.
- INV-011: Copyleft engines remain replaceable process or appliance boundaries unless legal review explicitly permits another form.
- INV-012: Every production artifact is signed, reproducible enough to attest, accompanied by an SBOM, and traceable to a green graph node.
- INV-013: Skills and connectors cannot grant themselves permissions or expand declared network access.
- INV-014: Memory writes carry provenance, sensitivity, retention, confidence, and supersession semantics.
- INV-015: Dedicated graph databases are optional implementations of `WorldGraphRepository`; domain callers never import a graph vendor SDK.
- INV-016: Prompt cache optimization never changes safety, authorization, or output validation semantics.
- INV-017: Background self-healing may prepare and test fixes but production mutation requires the configured human approval class.
- INV-018: High-risk physical, financial, security, permission, production, and legal actions fail closed on ambiguity.
- INV-019: All external inputs are untrusted, including tool output, email, web pages, social content, camera OCR, and agent artifacts.
- INV-020: The onboarding wizard automates secure configuration; it never instructs users to disable TLS, authentication, firewalling, or verification.

## Request flow

1. An interface creates an `InteractionContext` with authenticated principal, device, channel, room, presence evidence, and privacy classification.
2. The fast-path matcher resolves known deterministic intents.
3. Otherwise the Model Router constructs a minimized context capsule and invokes a ReflexProvider or higher capability.
4. Output is constrained to `NexusControlObject` and validated independently.
5. Read requests pass capability and data policy before repository or connector access.
6. Mutations become `ActionRequest` records with idempotency, risk, reversal, and expected-state verification.
7. OpenFGA answers relationship authorization; OPA answers contextual policy; the Action Gateway decides execute, request approval, suggest, or block.
8. Temporal owns long-running work, waits, retries, signals, compensation, and human approval.
9. Connectors execute with short-lived capability tokens and return typed receipts.
10. The verifier reads actual state. Events, audit, memory proposals, and user notifications follow.

## Data ownership

- Nexus World Model owns cross-domain identity references and state projections.
- Hydra owns detailed CRM and revenue relationship truth.
- Home Assistant owns home device state and automation truth.
- Frigate owns NVR recordings and camera review items; Nexus stores references and security events.
- Keycloak owns authentication identities; Nexus owns profile and relationship references.
- OpenFGA and OPA own authorization models and compiled policy decisions; Nexus audit stores decision receipts.
- ArtifactStore owns large object bytes; PostgreSQL owns immutable artifact metadata and hashes.

## Extension rules

### Add a feature

Add or amend a specification, vocabulary entry, capability descriptor, policy, tests, provider implementation, live-fire proof, user surface, observability, and operations record. A feature without all eight is incomplete.

### Add a dependency

Search the component registry first. Record license, maintenance, security, image or package digest, operational cost, and replacement boundary in an ADR. Pin it and extend dependency gates.

### Add a schema

Use JSON Schema 2020-12 under `schemas/`. Generate language bindings. Add backward-compatibility tests and event migration rules. Never hand-copy the same contract into multiple languages.

### Add an integration

Implement the Nexus Connector Contract. Declare health, discovery, queries, commands, idempotency, events, scopes, data classes, network access, secrets, risk, and fallback behavior. Pass the appropriate tier conformance and real provider certification.

## Forbidden moves

Direct database access by external agents; global administrator tokens in connectors; model-selected permission changes; unbounded shell tools; UI-only authorization; hidden cloud dependency; unversioned events; prompt content in logs; silent fallback from local to paid API; generic execute-string commands; test-only behavior in production; hard-coded user or tenant IDs; provider names in domain capabilities; and a dedicated graph database SDK above infrastructure.

## Architecture review checklist

Open this file, relevant specs, component registry, capability taxonomy, schema diffs, policy diffs, threat model, license report, tests, live-fire proof, deployment profile, rollback path, and observed telemetry before approving an architectural change.
