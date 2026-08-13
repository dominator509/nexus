# @nexus/workflows

Provider-neutral durable workflow contracts for Nexus (SPEC-023, ADR-010).

Owned public interfaces (EP-006 node contract):

- `ObjectiveWorkflow` - long-running objectives with milestone approvals.
- `ApprovalWorkflow` - durable human approval bound to an action digest.
- `ConnectorCertificationWorkflow` - connector certification lifecycle.
- `IncidentRemediationWorkflow` - diagnosis, HITL approval, remediation,
  verification, compensation.
- `DeploymentWorkflow` - staged rollout with canary and rollback.
- `WorkflowSignal` - durable, idempotent, immutable signals
  (APPROVAL / CANCEL / RESUME).
- `WorkflowQuery` - deterministic read-only state queries.

## Hard invariants

1. **Determinism** - workflow code never calls `Date.now()`,
   `Math.random()`, network, or database APIs. Time comes from the
   `WorkflowContext` engine clock; every side effect happens in an
   activity with an idempotency key. `src/determinism.ts` audits this.
2. **Replay** - the same history replays identically for the same
   workflow name + version (see `docs/versioning.md`).
3. **Idempotency** - every signal carries a `signalId`; duplicate signals
   collapse on `signalKey()`. Activity retries reuse the same
   idempotency key.
4. **Approval binding** - `ApprovalSignal` binds to the exact action
   digest and carries immutable principal + authentication strength;
   `assertApprovalBinding` enforces the match and the class requirement.
5. **Explicit timeout/cancel** - every workflow declares timeouts and a
   cancel action (`CANCEL` fail-closed or `COMPENSATE` rollback); the
   vocabulary exposes `TIMED_OUT`, `CANCELLED`, `COMPENSATED`.

## Layout

- `src/vocabulary.ts` - locked enums with parse-time rejection.
- `src/ids.ts` - branded UUIDv7 / digest identifiers.
- `src/signals.ts` - durable signals + approval binding.
- `src/activities.ts` - activity contracts + idempotency keys.
- `src/queries.ts` - deterministic queries + typed responses.
- `src/workflows.ts` - `WorkflowSpec` contracts and the five workflows.
- `src/policies.ts` - timeout/cancel/retry policies with validation.
- `src/versioning.ts` + `docs/versioning.md` - evolution strategy.
- `src/determinism.ts` + `src/determinism-rules.json` - determinism audit.

## Engine neutrality

This package never imports a Temporal SDK. `WorkflowContext` is the port;
`infra/temporal` implements it. Integration and live-fire tests live in
`tests/workflows/` and `scripts/live-fire/LF-017.sh`.
