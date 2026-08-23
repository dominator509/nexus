# Nexus Dashboards (EP-038 M5)

Real, validated Grafana-format dashboard configs backed by the M1
metric/health/SLO catalog semantics and the M4 ops catalog. Owned by
EP-038 M5 (`dashboards/`).

## Dashboards

- `nexus-health-overview.json` - composed health by node; health ladder
  (CONFIGURED != REACHABLE != READY != HEALTHY); no-data renders grey,
  never green.
- `nexus-incidents-slo.json` - incident deliveries, delivery failures,
  SLO error budget; NO_DATA and INSUFFICIENT_EVIDENCE never render
  healthy.
- `nexus-metrics-ops.json` - request rate, workflow duration, API
  availability; no-data renders grey.

## Truthfulness invariants (SPEC-007)

Every dashboard and panel preserves the permanent EP-038 truths:

- OBSERVED RAW EVENT != SAFE TO EXPORT
- LAST KNOWN HEALTHY != CURRENTLY HEALTHY
- TRACE ID PRESENT != TRACE EXPORTED != TRACE SAFE
- NO EVENTS != SLO MET
- NO ALERTS != SYSTEM HEALTHY
- CONFIGURED != REACHABLE != READY != HEALTHY

Concretely, the validator rejects any panel whose first threshold step
maps the null (no-data) bucket to green (`green_on_nodata`), rejects
selectors that reference metrics outside the canonical catalog
(`unknown_metric`), rejects secret-shaped literals anywhere in the JSON
(`secret_literal`), rejects raw high-cardinality label values
(`high_cardinality_label`), and fails closed on malformed JSON
(`unparseable`).

## Validation

The authority is the `nexus-dashboards` crate in this directory:

- `src/lib.rs` - dashboard model + validator + canonical catalog built
  from `nexus_observability_ops::ops_metric_definitions()`, the M1
  canonical fixture metrics, and every rule/slo id declared in
  `alerts/catalog.yaml` + `alerts/slo-catalog.yaml`.
- `src/bin/dashboard-validate.rs` - CLI: validates every
  `dashboards/*.json`, exits non-zero on any finding.
- `tests/ep038_m5_dashboards.rs` - 13 proofs including negative tests
  for every anti-pattern above.

Run from the repository root:

```sh
cargo run -p nexus-dashboards --bin dashboard-validate
cargo test -p nexus-dashboards
```

## Certification boundary (honest)

CERTIFIED:

- Dashboard JSON config is syntactically valid and structurally
  complete (uid/title/panels/datasource references/templating
  variables/query expressions).
- Every metric selector exists in the real canonical catalog
  (M1/M4 semantics); no invented metric ids.
- Redaction-safe: no secret-shaped literals (canary-tested).
- No green-on-no-data semantics: no-data/stale/insufficient-evidence
  never render healthy (validator-enforced).
- High-cardinality raw labels rejected.

NOT ASSERTED (no real instance exercised):

- Grafana server rendering (no Grafana instance was started for M5).
- Prometheus server ingestion/scraping.
- OpenTelemetry collector production pipeline.
- Loki/Tempo/Jaeger production deployment.
- PagerDuty/Slack/email incident delivery.
- Real fleet-wide telemetry.
- Production monitoring operations.

These dashboards are validated config artifacts for the local fallback
writers and any future certified Grafana deployment; the M5 gate proves
the config, not a live dashboard server.

## Notes

- M4's canonical ops metrics are dotted ids (e.g.
  `nexus.ops.health.composed`); the M2 Prometheus text writer validates
  names as `[a-zA-Z_:][a-zA-Z0-9_:]*` and therefore cannot render
  dotted ids. This is a real boundary gap surfaced by M5 dashboard
  validation: dashboard selectors use the canonical dotted catalog ids,
  and the local Prometheus fallback path for those ids is NOT asserted.
  Recorded in the ExecPlan Surprises; owned by deployment/ship review.
