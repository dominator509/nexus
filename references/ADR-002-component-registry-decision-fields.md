# ADR-002: Complete COMPONENT_REGISTRY.yaml decision fields

Status: Accepted
Date: 2026-08-12
Author: hermes-nexus-main (EP-000 M4)

## Problem

`scripts/license_validate.py` (L5 gate) requires every component registry to
contain `license:`, `integration_mode:`, `replacement_contract:`, and
`commercial_review:`. The blueprint pack's `COMPONENT_REGISTRY.yaml` ships with
`license:` and `integration_mode:` but no `replacement_contract:` or
`commercial_review:` entries, so `sh scripts/license-gate.sh` fails on the
unmodified pack.

EP-000's node contract acceptance obligation #2 states: "Every selected component
has a commercial integration mode and replacement boundary." The missing fields
are therefore owned implementation content for this node, not optional metadata.

## Decision

Add `replacement_contract:` and `commercial_review:` to every component entry in
`COMPONENT_REGISTRY.yaml`, with values grounded in:
- `VERSIONS.lock.yaml` (locked versions, licenses, policies)
- `SOURCE_REGISTRY.md` (replacement posture, copyleft notes)
- `references/SOURCE_VERIFICATION.json` (verified upstream evidence)

Authorize `COMPONENT_REGISTRY.yaml` in `.agent/expected-files/EP-000.txt` so the
scope audit accepts the change.

## Evidence

- Before: `grep -c replacement_contract COMPONENT_REGISTRY.yaml` -> 0; license gate exits 1.
- After: every component carries both fields; `sh scripts/license-gate.sh` -> `license gate: ok`.
- Values trace to the lock and source registry (e.g., Bifrost `replacement_contract`
  is `ModelGateway contract`; Postiz `commercial_review` is `isolated-sidecar; AGPL obligations`).

## Alternatives considered

1. Weakening the license gate to not require the fields -- rejected: L5 gates never weaken.
2. Leaving the gate failing -- rejected: blocks every node and the ship gate.
3. Filling placeholders -- rejected: the pack's anti-fabrication rule requires real values.

## Compatibility impact

Additive fields only; no schema, contract, or behavior change. Rollback: revert
the registry diff and the one-line fence addition.

## Security impact

None negative. The fields make the commercial and replacement posture of every
component explicit, which strengthens supply-chain review (SPEC-019).
