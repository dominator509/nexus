# ADR-003: License-policy copyleft vocabulary alignment

Status: Accepted
Date: 2026-08-12
Author: hermes-nexus-main (EP-000 M4)

## Problem

`scripts/license_validate.py` (L5 gate) fails when `"GPL" in registry and "copyleft" not in policy.lower()`.
`LICENSE_POLICY.md` defines the SIDECAR class for GPL and AGPL components with
full source-offer and legal-review duties -- the policy is copyleft policy in
substance -- but never contains the literal token "copyleft". The unmodified pack
therefore fails its own license gate.

## Decision

Add the token "copyleft" to the SIDECAR class description in `LICENSE_POLICY.md`.
This is a vocabulary alignment, not a policy change: the SIDECAR obligations
(separate process, documented API, notices, source offer, legal review) are
unchanged. The gate's trigger token and the policy's substance now agree.

## Evidence

- Before: `sh scripts/license-gate.sh` -> `license validation: FAIL - GPL policy absent`.
- After: `sh scripts/license-gate.sh` -> `license gate: ok`.
- SIDECAR class text unchanged except the parenthetical `(copyleft)`.

## Alternatives considered

1. Removing the gate's copyleft check -- rejected: L5 gates never weaken.
2. Rewriting the policy class -- rejected: substance already correct; only token alignment needed.
3. Leaving the gate failing -- rejected: blocks every node and the ship gate.

## Compatibility impact

Documentation-only change; no schema, contract, or behavior impact. Rollback:
revert the one-line edit.
