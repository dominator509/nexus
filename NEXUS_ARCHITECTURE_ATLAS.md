# NEXUS ARCHITECTURE ATLAS

## One logical brain

Nexus is logically centralized and physically distributed. The assistant identity, world model, memory, objectives, policies, capabilities, events, and audit history are canonical in the control plane. Specialized compute, home edge, mobile devices, desktop agents, security appliances, and future robots are registered nodes that execute bounded capabilities.

## Brain composition

The brain is not an LLM. It is the combination of:

1. Identity and presence evidence.
2. Universal World Model.
3. Memory Fabric.
4. Context Engine.
5. Objective and task graph.
6. Capability Registry.
7. Model Router.
8. Agent Orchestrator.
9. Temporal workflow layer.
10. NATS JetStream event nervous system.
11. OpenFGA and OPA policy.
12. Deterministic Action Gateway.
13. Artifact, skill, and connector registries.
14. Observability, Sentinel, and self-healing loops.

## Physical topology

- Cloud or VPS: durable control plane, PostgreSQL, NATS, Temporal, Keycloak, OpenFGA, OPA, OpenBao, dashboard, model gateway, agent coordination, and public callback ingress.
- Home edge: Home Assistant, local fast path, Frigate, go2rtc, voice pipeline, Bluetooth, local policy cache, cameras, device protocols, and offline queue.
- User devices: mobile and desktop interaction, passkeys, biometrics, approvals, local audio, notifications, camera input, and optional compute.
- Specialist nodes: desktop GPU, NAS, development workstations, security sensors, or future robots registered through the Compute Fabric.

## Authority and intelligence

The router selects intelligence. The Action Gateway grants authority. No model response, agent output, voice match, camera label, or social message directly executes a consequential action. Each command is parsed into a typed request, authorized, governed, executed through a scoped connector, verified against observed state, audited, and compensated or rolled back when supported.

## Local-first ladder

1. Deterministic code.
2. Local open-source engine.
3. User-owned remote node.
4. Primary paid API.
5. Secondary paid API.
6. Human decision.

A capability may skip a rung only when latency, reliability, hardware, legal, or quality evidence justifies it.

## DeepSeek and Microbrain

DeepSeek V4 Flash is the V1 ReflexProvider. Stable prefix segments and canonical serialization target at least 0.97 token cache hit ratio on cacheable traffic. The ReflexProvider returns one `NexusControlObject`. Microbrain implements the same contract later and is promoted only after frozen eval, shadow, adversarial, and canary gates.

## Connector fabric

Every important external subsystem exposes authenticated MCP or REST, typed capabilities, idempotent commands, correlation IDs, events or change cursors, and health. Systems lacking those features are wrapped by the Connector Sidecar. Agents request capabilities, never vendor names.

## Commercial boundary

Nexus owns the integration and experience layer. Mature open-source engines remain replaceable sidecars or appliances. This minimizes code, cost, and supply-chain surface while preserving a coherent product and commercial license boundary.
