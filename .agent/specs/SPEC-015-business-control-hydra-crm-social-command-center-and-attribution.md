# SPEC-015 - Business Control, Hydra CRM, Social Command Center, and Attribution

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define Hydra bounded context, CEO interface, social sidecar and direct APIs, customer identity linking, leads, campaigns, and revenue attribution.

## Canonical terms

HydraBinding, BusinessContext, CustomerReference, Campaign, SocialAccount, SocialMessage, LeadHandoff, Attribution, CEOBrief. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Hydra remains canonical for CRM and revenue relationship records; Nexus stores references and cross-domain projections.
2. Nexus accesses Hydra only through authenticated MCP, REST, and durable events; no direct Hydra database access.
3. Business agents receive a single business scope unless explicitly authorized for portfolio-level reads.
4. Postiz is an isolated AGPL sidecar for scheduling and connector breadth; direct official APIs implement strategic gaps.
5. Social content supports platform-native variants, calendar, approvals, inbox, moderation, analytics, listening, attribution, and CRM handoff.
6. A social identity is linked to a Hydra person only through deterministic or human-reviewed identity resolution.
7. CEO briefs combine permitted CRM, social, communications, finance, and operational sources with provenance and data freshness.
8. Paid-ad budget changes and public crisis responses require human approval.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Duplicating Hydra CDM
- Blind social auto-replies
- Scraping where official APIs prohibit it
- Automatic person merge from an LLM guess

## Required tests

- Hydra capability and event contract
- Cross-business isolation
- Social publish certification
- Lead handoff live-fire
- Attribution reconciliation
- CEO brief provenance

## Acceptance

Nexus can command a portfolio through Hydra and social systems without vendor leakage, duplicate truth, unsafe merges, or unapproved external messaging.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
