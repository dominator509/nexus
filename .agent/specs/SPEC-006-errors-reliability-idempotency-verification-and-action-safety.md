# SPEC-006 - Errors, Reliability, Idempotency, Verification, and Action Safety

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define typed failures, deterministic retries, action lifecycle, verification, compensation, and fail-closed behavior.

## Canonical terms

NexusError, Problem Details, ActionRequest, ActionDecision, ActionReceipt, ExpectedState, VerificationResult, Compensation, IdempotencyRecord, RiskClass, ApprovalClass. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Every boundary returns a stable machine code and safe human explanation using RFC 9457-compatible problem details for HTTP.
2. Commands require idempotency keys when transport or workflow retries are possible.
3. Reusing a key with the same canonical request returns the original result; conflicting reuse returns a deterministic conflict.
4. Action lifecycle is requested, evaluated, awaiting approval, approved, executing, verifying, succeeded, failed, compensating, compensated, or rejected.
5. External success is not accepted until the verifier reads actual state or an authoritative receipt.
6. R3 and R4 ambiguity fails closed. Lower-risk ambiguity requests clarification or escalates intelligence.
7. Retries are bounded, jittered, classified by error, and owned by Temporal or connector policy rather than nested arbitrary loops.
8. Compensation and rollback are explicit capabilities; irreversible actions disclose that fact before approval.
9. User-facing operations remain responsive through progress events and cancellation.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Catch-all internal errors to clients
- Retry forever
- Assume success after HTTP 200
- Generic undo claim

## Required tests

- Problem-code snapshots
- Idempotency concurrency tests
- Forced provider timeout
- Verification mismatch
- Compensation test
- Cancellation test
- Risk fail-closed property tests

## Acceptance

Every mutation produces one terminal receipt, no duplicate side effect under retry, and observable recovery or blocked evidence.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
