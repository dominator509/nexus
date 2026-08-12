# SPEC-004 - User Experience, Dashboard, Desktop, and Onboarding

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define a nontechnical setup wizard and consistent web, desktop, and device-management experience.

## Canonical terms

Nexus Setup, Deployment Plan, Fleet View, Operations Center, Approval Card, Integration Card, Device Naming, Recovery Kit, Release Channel. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. A user deploys the reference profile without editing shell, YAML, JSON, Terraform, or container files.
2. The setup wizard profiles hardware, shows placement, privacy, recurring cost, and fallbacks, then executes a resumable transactional plan.
3. Owner recovery material is created before cloud provisioning and is never recoverable from Nexus SaaS in self-hosted mode.
4. Home-edge enrollment uses a one-time QR or code that becomes device-bound mTLS identity.
5. The dashboard provides chat, voice status, objectives, agents, approvals, home, security, businesses, social, communications, fleet, costs, incidents, memory, skills, and integrations.
6. Every UI state includes loading, empty, error, degraded, permission-denied, and success behavior.
7. Web and desktop meet WCAG 2.2 AA and support keyboard, screen reader, reduced motion, large text, and non-color status.
8. The UI displays provider certification, self-hosted or API route, cost, privacy, and data egress before activation.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Requiring expert terminology
- Hiding errors behind indefinite spinners
- UI authorization without server policy
- Separate cloud and self-hosted product forks

## Required tests

- Playwright onboarding path
- Accessibility scan and keyboard test
- Resume-after-failure test
- No-secret-in-UI test
- Cost and egress disclosure snapshot

## Acceptance

A clean machine can complete the guided local profile, sign in, enroll an edge, discover a device, run a guided command, and export a support bundle.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
