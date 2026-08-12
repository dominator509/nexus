# SPEC-000 - Product Scope and Constitutional Priorities

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define the user-visible product, self-hosted-first selection law, release profiles, core outcomes, and what Nexus must never become.

## Canonical terms

Nexus, Control Plane, Home Edge, Compute Node, Provider, Connector, Sidecar, Capability, Action Gateway, World Model, Memory Fabric, Objective, Live-Fire Proof, Provider Certification. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Nexus presents one logical assistant identity across voice, web, desktop, mobile, agents, businesses, and devices.
2. Every feature selection follows deterministic code, local open-source engine, user-owned remote node, primary API, secondary API, then human decision.
3. The one-package setup wizard supports managed cloud, BYOC, existing SSH, hybrid, and fully local profiles from the same distribution.
4. Core self-hosted operation remains functional without Nexus-operated cloud services.
5. All twenty-eight outcomes in LIVE_FIRE_PROOFS.md are final ship criteria for the full release profile.
6. Optional providers are disabled and labeled unverified until real provider certification passes.
7. Commercial value resides in Nexus orchestration, memory, policy, lifecycle, user experience, and interoperability rather than unnecessary forks of mature engines.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Foundation-model training from scratch
- Universal autonomous authority
- Mandatory Kubernetes
- Mandatory local generative inference
- Vendor security bypass
- Robot hardware in V1

## Required tests

- Product vocabulary snapshot
- Release-profile capability test
- No hidden-cloud dependency static test
- Live-fire registry completeness test

## Acceptance

Every advertised feature maps to a capability, owner node, release profile, test, live-fire proof or provider-certification proof, and rollback or disable path.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
