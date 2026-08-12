# SPEC-020 - Privacy, Data Governance, Retention, Export, and Deletion

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define classification, purpose, consent, egress, memory privacy, recordings, calls, cameras, social, business data, and user rights.

## Canonical terms

DataClass, Purpose, Consent, EgressDecision, Retention, LegalHold, Export, Deletion, Redaction, PrivateResponse. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Data classes are public, household, personal, sensitive, confidential-business, security, secret, biometric-evidence, audio, image, and regulated-profile.
2. Every store and event declares data classes, purpose, owner, retention, encryption, export, deletion, and egress policy.
3. Raw audio and transient vision frames expire by default; recordings follow explicit per-camera and per-call policies.
4. Provider egress is disclosed, logged by provider and data class, and denied when privacy policy requires local processing.
5. Shared-space voice never speaks sensitive content when privacy evidence is insufficient.
6. Memory training or Microbrain datasets exclude user content unless opt-in, scrubbed, and separately approved.
7. Export and deletion span canonical state, projections, artifacts, caches, vector indexes, connectors, and backups according to documented constraints.
8. No regulated certification claim appears unless the corresponding profile is separately validated.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Collect everything because it may be useful
- Permanent raw voice storage
- Training from private traffic by default
- False compliance badges

## Required tests

- Data inventory completeness
- Egress policy
- Shared-space private response
- Export and deletion
- Backup retention
- Training dataset scrub

## Acceptance

A user can understand, export, restrict, and delete their data and can see every configured external egress path.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
