# SPEC-018 - Self-Healing Engineering and Incident Feedback Loop

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define automated detection, investigation, patch preparation, testing, review, approval, canary, rollback, and learning.

## Canonical terms

Incident, Diagnosis, Reproduction, PatchCandidate, Review, Canary, HealthCriterion, Rollback, SkillCandidate, IncidentMemory. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. OpenTelemetry and GlitchTip signals create deduplicated incidents with severity, affected components, first and last occurrence, and correlations.
2. Investigation gathers logs, traces, metrics, recent changes, dependency advisories, component health, and minimal source context.
3. A coding agent works in an isolated worktree and sandbox with scoped repository and test capabilities.
4. An independent reviewer examines root cause, diff, tests, security, compatibility, and rollback.
5. Production application of code, policy, schema, permissions, or secrets requires the configured human approval class.
6. Deployments are staged, observed against explicit criteria, promoted, or automatically rolled back.
7. Successful generalizable procedures may become signed skill candidates after eval and approval.
8. The system never rewrites its own immutable control laws or bypasses graph and release gates.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Unreviewed self-modification
- Auto-deploy of security or auth changes
- Patching by log text alone
- Deleting evidence after closure

## Required tests

- Controlled defect live-fire
- Reproduction proof
- Patch and reviewer disagreement
- Approval denial
- Canary regression rollback
- Skill candidate promotion

## Acceptance

A controlled crash progresses to an evidence-backed fix or terminal blocked report without production surprise or lost audit.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
