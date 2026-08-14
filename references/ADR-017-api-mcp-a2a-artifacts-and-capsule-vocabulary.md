# ADR-017 - API, MCP, A2A, Artifact, and Context Capsule Vocabulary

Status: Accepted
Date: 2026-08-14
Owner: hermes-nexus-main

## Context

EP-012 owns the Nexus fabric surface: versioned REST, WebSocket, MCP
Streamable HTTP, A2A, artifact exchange, and scoped context capsules
(node contract `.agent/node-contracts/EP-012.md`), in the Rust crate
`crates/nexus-fabric`. SPEC-003 locks the canonical terms `Nexus
Connector Contract`, `MCP`, `A2A`, `Agent Card`, `Context Capsule`,
`Capability Descriptor`, `Query`, `Command`, `Workflow`, `Stream`,
`Artifact Manifest`, and `Invocation Context`. SPEC-003 requires MCP
specification 2025-11-25 with Streamable HTTP, A2A protocol 1.0.1,
artifacts immutable by hash with lineage, and context capsules that
contain only authorized, task-relevant, cited data and expire after the
task or declared retention. EP-005 M1 doctrine requires every new public
name to come from an accepted vocabulary or be added by an ADR and a
schema update in the same milestone.

## Decision

Add the following vocabulary-locked classes, owned by
`crates/nexus-fabric` and documented in `docs/vocabulary/README.md`:

- `ApiTransport` (SPEC-003): `REST`, `WEBSOCKET`,
  `MCP_STREAMABLE_HTTP`, `A2A` - the fabric transport families.
- `McpProtocolVersion` (SPEC-003 required behavior 2): `2025-11-25` -
  the locked MCP Streamable HTTP specification target; unknown versions
  fail closed.
- `A2AProtocolVersion` (SPEC-003 required behavior 3): `1.0.1` - the
  locked A2A protocol target.
- `StreamState` (SPEC-003 canonical term `Stream`): `PENDING`, `RUNNING`,
  `COMPLETED`, `CANCELLED`, `FAILED` - A2A task stream lifecycle.
- `WebSocketState` (SPEC-003): `CONNECTING`, `OPEN`, `CLOSING`,
  `CLOSED` - WebSocket session lifecycle.
- `McpContentKind` (SPEC-003 required behavior 2 structured content):
  `TEXT`, `IMAGE`, `AUDIO`, `RESOURCE`, `EMBEDDED`.
- `A2ATaskState` (SPEC-003 required behavior 3): `SUBMITTED`,
  `WORKING`, `INPUT_REQUIRED`, `COMPLETED`, `CANCELLED`, `FAILED`.
- `AgentCardState` (SPEC-003 canonical term `Agent Card`):
  `REGISTERED`, `SUSPENDED`, `REVOKED`.
- `ArtifactState` (SPEC-003 canonical term `Artifact Manifest`):
  `SEALED`, `SUPERSEDED`, `REVOKED` - artifacts are immutable by hash;
  revocation supersedes, never mutates content.
- `CapsuleState` (SPEC-003 canonical term `Context Capsule`): `ACTIVE`,
  `EXPIRED`, `REVOKED` - capsules expire after task or declared
  retention.

Every enum parses from its canonical SCREAMING_SNAKE_CASE wire string
and rejects unknown values (fail closed). Ports (`RestApi`,
`WebSocketSession`, `McpServer`, `McpClient`, `A2AGateway`,
`AgentCardRegistry`, `ArtifactExchange`, `ContextCapsuleService`) carry
authenticated tenant and principal context; tenant is never selectable
through untrusted metadata (acceptance obligation 3); models never
grant authority.

## Consequences

- The fabric contract crate depends only on `nexus-domain`,
  `nexus-identity`, `nexus-auth`, and serde (dependency-direction test).
- Provider adapters implement these ports in later milestones; the
  contracts are transport-agnostic and versioned.
- Unknown vocabulary values fail closed at the boundary.

## Reversal

A future ADR plus schema update.
