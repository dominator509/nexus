# ADR-013 - Trust Vocabulary

Status: Accepted
Date: 2026-08-14
Owner: hermes-nexus-main

## Context

EP-009 owns secrets, trust, and the private mesh: secret references
(never values), bootstrap and device secret stores, a certificate
authority, service identity, a Headscale-compatible private mesh, and
short-lived capability tokens (SPEC-005, SPEC-020). The node contract
lists seven public interfaces (`SecretStore`, `BootstrapSecretStore`,
`DeviceSecretStore`, `CertificateAuthority`, `ServiceIdentity`,
`MeshController`, `CapabilityTokenIssuer`) owned by the Rust crate
`crates/nexus-trust`. SPEC-005 and SPEC-020 "Canonical terms" lock
`Secret Reference`, `Capability Token`, `Trust Zone`, `Service
Identity`, and `Device Identity`; the existing nexus-domain vocabulary
already carries `PrincipalType`, `CapabilityClass`, `ApprovalClass`,
`Risk`, and `AuthenticationStrength` (the latter from nexus-auth via
ADR-011). EP-005 M1 doctrine requires every new public name to come
from an accepted vocabulary or be added by an ADR and a schema update
in the same milestone.

## Decision

Add the following vocabulary-locked classes, owned by
`crates/nexus-trust` and documented in `docs/vocabulary/README.md`:

- `TrustZone` (SPEC-020 trust boundary; SPEC-005 behavior 7): `PUBLIC`,
  `GUEST`, `LOCAL`, `PRIVATE_MESH`. Every service, device, and mesh
  node belongs to exactly one zone; zone boundaries determine mTLS
  policy, WireGuard segment membership, and secret exposure. The OPA
  contextual policy consumes the same classification as
  `context.network_trust` (`UNTRUSTED` -> `PUBLIC`, `GUEST` -> `GUEST`,
  `TRUSTED` -> `LOCAL`/`PRIVATE_MESH`).
- `TokenState` (capability token lifecycle): `ACTIVE`, `REVOKED`,
  `EXPIRED`. Tokens are short-lived, audience restricted, resource
  restricted, action restricted, and non-transferable; `REVOKED` and
  `EXPIRED` are terminal.
- `SecretState` (secret lifecycle): `ACTIVE`, `ROTATING`, `REVOKED`.
  `ROTATING` means a new version is being installed; `REVOKED` means
  the reference no longer resolves.
- `CertificateState` (mTLS certificate lifecycle): `ACTIVE`,
  `EXPIRED`, `REVOKED`. Certificates are short-lived; `EXPIRED` is
  terminal after `not_after`, `REVOKED` is terminal before `not_after`.
- `ServiceIdentityState` (service identity lifecycle): `ACTIVE`,
  `SUSPENDED`, `REVOKED`. A service identity is the canonical service
  principal bound to an mTLS certificate; `SUSPENDED` stops new
  issuance without destroying the record, `REVOKED` terminates it.
- `MeshNodeState` (mesh node lifecycle): `PENDING`, `REGISTERED`,
  `ONLINE`, `OFFLINE`, `REVOKED`. `PENDING` means a node requested
  membership but is not yet registered; `REGISTERED` means it holds a
  WireGuard key pair and can connect; `ONLINE`/`OFFLINE` are
  operational observations; `REVOKED` is terminal.

New struct types (`SecretReference`, `SecretValue`,
`BootstrapBundle`, `DeviceSecretReference`, `CapabilityToken`,
`Certificate`, `ServiceIdentity`, `MeshNode`, `WireGuardConfig`,
`WireGuardPeer`) are interface records, not vocabulary classes; their
field names are camelCase wire-stable via serde. `SecretValue` and
`DeviceSecretValue` are redaction wrappers: they serialize as
`<redacted>` and never deserialize from wire payloads.

## Consequences

- Secrets are referenced by name and never enter model context
  (SPEC-005 behavior 6; INV-003): values are opaque
  `SecretValue`/`DeviceSecretValue` wrappers whose `Debug` output and
  serialization are redacted, and whose `Deserialize` implementations
  fail closed.
- No long-lived universal bearer token exists (INV-003; SPEC-005
  behavior 5): capability tokens are short-lived, scoped to
  audience/resource/action/tenant, and terminal after expiry or
  revocation.
- Services use mTLS and short-lived credentials (SPEC-005 behavior 7):
  certificates are short-lived by construction and revoked by the CA
  port; identity records never embed private key material.
- New synonyms or lifecycle states require an ADR + vocabulary update,
  mirroring ADR-011 for the auth node and ADR-012 for the policy node.
