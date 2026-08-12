# ADR-009 - Events, Outbox, and Consumer Vocabulary

Status: Accepted
Date: 2026-08-12
Owner: hermes-nexus-main

## Context

EP-005 owns the event nervous system: NATS JetStream, canonical events,
outbox, replay, correlation, and durable consumers (SPEC-023). The node
contract lists seven public interfaces. SPEC-023's "Canonical terms"
section names `EventEnvelope`, `Outbox`, `JetStream`, `DurableConsumer`,
`Workflow`, `Activity`, `Signal`, `Query`, `Schedule`, `ApprovalWorkflow`,
`Compensation` as vocabulary locked. `EventId` and `CorrelationId` already
exist in `docs/vocabulary/README.md` (typed IDs from SPEC-022/SPEC-003);
the remaining names do not. SPEC-023 and the EP-005 milestone doctrine
require every new public name to come from an accepted vocabulary or be
added by an ADR and a schema update in the same milestone.

## Decision

Add the following vocabulary-locked contracts, owned by
`crates/nexus-events` and documented in `docs/vocabulary/README.md`:

- `EventEnvelope`: the canonical event wire model (SPEC-023 behavior 3).
  Carries event ID, type, schema version, source, subject, time, tenant,
  actor reference, correlation, causation, data class, and payload.
- `EventType`: a dotted lowercase slug (e.g. `memory.record.created`).
  New types are added by ADR + schema update, never invented at runtime.
- `EventDataClass`: `PUBLIC`, `HOUSEHOLD`, `PERSONAL`, `SENSITIVE`,
  `BUSINESS_CONFIDENTIAL`, `SECURITY`, `SECRET` - the same privacy ladder
  as memory `Sensitivity` (SPEC-020), so event filtering and redaction
  reuse the same policy classes (INV-014).
- `Outbox`: the transactional outbox. State changes and outbox records
  commit in one PostgreSQL transaction (SPEC-023 behavior 1); JetStream
  publish acknowledgement precedes outbox completion (behavior 2).
- `OutboxStatus`: `PENDING`, `PUBLISHING`, `PUBLISHED`, `FAILED`.
- `Inbox`: the consumer-side deduplication ledger. Consumers are
  idempotent and maintain durable checkpoints; replay does not create
  duplicate logical effects (SPEC-023 behavior 4).
- `InboxStatus`: `NEW`, `PROCESSING`, `DONE`, `FAILED`.
- `DurableConsumer`: a consumer with a durable checkpoint, able to resume
  after restart (SPEC-023 behavior 4). `ConsumerCheckpoint` carries
  consumer, stream, subject, and last sequence.
- `StreamProvisioner`/`StreamConfig`: one canonical stream and subject
  namespace before stream sharding (EP-005 fallback doctrine).

`Workflow`, `Activity`, `Signal`, `Query`, `Schedule`, `ApprovalWorkflow`,
and `Compensation` are Temporal-owned (SPEC-023 behaviors 5-7) and are
added by the workflow node, not EP-005.

## Consequence

The event wire model is closed (additionalProperties: false) and
parse-time rejects unknown event types and data classes. The schema
`schemas/event-envelope.schema.json` is created in EP-005 M3 and locked
to this vocabulary. Providers (NATS JetStream) implement the ports in
`infra/nats`; the contracts remain provider-neutral.

## Alternatives

- Free-form event type strings without a slug contract: rejected, loses
  parse-time rejection and enables drift.
- Reusing memory `Sensitivity` for `EventDataClass`: considered; the
  ladder is identical, but the event contract names its own class so the
  event schema is self-contained (SPEC-023 lists data class as an event
  field, not a memory import).

## Security and compatibility

No secrets in event logs; `data_class` drives redaction. The vocabulary
is additive; existing typed IDs (`EventId`, `CorrelationId`) are reused,
not duplicated.
