# ADR-007 - Identity and Presence Vocabulary

Status: Accepted
Date: 2026-08-12
Owner: hermes-nexus-main

## Context

EP-003 owns people, households, businesses, devices, sessions, presence
evidence, and tenant boundaries (SPEC-001, SPEC-005). The node contract
lists ten public interfaces. Several require canonical enum classes that do
not yet exist in `docs/vocabulary/README.md`. SPEC-001 rule 2 of the
"Required behavior" section and the EP-003 milestone doctrine require every
new public name to come from an accepted vocabulary or be added by an ADR
and a schema update in the same milestone.

## Decision

Add the following vocabulary-locked enums, owned by `crates/nexus-identity`
and documented in `docs/vocabulary/README.md`:

- `EvidenceKind`: `VOICE`, `ROOM`, `BLE`, `MOBILE`, `CAMERA`. Source:
  EP-003 acceptance obligation 2 ("Voice, room, BLE, mobile, and camera
  evidence combine without becoming cryptographic authentication") and
  SPEC-005 behavior 3.
- `ConfidenceLevel`: `LOW`, `MEDIUM`, `HIGH`. Deterministic bands over the
  fused evidence score; evidence is never authentication (INV-003).
- `DeviceKind`: `PHONE`, `TABLET`, `DESKTOP`, `LAPTOP`, `SPEAKER`, `CAMERA`,
  `DISPLAY`, `SERVER`, `APPLIANCE`, `UNKNOWN`. Provider-neutral device
  classes (SPEC-012, SPEC-017).
- `TrustLevel`: `UNVERIFIED`, `LOCAL`, `VERIFIED`. Device trust ladder;
  distinct from authentication strength, never a substitute for a
  cryptographic step-up (SPEC-005 behaviors 3-4).
- `LifecycleState`: `PENDING`, `ACTIVE`, `SUSPENDED`, `DISABLED`, `ARCHIVED`.
  SPEC-001 world-entity lifecycle state.
- `SessionState`: `ACTIVE`, `EXPIRED`, `REVOKED`. Sessions are independently
  scoped from people and devices (EP-003 acceptance obligation 1).

These enums parse from their canonical wire strings and reject unknown
values, following the `nexus-domain` vocabulary pattern.

## Evidence

- `.agent/node-contracts/EP-003.md` interface map
- `docs/vocabulary/README.md` (updated in this milestone)
- `crates/nexus-identity/src/*` unit tests `ep003_unit_*`
- `schemas/identity/` JSON Schemas (M3) mirror the same canonical strings

## Alternatives rejected

- Reuse free-form strings for evidence kind and device kind: loses the
  parse-time rejection that the vocabulary pattern provides.
- Add the enums to `nexus-domain`: EP-003 owns identity/presence semantics;
  `nexus-domain` stays the shared lower layer.

## Consequence

`crates/nexus-identity` depends on `nexus-domain` (typed IDs, PrincipalType,
Privacy, Risk, Locality) and adds the new enums. `schemas/identity/` (M3)
mirrors them for cross-language contracts. Reversal: remove the enums, ADR,
and vocabulary entries together; the schema update in M3 is the compatibility
boundary.

## Security and license impact

No new dependency; no license impact. Evidence kinds never grant R3/R4
authority (INV-003, SPEC-005 behavior 3).
