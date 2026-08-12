# SPEC-010 - Objectives, Agents, Harness Adapters, Artifacts, and Skills

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define the central objective graph, capability-based agent selection, Codex and Claude cowork, Hermes and OpenClaw adapters, artifact exchange, and skills.

## Canonical terms

Agent Registry, Agent Adapter, Agent Capability, Objective, AgentTask, Delegation, Artifact, Agent Skills, Skill Trust, Skill Factory. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Nexus owns canonical objectives, task state, context, permissions, budgets, artifacts, and results.
2. Agents request capabilities rather than named agents. Nexus selects based on quality, cost, trust, availability, and historical success.
3. Direct agent-to-agent authority is forbidden; A2A communication passes through Nexus policy and correlation.
4. Codex and Claude Code adapters support start, message, progress, input request, pause, cancel, resume, artifacts, tests, and review semantics where the harness permits.
5. Hermes and OpenClaw retain temporary scratch state; durable knowledge is proposed back to Nexus memory.
6. Agent Skills packages follow the open format with Nexus metadata, signatures, permissions, network rules, tests, license, provenance, and trust tier.
7. Community skills begin inspect-only or sandboxed and cannot request undeclared permissions at runtime.
8. Skill Factory creates candidates from successful work, tests them against frozen evals, requests human promotion, and retains rollback versions.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Untracked agent conversations
- Agent-owned canonical memory
- Blind skill installation
- Hard-coded Codex-only workflows

## Required tests

- Agent adapter conformance
- Codex-implement and Claude-review live-fire
- Budget and cancellation
- Artifact lineage
- Skill signature tamper
- Permission escalation denial
- Skill rollback

## Acceptance

A multi-agent objective can survive restarts, enforce least privilege, exchange artifacts, resolve review loops, and return one audited result.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
