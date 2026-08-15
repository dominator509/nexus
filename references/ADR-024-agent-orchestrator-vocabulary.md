# ADR-024 - Agent Orchestrator Vocabulary

Status: Accepted
Date: 2026-08-15
Owner: EP-017 (Agent Orchestrator and Harness Adapters)

## Context

SPEC-010 defines the central objective graph, capability-based agent
selection, Codex and Claude cowork, Hermes and OpenClaw adapters,
artifact exchange, and skills. Its canonical terms are vocabulary
locked: Agent Registry, Agent Adapter, Agent Capability, Objective,
AgentTask, Delegation, Artifact, Agent Skills, Skill Trust, Skill
Factory.

EP-017 owns the agent orchestrator contracts. Several needed vocabulary
classes did not exist: the agent task lifecycle, the harness adapter
kinds, the capability set, the delegation lifecycle, and the budget
classes. Per the vocabulary rule, new public names require an ADR and a
schema/vocabulary update in the same milestone.

## Decision

Add the following EP-017-owned vocabulary in `crates/nexus-agents`
(vocabulary module), documented in `docs/vocabulary/README.md`, with
unknown-value rejection at parse time:

- `AgentTaskState`: `REQUESTED`, `ASSIGNED`, `RUNNING`, `PAUSED`,
  `WAITING_INPUT`, `REVIEWING`, `CANCELLED`, `SUCCEEDED`, `FAILED`.
  Mirrors SPEC-006 ActionLifecycle terminal outcomes; terminal states
  are final.
- `AgentAdapterKind`: `CODEX`, `CLAUDE_CODE`, `HERMES`, `OPENCLAW`.
  Vocabulary-locked adapter identity; concrete implementations live in
  the EP-017 M2 crate boundary.
- `AgentCapability`: `ORCHESTRATE`, `IMPLEMENT`, `REVIEW`, `TEST`,
  `EXECUTE`, `SUMMARIZE`, `ARTIFACT`. Capability-based selection
  (SPEC-010 behavior 2): agents request capabilities, never named
  peers.
- `DelegationState`: `PROPOSED`, `ACCEPTED`, `ACTIVE`, `COMPLETED`,
  `REVOKED`, `FAILED`. Delegation is recorded by Nexus; direct
  agent-to-agent authority is forbidden (SPEC-010 behavior 3).
- `AgentBudgetClass`: `TOTAL_TOKENS`, `TOTAL_COST`, `MAX_CONCURRENT`,
  `MAX_DURATION_SECS`. Fixed declared limits Nexus enforces fail-closed.

Canonical terms from earlier nodes are reused, never redefined:
nexus-domain typed ids (ObjectiveId, TaskId, CorrelationId,
ArtifactId, CapabilityId), nexus-fabric Agent Card vocabulary
(`AgentCard`, `AgentCardId`, `AgentCardState`), Artifact Manifest
(`ArtifactManifest`, `ArtifactState`), A2A task vocabulary (`A2ATask`,
`A2ATaskState`, `A2ATaskStatus`, `TaskMessage`), and SPEC-006 errors
(`FabricError`/`FabricErrorCode`; EP-017 wraps them in `AgentsError`
with the same codes).

## Alternatives

- Reuse the generic SPEC-006 ActionLifecycle for task state (rejected:
  agent tasks have distinct durable states WAITING_INPUT and REVIEWING
  that ActionLifecycle does not express).
- Let adapters declare free-form capability strings (rejected:
  capability-based selection requires a locked set to select
  deterministically and to deny undeclared capabilities).
- Reuse `AgentCard.capabilities` as the capability vocabulary
  (rejected: card metadata strings are discovery hints; the request
  contract needs a typed, vocabulary-locked capability set).

## Consequence

EP-017 contracts have a stable, locked vocabulary; unknown values fail
closed at parse time; capability selection is deterministic; agent
delegation is auditable through Nexus.

## Reversal

Revert the EP-017 M1 commit and remove the vocabulary sections from
`docs/vocabulary/README.md`.

## Compatibility

Additive. No existing surface changes; nexus-fabric and nexus-domain
vocabulary is re-exported unchanged.
