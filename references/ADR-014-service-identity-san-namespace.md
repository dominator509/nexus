# ADR-014 - Service Identity SAN Namespace

Status: Accepted
Date: 2026-08-14
Owner: hermes-nexus-main

## Context

EP-009 M4 must prove real certificate authority, service identity,
certificate lifecycle, and real mutual TLS. The `nexus-trust` contract
defines `ServiceIdentity` (identity_id, tenant_id, name, zone) but no
wire format exists for binding a certificate to that identity. Directives
C and H require one deterministic canonical identity namespace, SAN-based
binding, and standard (never disabled) hostname/SAN verification in the
mTLS proof.

## Decision

Every certificate issued by the Nexus CA carries TWO SANs, both derived
deterministically from the SAME `ServiceIdentity` record (one identity,
two encodings; not competing schemes):

1. Canonical URI SAN (the authoritative identity):
   `nexus://tenant/<tenant_id>/service/<identity_id>`
   - namespace prefix `nexus://tenant/`; exact segments
     `tenant/<tenant_id>` and `service/<identity_id>`; no extra
     segments, no wildcards, no free-form hostnames.
   - parsed and validated by `nexus-pki` `parse_canonical_uri`; the
     Nexus identity layer binds the authenticated mTLS peer to this URI
     and rejects mismatch (directive H.4).
2. Transport DNS SAN (for standard TLS hostname verification):
   `<identity_id>.<tenant_id>.svc.nexus.internal`
   - lowercase, colon-free encoding of the same two identity fields.
   - consumed by rustls `ServerName` verification on the client side
     (directive H.5: never disable hostname verification, never install
     a permissive custom verifier).

The Common Name of issued leaves is the transport DNS name (the OpenBao
role enforces hostnames; this keeps issuance inside the role's
`allowed_domains` while the URI SAN carries the canonical identity).

Issuance role constraints (directive D):
- `allowed_domains: svc.nexus.internal`, `allow_subdomains: true`,
  `allow_any_name: false`, `require_cn: false`,
  `enforce_hostnames: true`;
- `allowed_uri_sans: nexus://*` for the standard role and a restricted
  per-tenant variant for the role-violation proof;
- `key_type: ec`, `key_bits: 256`, `max_ttl: 24h`;
- `server_flag: true`, `client_flag: true` (serverAuth + clientAuth
  EKUs); no CA issuance from the leaf role.

## Consequences

- Identity binding is deterministic and testable: the URI SAN parses
  back into exactly (tenant_id, identity_id).
- Transport and canonical identity stay in lockstep because both derive
  from the same record.
- Hostname verification remains fully standard; the wrong-CA, wrong-SAN,
  wrong-EKU, expired, not-yet-valid, and malformed negative cases are
  real rustls failures.
- New identity namespaces require an ADR (vocabulary doctrine,
  SPEC-005/EP-005).

## Alternatives

- DNS-only identity binding (no URI SAN): rejected - the canonical
  identity would be overloaded onto a hostname and could not express
  tenant/service structure canonically.
- SPIFFE-style `spiffe://` namespace: rejected - Nexus already locks its
  own vocabulary; introducing SPIFFE would be a second identity scheme.
- Custom rustls verifier to check URI SANs at handshake: rejected - the
  identity layer performs the binding after the standard handshake,
  keeping verification standard and fail-closed.

## Reversal

Supersede via a new ADR; the vocabulary README and the
`nexus-pki::identity` module must change in the same milestone.
