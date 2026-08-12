# SPEC-008 - Production Readiness, Certification, and Ship Standard

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define core release, optional provider certification, hardware certification, final live-fire, review, drills, and release evidence.

## Canonical terms

Core Release, Release Profile, Provider Certification, Hardware Certification, Green Tag, Release Candidate, Ship Gate, Accepted Risk. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Core release requires every core profile proof and all mandatory security, data, workflow, installation, update, backup, and rollback gates.
2. Optional providers require real credentials and observable external effects before UI certification.
3. Hardware classes require model, firmware, transport, capability, latency, privacy, and failure evidence in the hardware matrix.
4. Final ship uses a fresh-clone-equivalent environment and reruns verify, production-readiness, and all active live-fire from scratch.
5. Restore, rollback, provider failover, identity recovery, Sentinel containment, and update failure drills have dated evidence.
6. No critical vulnerability, unreviewed license, missing SBOM, stale backup, or failed required proof may be accepted by a generic waiver.
7. Production deployment remains a manual command because auto-deploy is not authorized.
8. Release notes distinguish implemented, certified, experimental, unavailable, and deferred capabilities.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Paper readiness
- Passing from cached output
- Certifying every device on a vendor family
- Production deploy from the coding graph

## Required tests

- Production gate self-test
- Fresh clone build
- Full live-fire
- Restore and rollback drills
- Provider and hardware matrix audit
- Release copy truth test

## Acceptance

EP-043 observes all gate sentinels, signs the release, creates evidence index, prints the exact manual deploy command, and appends RUN_COMPLETE.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
