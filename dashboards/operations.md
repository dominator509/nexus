# EP-038 Observability Operations (M5)

Operational procedures for the EP-038 owned components as actually
implemented and exercised by the M1-M5 gates. Nothing here claims
production fleet behavior that was not run.

## 1. Observability stack diagnosis

- The ops diagnostic ladder lives in `nexus-observability-ops`
  (`infra/observability/src/diag.rs`). States are strictly ordered:
  CONFIGURED != REACHABLE != RESPONDING != READY.
- READY requires a real production probe (envelope POST + readback),
  never config alone. If READY is reported, the probe passed.
- The probe ladder for the GlitchTip path is also available as
  `scripts/probes/glitchtip.sh` (CONFIGURED -> REACHABLE -> RESPONDING
  -> AUTHENTICATED -> READY). It never prints the DSN key or token.

## 2. Dashboard validation

- `cargo run -p nexus-dashboards --bin dashboard-validate` validates
  every `dashboards/*.json` against the canonical catalog. Exit 0 means
  all dashboards are syntactically valid, catalog-backed, redaction-safe,
  and free of green-on-no-data.
- A dashboard that renders green from no data, references an unknown
  metric, or contains a secret-shaped literal fails validation.
- Full proof: `cargo test -p nexus-dashboards` (13 tests).

## 3. Redaction policy checks

- Redaction is enforced by the M1 `RedactionPolicy`; the M2 export
  boundary re-verifies `assert_exportable()` before any byte leaves.
- The M4 runtime rejects secret-shaped context at construction and
  quarantines incidents locally if delivery fails.
- Secret-shaped values are `sha256:` fingerprinted, never rendered.
- Checks: run the M4 gate (`sh scripts/ep038-m4-tests.sh`) and look for
  the secret-canary proofs; the M5 dashboard validator scans every
  dashboard JSON for secret-shaped literals.

## 4. Incident sink diagnosis

- The real sink is GlitchTip 6.1.8 (M3). Envelope ingestion is
  authenticated from the `X-Sentry-Auth` header (the envelope-body dsn
  is ignored). A healthy POST returns HTTP 200; the embedded worker
  processes asynchronously, so readback must be polled against a
  deadline.
- 401 after delete = revoked token; refused connection = provider
  stopped; 5xx = provider error. All map to canonical SPEC-006 codes.

## 5. GlitchTip stopped-provider handling

- When the provider is stopped, the transport observes connection
  refused -> `Unavailable`. The M4 runtime's bounded recovery retries
  only `Unavailable`/`Timeout`/`ExternalProvider`; `Authorization`/
  `Policy` never retry.
- Restart the same provider and re-run the production probe before
  treating it as ready. healthz alone is never readiness.

## 6. Revoked-token handling

- After a token is deleted in the GlitchTip DB, readback returns 401 ->
  `Authorization`. The diagnostic ladder reports NOT READY (fail
  closed). Do not retry authorization failures; mint a fresh token and
  re-probe.
- Tokens must never appear in argv or logs; use mode-600 temp files.

## 7. Restart recovery

- The M4 restart-recovery phase proves: same provider restarted ->
  generation increments -> full production readiness probe ->
  subsequent production operation succeeds. A fresh token is minted
  after the revoked phase.

## 8. Health ladder interpretation

- CONFIGURED != REACHABLE != RESPONDING != READY != HEALTHY.
- Stale observations compose to Unknown/Degraded, never healthy.
- A node with no current report is stale/unknown, never green.

## 9. SLO no-data interpretation

- `NoData` (zero events) and `InsufficientEvidence` are explicitly NOT
  met. A dashboard/SLO view must show grey/no-data, never green.
- `nexus.slo.*` ids in the dashboards carry this semantic.

## 10. Metric cardinality troubleshooting

- The M1 catalog rejects unbounded labels and high-cardinality raw
  values (`CardinalityPolicy::DenyHighCardinality`).
- The M5 dashboard validator rejects raw UUID/artifact-shaped label
  values in selectors. Prefer canonical low-cardinality labels
  (`node`, `source`, `classification`).

## 11. Cleanup / resource hygiene

- Every EP-038 gate owns its containers/network/volume/temp files and
  removes them on success, failure, panic, and trap (EXIT). Named
  resources carry the `nexus-ep038-` prefix.
- Verify zero owned residue after each run:
  `docker ps -aq --filter name=^/nexus-ep038-` must be empty.
- Never use a broad `docker prune` as a substitute for correct cleanup.

## 12. Rollback / disable procedures

- The dashboard validator and ops crate are pure config/code additions;
  rollback = revert the M5 commit (`git revert`) - no data migration.
- To disable the GlitchTip sink, configure the runtime without a DSN;
  incidents remain quarantined locally (never lost) and are not
  delivered. To disable dashboards, remove `dashboards/*.json`; the
  gate then fails (intentional - the gate proves owned content).
- Closed-milestone invariants (M1/M2/M3/M4) are not altered by M5; the
  M1-M4 regression gates remain the rollback safety net.

## Certification boundary

Procedures above document only what the M1-M5 gates actually ran:
real GlitchTip 6.1.8 fixture, real postgres 18.4, real redis 7-alpine,
real envelope POST + worker + readback, real stopped/revoked/restart
phases, real dashboard JSON validation. NOT ASSERTED: production
Grafana/Prometheus/OTel collector, Loki/Tempo/Jaeger, PagerDuty/Slack/
email delivery, fleet-wide telemetry, production monitoring operations.
