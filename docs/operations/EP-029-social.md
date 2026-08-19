# EP-029 Social Command Center - Operations

Owned components: `crates/nexus-social` (contract), `connectors/postiz`
(Postiz sidecar adapter), `connectors/social-direct` (direct X API v2
connector), `infra/postiz` (failure suite + diagnostics), `tests/social`
(live-fire evidence).

## Health

- Contract: `cargo test -p nexus-social`
- Postiz adapter: `cargo test -p nexus-postiz-connector`
- Direct connector: `cargo test -p nexus-social-direct-connector`
- Failure suite: `cargo test -p nexus-postiz-e2e`
- Live-fire: `cargo test -p nexus-social-live-e2e`

## Diagnostics

`sh infra/postiz/postiz-diag.sh <base-url>` probes a Postiz endpoint.
It NEVER reports healthy from configuration existence: an unreachable
endpoint exits non-zero with `reachable=no`. Treat exit 0 as
"reachable and answering"; anything else as degraded.

## Publish-state interpretation (APPROVED != PUBLISHED)

- GRANTED approval is NOT publication.
- Provider create-post acceptance (draft/schedule/now) is PROVIDER
  ACCEPTANCE, not independent proof of publication.
- The only PUBLISHED authority is an independent provider readback
  (documented GET /posts) showing the post in `published` state.
- A schedule acceptance is SCHEDULED, never PUBLISHED.

## Approval classes (never collapse)

- PUBLISH: HUMAN
- REPLY: POLICY
- SPEND_CHANGE: STRONG_HUMAN (>= HUMAN required)
- CRISIS_STATEMENT: FOUR_EYES (>= HUMAN required)

An insufficient class fails closed with Policy and makes ZERO provider
calls. Denied actions are still recorded in the audit ring with
correlation (observability never depends on provider success).

## Redaction

Credentials (Postiz API key, X bearer token) are registered as
observability redaction secrets. Errors, audit entries, and evidence
never contain them. The live-fire canaries assert ZERO_LEAKAGE.

## Certification boundaries (honest)

- Postiz connector: IMPLEMENTED / TRANSPORT_CERTIFIED against
  controlled HTTP fixtures (documented public API surface).
- Direct X connector: IMPLEMENTED / TRANSPORT_CERTIFIED against
  controlled real-socket fixtures (documented X API v2 surface).
- Real Postiz provider: NOT ASSERTED (no owned credentials here).
- Real X provider: NOT ASSERTED (no owned credentials here).
- Postiz inbox/analytics/leads: NOT IMPLEMENTED / FAIL-CLOSED BY
  DESIGN (no documented API; the direct connector covers the gaps).
- Spend/crisis execution: FAIL CLOSED (no documented spend surface on
  either provider).

## Evidence

- `.agent/state/evidence/LF-014-ep029-m5.json`
- `.agent/state/evidence/LF-027-ep029-m5.json`

Both embed `EP029_M5_RUN_ID`; stale evidence never satisfies the M5
gate.

## Shutdown / isolation

The connectors make outbound HTTP only to the configured base URL.
No control-plane containers, no child processes, no persistent
listeners. The e2e fixtures are in-process std::net listeners that
close when the test binary exits.
