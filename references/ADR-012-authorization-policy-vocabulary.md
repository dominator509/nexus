# ADR-012 - Authorization Policy Vocabulary

Status: Accepted
Date: 2026-08-13
Owner: hermes-nexus-main

## Context

EP-008 owns authorization policy and the action gateway: OpenFGA
relationships, OPA contextual policy, risk classes, short-lived grants,
deterministic action gateway, verification, and receipts (SPEC-005,
SPEC-006). The node contract lists eight public interfaces
(`RelationshipAuthorizer`, `ContextPolicyEngine`, `RiskClassifier`,
`ActionGateway`, `CapabilityGrant`, `ApprovalAssertion`, `ActionReceipt`,
`VerificationPlan`) owned by the Rust crates `crates/nexus-policy`
(contracts) and `crates/nexus-action-gateway` (deterministic engine).
SPEC-005 and SPEC-006 "Canonical terms" lock `Relationship Tuple`,
`Policy Decision`, `Approval Assertion`, `ActionRequest`,
`ActionDecision`, `ActionReceipt`, `ExpectedState`, `VerificationResult`,
`RiskClass`, `ApprovalClass`, and `Capability Token`; the existing
nexus-domain vocabulary already carries `Risk` (R0..R4),
`ApprovalClass`, `CapabilityClass`, `Reversal`, `Idempotency`, and
`PrincipalType`. EP-005 M1 doctrine requires every new public name to
come from an accepted vocabulary or be added by an ADR and a schema
update in the same milestone.

## Decision

Add the following vocabulary-locked classes, owned by
`crates/nexus-policy` and documented in `docs/vocabulary/README.md`:

- `ActionLifecycleState` (SPEC-006 behavior 4): `REQUESTED`,
  `EVALUATED`, `AWAITING_APPROVAL`, `APPROVED`, `EXECUTING`,
  `VERIFYING`, `SUCCEEDED`, `FAILED`, `COMPENSATING`, `COMPENSATED`,
  `REJECTED`. Every consequential action moves through this
  deterministic lifecycle; gateway decisions and receipts reference the
  state at each boundary.
- `GrantState` (capability grant lifecycle): `ACTIVE`, `REVOKED`,
  `EXPIRED`. Grants never outlive expiry and never widen scope
  (SPEC-005 behavior 5).
- `ApprovalDecision`: `APPROVED`, `REJECTED`.
- `ReceiptState`: `ISSUED`, `SUPERSEDED`.
- `DenialReason` (SPEC-006 error classes at the gateway boundary):
  `RELATIONSHIP`, `POLICY`, `INSUFFICIENT_STRENGTH`, `NO_CAPABILITY`,
  `MISSING_APPROVAL`, `VERIFICATION_FAILED`.

The canonical term `RiskClass` is implemented by the existing
nexus-domain `Risk` (R0..R4); `ApprovalClass`, `CapabilityClass`,
`Reversal`, and `PrincipalType` are reused from nexus-domain without
synonyms. New struct types (`RelationshipTuple`, `PolicyInput`,
`PolicyDecision`, `CapabilityGrant`, `ApprovalAssertion`,
`ActionRequest`, `ActionDecision`, `ActionReceipt`, `VerificationPlan`,
`ExpectedState`, `VerificationResult`) are interface records, not
vocabulary classes; their field names are camelCase wire-stable via
serde.

## Consequences

- The policy crate owns the deterministic risk floor
  (`deterministic_risk_floor`): R3/R4 require step-up or explicit
  preauthorization (SPEC-005 behavior 4); R4 never accepts model
  approval.
- Fail-closed: relationship, policy, capability, approval, and
  verification denials are explicit decisions; provider failures
  surface as `PolicyError` with SPEC-006 codes, never as grants.
- New synonyms or lifecycle states require an ADR + vocabulary update,
  mirroring ADR-011 for the auth node.
