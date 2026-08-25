# ADR-029 - Production Readiness and Ship Vocabulary

Status: Accepted
Date: 2026-08-25
Owner: EP-043 (Production Readiness and Ship)

## Context

SPEC-008 defines the final ship standard: core release requires every core
profile proof and all mandatory security, data, workflow, installation,
update, backup, and rollback gates; optional providers require real
credentials and observable external effects before UI certification;
hardware classes require model, firmware, transport, capability, latency,
privacy, and failure evidence; final ship uses a fresh-clone-equivalent
environment; restore, rollback, provider failover, identity recovery,
Sentinel containment, and update failure drills have dated evidence; no
critical vulnerability, unreviewed license, missing SBOM, stale backup, or
failed required proof may be accepted by a generic waiver; production
deployment remains a manual command; release notes distinguish
implemented, certified, experimental, unavailable, and deferred
capabilities.

The EP-043 node contract names four public interfaces: ShipGate,
ReleaseEvidence, ManualDeployHandoff, and ProductionReadinessDecision.
None of these vocabulary classes existed in `crates/nexus-domain` or the
EP-042 release crate. EP-043 owns the ship certification boundary and must
encode the authority distinctions SPEC-008 requires: a release candidate
is not a core release; a gate passed is not a release shipped; evidence
existing is not a release signed; an evidence index is not dated drill
evidence; a decision made is not a release shipped; a handoff existing is
not a deployment executed; and a generic waiver never clears a blocking
item.

## Decision

EP-043 defines a provider-neutral, versioned contract package
`release-evidence/` (@nexus/release-evidence) with schema_version 1 and
deny-unknown wire parsing for every interface and vocabulary.

Vocabulary classes (all deny-unknown):

- `CapabilityStatus`: IMPLEMENTED, CERTIFIED, EXPERIMENTAL, UNAVAILABLE,
  DEFERRED (SPEC-008 release-note distinction).
- `ReleaseKind`: RELEASE_CANDIDATE, CORE_RELEASE.
- `GateVerdict`: PENDING, BLOCKED, PASSED (PASSED != SHIPPED).
- `ProofStatus`: NOT_RUN, PASS, FAIL, BLOCKED (passing requires a real
  evidence reference).
- `DrillStatus`: NOT_RUN, DATED_EVIDENCE, FAILED (dated evidence is
  required for every drill kind).
- `CertificationRowState`: PENDING, SIGNED, RELEASE-BLOCKING-PENDING
  (mirrors scripts/certification_validate.py).
- `ShipPhase`: PRE_SHIP, FRESH_CLONE_VERIFY, PRODUCTION_READINESS,
  LIVE_FIRE, SHIP_DECISION, MANUAL_DEPLOY_HANDOFF.
- `WaiverClass`: NONE, ACCEPTED_RISK, GENERIC (GENERIC never clears a
  block; ACCEPTED_RISK requires a dated decision).
- `DrillKind`: RESTORE, ROLLBACK, PROVIDER_FAILOVER, IDENTITY_RECOVERY,
  SENTINEL_CONTAINMENT, UPDATE_FAILURE.
- `ReviewDomain`: SECURITY, PRIVACY, PERFORMANCE, ACCESSIBILITY,
  OBSERVABILITY, BACKUP, RESTORE, UPDATE, ROLLBACK.
- `RequiredGateFamily`: SECURITY, DATA, WORKFLOW, INSTALLATION, UPDATE,
  BACKUP, ROLLBACK.

Authority semantics encoded:

- `ShipGate.verdict` is computed, never trusted from input; a declared
  verdict that differs from the deterministic evaluation fails parsing.
- `ReleaseEvidence.evidenceDigest` is a real sha256 over the canonical
  payload; tampering fails parsing.
- `ProductionReadinessDecision.decision` is computed from gate, evidence,
  and handoff; READY requires gate PASSED + fresh-clone rerun + all
  certification rows SIGNED + all reviews PASS + all drills dated + a
  non-empty exact manual command.
- `ManualDeployHandoff.exactCommand` must be a single command; compound
  commands and embedded secrets are denied.

## Consequences

Ship certification is provider-neutral and versioned; the four public
interfaces can be composed by downstream ship tooling without coupling to
a deployment vendor. Deny-unknown parsing fails closed on unknown wire
values, preventing vocabulary drift. The evidence digest gives a
deterministic tamper check over the evidence index.

## Reversal

Reversing this ADR requires a new ADR and schema update; any new synonym
for a locked name is forbidden without that update.

## Security and license impact

No new third-party dependency; the package imports only TypeScript
standard library and vitest (dev). Evidence redaction scrubs secret-shaped
content before it can enter logs or artifacts.
