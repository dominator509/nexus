# ADR-026 - Self-Healing Engineering Loop Vocabulary and Authority Semantics

Status: Accepted
Date: 2026-08-16
Owner: EP-019 (Self-Healing Engineering Loop)

## Context

SPEC-018 defines automated detection, investigation, patch preparation,
testing, review, approval, canary, rollback, and learning. The canonical
terms are Incident, Diagnosis, Reproduction, PatchCandidate, Review,
Canary, HealthCriterion, Rollback, SkillCandidate, and IncidentMemory.
None of these vocabulary classes existed in `crates/nexus-domain` or a
healing crate. EP-019 owns the healing contracts and must encode several
authority distinctions the owner directive requires: a model/agent may
propose a diagnosis or patch but can never declare its own fix
successful; the lifecycle is a single canonical path with explicit
terminal/failure states that are never collapsed; approval binds to the
exact patch digest; and the self-healing system cannot install its own
generated skills.

## Decision

Add the EP-019-owned vocabulary in `crates/nexus-healing` (vocabulary
module), documented in `docs/vocabulary/README.md`, with unknown-value
rejection at parse time:

- `IncidentState`: `OBSERVE`, `INCIDENT`, `CORRELATE`, `DIAGNOSE`,
  `REPRODUCE`, `PATCH_PROPOSED`, `SANDBOX_VALIDATION`,
  `SECURITY_VALIDATION`, `APPROVAL`, `STAGED_DEPLOYMENT`,
  `POST_DEPLOY_VERIFICATION`, `CLOSED`, `REJECTED`, `UNREPRODUCIBLE`,
  `VALIDATION_FAILED`, `SECURITY_FAILED`, `ROLLED_BACK`, `BLOCKED`.
  Canonical lifecycle, explicit terminal states, fail closed, no
  resurrection, no state collapse.
- `DiagnosisConfidence`: `HYPOTHESIS`, `SUPPORTED`, `REPRODUCED`,
  `VALIDATED`. A model-generated explanation always begins as
  HYPOTHESIS; only reproducible evidence raises it to VALIDATED.
- `IncidentSignalKind`: `PROCESS_FAILURE`, `HEALTH_FAILURE`,
  `TEST_FAILURE`, `WORKFLOW_FAILURE`, `CONNECTOR_FAILURE`,
  `SECURITY_EVENT`, `RESOURCE_EXHAUSTION`, `DEPLOYMENT_REGRESSION`.
- `ReviewDecision`: `APPROVE`, `REJECT`, `REQUEST_CHANGES`.
- `CanaryState`: `PLANNED`, `VALIDATING`, `HEALTHY`, `PROMOTED`,
  `ROLLED_BACK`, `FAILED`.
- `HealthCriterionState`: `HEALTHY`, `DEGRADED`, `UNAVAILABLE`,
  `UNKNOWN`.
- `RollbackState`: `PLANNED`, `EXECUTING`, `RESTORED`, `FAILED`.
- Typed identifiers in `crates/nexus-domain`: `IncidentId`,
  `DiagnosisId`, `PatchId`, `ApprovalId`, `DeploymentId`, `RollbackId`
  (Rust-only at M1; no generated wire binding ripple).

Authority semantics locked by this ADR:

1. **DETECTED != DIAGNOSED != REPRODUCED != PATCHED != VERIFIED !=
   APPROVED != DEPLOYED != REMEDIATED.** Each lifecycle phase is a
   distinct, non-collapsed state. There is no `FIXED` or `REMEDIATED`
   vocabulary value a model can emit; `CLOSED` is reached only through
   real observed post-deploy verification.

2. **A model/agent may propose, never self-certify.** A diagnosis is a
   HYPOTHESIS until reproducible evidence supports it. There is no
   engine method that accepts a model's claim of "fixed"; verification
   comes from real reproduction before/after, validation gates, and
   post-deploy health/readiness checks.

3. **Approval binds to the exact patch digest.** `RemediationApproval`
   carries the canonical patch digest; approval of patch A can never
   authorize patch B. Reviewer, approver, and proposer are distinct
   principals.

4. **Patch scope is explicit.** `PatchProposal.files_changed` is the
   exact scope; unexpected expansion fails validation. The sandbox gate
   re-checks scope.

5. **Sandbox and security validation are mandatory gates.** A patch that
   fixes functionality but weakens security is rejected. Real OS-level
   sandbox certification is deferred to its exact owner (EP-043/EP-040);
   this crate proves the enforcement boundary and records the
   certification boundary.

6. **Rollback is deterministic and bound to known artifacts.**
   `RollbackPlan` references the known previous artifact/version; rollback
   is never improvised from model-generated source. The state machine
   (PLANNED -> EXECUTING -> RESTORED | FAILED) is deterministic now; real
   production deployment proof belongs to the deployment-owning node.

7. **Self-healing cannot self-expand authority.** The remediation engine
   cannot modify its own policy, grant itself capabilities, lower
   approval requirements, disable security gates, increase its own
   budget, or change tenant identity.

8. **Successful remediation is not a trusted skill.** A successful fix
   is only a skill CANDIDATE; the EP-018 evaluation/signing/trust/
   install process must approve it before it becomes reusable. The
   self-healing system cannot directly install its own generated skills.

## Alternatives considered

- Treating model assertion as diagnosis (rejected: violates the
  primary invariant that model-thinks-fixed != fixed).
- Collapsing lifecycle phases (rejected: would let a failure state
  masquerade as remediation).
- Allowing approval by free text (rejected: approval must bind to the
  exact patch digest).
- Permitting the healing loop to install its own skills (rejected:
  would bypass the EP-018 skill factory boundary).

## Consequences

Contracts are fail closed and unambiguous: later implementation cannot
confuse detection, diagnosis, reproduction, patching, verification,
approval, deployment, or remediation. The vocabulary README and the
healing crate must stay in sync; new public names require a new ADR and
schema/vocabulary update.

## Reversal

Reversing requires a new ADR demonstrating that the authority
distinctions are preserved by an equivalent or stronger model.
