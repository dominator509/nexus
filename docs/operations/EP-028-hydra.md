# EP-028 Hydra Business-Control Seam - Operations Runbook

Applies to: `crates/nexus-hydra` (contract), `connectors/hydra` (adapter +
HTTP transport), `schemas/hydra` (canonical JSON Schemas),
`tests/hydra/` (failure suite), `tests/hydra-live/` (live-fire evidence).

## Purpose

The authenticated Nexus-to-Hydra seam (SPEC-015): capability, context,
action, event, identity, and business-binding control. Hydra remains the
CRM canonical source; Nexus stores references and cross-domain
projections, never duplicated truth. All access is through authenticated
MCP, REST, and durable events - there is no direct-database channel.

## Health

- Adapter health is exercised by the failure suite and live-fire proofs:
  `cargo test -p nexus-hydra-e2e --all-targets` (forced failures) and
  `cargo test -p nexus-hydra-live-e2e --all-targets` (LF-015/LF-025).
- Ops diagnostic (fail-closed):
  `sh tests/hydra/ops/hydra-diag.sh diagnose [base_url]`
  - reachable provider -> truthful capabilities, exit 0;
  - unreachable provider -> `reachable=no`, exit 3 (never "healthy"
    merely because config exists).
- The diagnostic never prints credentials (Bearer/Authorization redacted).

## Readiness

- The canonical REST surface is versioned (schemas/hydra/):
  - `GET  {base}/v1/context` authorized business context projection
  - `GET  {base}/v1/capabilities` advertised capabilities
  - `POST {base}/v1/actions` submit a governed action
  - `GET  {base}/v1/actions/{id}` exact-target readback
- A provider is ready when capabilities advertise the required kinds and
  context reads return a canonical projection. Unadvertised capabilities
  resolve UNAVAILABLE (fail closed).

## Backup / Restore

- The seam stores no CRM truth: context projections are references with
  `observed_at` freshness. No Hydra data is duplicated on disk.
- Schema parity is the durable contract: `schemas/hydra/*.json` is the
  cross-language source; Rust serde output is validated against it at
  build time (M3 integration tests). Keep schemas/ under version control.
- Restore = rebuild from the committed crate/schema set and rerun the
  gates (below).

## Upgrade / Disable / Rollback

- Upgrade: change the canonical surface in `schemas/hydra/` with an ADR
  and vocabulary-locked enum updates in `crates/nexus-hydra`; the schema
  parity tests fail the build if Rust and JSON drift.
- Disable: do not bind a transport. `UnboundHydraProvider` fails closed
  (Unavailable) and advertises nothing; a fallback of read-only context +
  proposal generation is the documented replacement until execution
  capabilities advertise certified availability.
- Rollback: revert to the previous green tag (`green/EP-028` or the
  owning node's green tag) under LOOPS.md; never cross a completed green
  tag.

## Certification Boundary

- Hydra contract: INTERNAL_CERTIFIED (M1).
- Hydra adapter: IMPLEMENTED (M2).
- Hydra HTTP transport: TRANSPORT_CERTIFIED against controlled
  real-socket fixtures (M3, hardened M4).
- Real Hydra/CRM provider: NOT ASSERTED (no component selected in
  COMPONENT_REGISTRY; real provider certification DEFERRED with owner).
- Postiz/social providers: NOT ASSERTED (EP-029 owner).
- Direct database access: NOT A SUPPORTED CHANNEL (structural).

## Evidence

- LF-015: `.agent/state/evidence/LF-015-ep028-m5.json` (current-run
  run_id bound; stale never satisfies).
- LF-025: `.agent/state/evidence/LF-025-ep028-m5.json` (current-run
  run_id bound; stale never satisfies).
- Evidence is machine-readable, redacted (credential canaries scanned),
  and written from the live-fire proofs over REAL sockets.
