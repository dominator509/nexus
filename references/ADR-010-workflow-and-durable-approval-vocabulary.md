# ADR-010 - Workflow and Durable Approval Vocabulary

Status: Accepted
Date: 2026-08-13
Owner: hermes-nexus-main

## Context

EP-006 owns Temporal durable workflows: namespaces, workers, workflow
contracts, approvals, retries, signals, and cancellation (SPEC-023). The
node contract lists seven public interfaces (`ObjectiveWorkflow`,
`ApprovalWorkflow`, `ConnectorCertificationWorkflow`,
`IncidentRemediationWorkflow`, `DeploymentWorkflow`, `WorkflowSignal`,
`WorkflowQuery`) owned by the TypeScript package `@nexus/workflows`.
SPEC-023's "Canonical terms" names `Workflow`, `Activity`, `Signal`,
`Query`, `Schedule`, `ApprovalWorkflow`, `Compensation` as vocabulary
locked; ADR-009 explicitly deferred these seven names to the workflow
node. SPEC-005 additionally locks `Authentication Strength` and
`Approval Assertion`, which no earlier node has added to
`docs/vocabulary/README.md`. EP-005 M1 doctrine requires every new public
name to come from an accepted vocabulary or be added by an ADR and a
schema update in the same milestone.

## Decision

Add the following vocabulary-locked contracts, owned by
`packages/workflows` (`@nexus/workflows`) and documented in
`docs/vocabulary/README.md`:

- `Workflow`: a durable, deterministic, versioned unit of long-running
  work (SPEC-023 behavior 5). Time and I/O only through the workflow
  context; every side effect lives in an activity (behavior 6).
- `Activity`: the only surface that touches the outside world
  (EXTERNAL_EFFECT, VERIFY, COMPENSATE). Carries an idempotency key and
  a bounded, error-classified retry policy (SPEC-006 behaviors 2, 5, 7,
  8).
- `Signal`: an immutable, durable, idempotent message to a workflow.
  Every signal carries a `signalId` (UUIDv7); duplicate signals collapse
  on the canonical `signalKey` (workflow + type + signalId).
- `Query`: a deterministic, read-only view of workflow state; answers
  derive from the durable history so replay answers identically.
- `Schedule`: Temporal-owned scheduled execution (deferred name, kept in
  vocabulary for later nodes).
- `ApprovalWorkflow`: a durable human-approval gate. An approval is an
  immutable assertion carrying the exact action digest, the signer
  principal, and the authentication strength/context; it binds to the
  exact action payload digest, never to free text (SPEC-023 behavior 7).
- `Compensation`: an explicit rollback capability registered per effect,
  executed in reverse order (SPEC-006 behavior 8).
- `AuthenticationStrength`: `NONE`, `SINGLE_FACTOR`, `MULTI_FACTOR`,
  `STEP_UP`. SPEC-005 behavior 4 requires a cryptographic step-up for R3
  and R4 actions; R4 never accepts model approval.
- `ApprovalAssertion`: the immutable payload of an ApprovalSignal
  (principal, authentication, action digest, decision, decided time,
  optional comment).
- `ActionDigest`: canonical lowercase SHA-256 hex (64 chars) of the exact
  action payload being approved.

Supporting workflow kinds and states owned by this node:

- `WorkflowKind`: `OBJECTIVE`, `APPROVAL`, `CONNECTOR_CERTIFICATION`,
  `INCIDENT_REMEDIATION`, `DEPLOYMENT`.
- `WorkflowState`: `REQUESTED`, `EVALUATED`, `AWAITING_APPROVAL`,
  `APPROVED`, `EXECUTING`, `VERIFYING`, `SUCCEEDED`, `FAILED`,
  `REJECTED`, `COMPENSATING`, `COMPENSATED`, `CANCELLED`, `TIMED_OUT`.
  The action-facing states mirror SPEC-006 ActionLifecycle; `CANCELLED`
  and `TIMED_OUT` are explicit Temporal-owned terminals (EP-006
  acceptance obligation 3).
- `SignalType`: `APPROVAL`, `CANCEL`, `RESUME`. New signal types require
  an ADR.
- `QueryType`: `WORKFLOW_STATUS`, `PENDING_APPROVAL`, `ACTIVITY_STATE`,
  `ACTION_RECEIPT`.
- `ActivityKind`: `EXTERNAL_EFFECT`, `VERIFY`, `COMPENSATE`.
- `CancelAction`: `CANCEL` (fail closed) or `COMPENSATE` (rollback).

## Consequence

`@nexus/workflows` is the provider-neutral workflow contract surface:
it never imports a Temporal SDK. The engine adapter in `infra/temporal`
implements `WorkflowContext` and `execute()` against these contracts
(EP-006 M2); integration tests use the real Temporal test environment
and a real server E2E (EP-006 M3); failure tests exercise cancellation,
timeout, duplicate signals, and compensation (EP-006 M4); LF-017 proves
worker restart + delayed approval resumes exactly once (EP-006 M5).
Versioning follows `packages/workflows/docs/versioning.md` (SPEC-023
behavior 8: in-flight compatibility preserved; breaking changes ship as
new workflow names on isolated task queues).

## Alternatives

- Reuse EP-005 event vocabulary for signals: rejected. Events are
  publish-once, fan-out records; signals are workflow-directed,
  idempotent, and reply on durable history. Separate names keep the two
  contracts honest.
- Free-form approval comments as the binding: rejected. Approvals bind
  to the `actionDigest`, never to human text.
- No explicit `TIMED_OUT`/`CANCELLED` states: rejected. EP-006
  acceptance obligation 3 requires timeout and cancellation semantics to
  be explicit in the vocabulary, not emergent engine behavior.

## Security and compatibility

Approval signals carry immutable principal and authentication strength;
`assertApprovalBinding` enforces the digest match and the strength
requirement at the contract boundary. The vocabulary is additive; a
major change requires a new ADR, a new workflow name, and a drain of
in-flight executions per `docs/versioning.md`. No secrets in workflow
logs; `decidedAt` and `verifiedAt` are signer-set ISO-8601 timestamps,
never engine wall clocks.
