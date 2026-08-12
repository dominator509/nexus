# SPEC-023 - Events, Outbox, Temporal Workflows, Scheduling, and Human Approvals

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define durable messaging, workflow lifecycle, replay, exactly-once intent, timers, signals, compensation, and approval.

## Canonical terms

EventEnvelope, Outbox, JetStream, DurableConsumer, Workflow, Activity, Signal, Query, Schedule, ApprovalWorkflow, Compensation. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. State changes and outbox records commit in one PostgreSQL transaction.
2. JetStream publish acknowledgement is required before marking an outbox row published.
3. Events have event ID, type, schema version, source, subject, time, tenant, actor reference, correlation, causation, data class, and payload.
4. Consumers are idempotent and maintain durable checkpoints. Replay does not create duplicate logical effects.
5. Temporal owns long-running objectives, external retries, timers, waits, cancellation, approvals, compensation, and scheduled jobs.
6. Workflow code is deterministic and all network or time side effects occur in activities.
7. Human approval signals carry immutable principal, authentication strength, action digest, decision, time, and optional comment.
8. Workflow and event schema upgrades preserve in-flight compatibility.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- NATS as canonical database
- Nested custom retry loops
- Workflow logic in UI
- Approval as a chat message

## Required tests

- Transactional outbox
- Publish failure
- Replay deduplication
- Worker restart during approval
- Workflow versioning
- Timer and cancellation
- Compensation

## Acceptance

A multi-day objective survives process and host restarts, emits traceable events, resumes after approval, and causes each external effect at most once logically.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
