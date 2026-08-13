# ADR-011 - Authentication and Passkey Vocabulary

Status: Accepted
Date: 2026-08-13
Owner: hermes-nexus-main

## Context

EP-007 owns authentication and passkeys: deploy Keycloak and implement
OIDC, passkeys, service identities, sessions, device enrollment, and
step-up (SPEC-005). The node contract lists seven public interfaces
(`OidcClient`, `TokenValidator`, `PasskeyEnrollment`,
`DeviceEnrollment`, `SessionService`, `StepUpChallenge`, `RecoveryKit`)
owned by the Rust crate `crates/nexus-auth`. SPEC-005's "Canonical
terms" locks `Authentication Strength`, `Service Identity`, `Capability
Token`, `Secret Reference`, and `Trust Zone`; `AuthenticationStrength`
was added to `docs/vocabulary/README.md` by ADR-010 (workflow node) but
has no Rust representation yet. EP-005 M1 doctrine requires every new
public name to come from an accepted vocabulary or be added by an ADR
and a schema update in the same milestone.

## Decision

Add the following vocabulary-locked classes, owned by `crates/nexus-auth`
and documented in `docs/vocabulary/README.md`:

- `AuthenticationStrength` (Rust mirror of the ADR-010 name): `NONE`,
  `SINGLE_FACTOR`, `MULTI_FACTOR`, `STEP_UP`, ordered as a strength
  ladder (`NONE < SINGLE_FACTOR < MULTI_FACTOR < STEP_UP`). SPEC-005
  behavior 4: R3/R4 actions require `STEP_UP` or explicit
  preauthorization; R4 never accepts model approval.
- `TokenClass`: `ACCESS` (short-lived bearer), `REFRESH` (rotation-only),
  `ID` (identity claims).
- `PasskeyState`: `PENDING_CHALLENGE`, `REGISTERED`, `REVOKED`.
- `DeviceEnrollmentState`: `PENDING_VERIFICATION`, `ENROLLED`,
  `REJECTED`, `REVOKED`.
- `StepUpState`: `PENDING`, `SATISFIED`, `EXPIRED`, `CANCELLED`.
- `RecoveryMaterialKind`: `SEALED_ENVELOPE`, `SPLIT_SHARES`,
  `RECOVERY_CODE`.
- `GrantFlow` (OIDC/OAuth2 authorization grant families): 
  `AUTHORIZATION_CODE`, `CLIENT_CREDENTIALS`, `REFRESH_TOKEN`.
- `RecoveryKitState`: `PROVISIONED`, `SEALED`, `VERIFIED`, `REVOKED`.

`Service Identity`, `Capability Token`, `Secret Reference`, and `Trust
Zone` remain vocabulary-locked SPEC-005 names; their full Rust and JSON
Schema representations belong to the node milestones that own them
(secret references are already surfaced as opaque strings in this node;
Capability Tokens are EP-008-owned).

## Consequence

`crates/nexus-auth` is the provider-neutral authentication contract
surface: it never imports an OIDC/Keycloak/WebAuthn SDK (enforced by
`ep007_unit_auth_crate_has_no_infrastructure_dependencies`). The
Keycloak adapter in `infra/keycloak` implements these contracts (EP-007
M2); integration tests use a real Keycloak container (EP-007 M3);
failure tests exercise unavailable provider, timeout, malformed input,
duplicate request, denied permission, and partial side effects
(EP-007 M4); LF-003 proves owner passkey onboarding live (EP-007 M5).

## Alternatives

- Reuse the EP-003 `Session` directly without an auth-layer projection:
  rejected. The auth layer must record issuance strength, refresh
  binding, and audit action without mutating EP-003's model.
- Free-form strength strings: rejected. Strength is an ordered,
  vocabulary-locked ladder; unknown values must fail closed.
- Strength as a provider enum (e.g. Keycloak's own step-up notions):
  rejected. Provider concepts normalize at the boundary, never into
  domain contracts (SPEC-005 inputs/outputs rule).

## Security and compatibility

Challenge and recovery payloads are opaque and never stored plaintext
after use; secret references point to the platform secure store, never
carry material. The vocabulary is additive; a major change requires a
new ADR, a schema version bump, and a drain of in-flight records. No
secrets in logs or audit records.
