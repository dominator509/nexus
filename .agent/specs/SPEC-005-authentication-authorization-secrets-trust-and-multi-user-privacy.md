# SPEC-005 - Authentication, Authorization, Secrets, Trust, and Multi-User Privacy

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define people, devices, service identities, passkeys, step-up, OpenFGA relationships, OPA policies, secrets, mTLS, and shared-space privacy.

## Canonical terms

Principal, Person, Service Identity, Device Identity, Authentication Strength, Presence Evidence, Relationship Tuple, Policy Decision, Capability Token, Approval Assertion, Secret Reference, Trust Zone. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Keycloak provides OIDC, OAuth2, passkeys, federation, short-lived access tokens, refresh rotation, and service identities.
2. OpenFGA determines relationships; OPA evaluates contextual policy; Nexus Action Gateway combines both with risk, presence, device, time, and requested capability.
3. Voice, face, BLE proximity, room occupancy, and geofence are evidence only and cannot authorize R3 or R4 actions.
4. R3 and R4 actions require a cryptographic step-up or explicit preauthorization; R4 never accepts model approval.
5. Capability tokens are short-lived, audience restricted, resource restricted, action restricted, and non-transferable where platform support permits.
6. OpenBao stores central secrets; SOPS and age protect bootstrap configuration; mobile and desktop use platform secure stores; connector code receives secret references rather than durable plaintext.
7. Headscale-compatible WireGuard and mTLS protect node communication. Public ingress is minimized and Caddy terminates TLS where used.
8. Shared-room responses route sensitive content to a private device when other people may be present.
9. Every authorization decision creates a redacted receipt with policy version and evidence references.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Voice as a password
- Permanent universal API keys
- Secrets in memory or prompts
- Client-side-only permissions

## Required tests

- Passkey and token lifecycle
- OpenFGA tuple matrix
- OPA policy table
- Step-up and four-eyes tests
- Cross-user memory denial
- mTLS revocation
- Shared-room private-response live-fire

## Acceptance

No tested principal, device, model, agent, connector, or tenant can obtain a capability outside its relationships, policy, token, and approval class.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
