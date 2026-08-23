# infra/otel - OpenTelemetry provider layer (EP-038 M2)

Owner: EP-038 M2 (SPEC-007). M2 owns the deterministic provider
behavior on top of the M1 contract crate (`crates/nexus-observability`).

## Contents

- `src/otlp.rs` - OTLP/JSON serialization for traces, metrics, and
  logs, hand-rolled against the authoritative `opentelemetry-proto`
  wire format:
  - `trace_id`: 16 bytes -> 32 lowercase base16 chars in OTLP/JSON
  - `span_id`: 8 bytes -> 16 lowercase base16 chars
  - camelCase field names (`resourceSpans`, `scopeSpans`, `traceId`,
    `spanId`, `startTimeUnixNano`, `severityNumber`, ...)
  - `fixed64` timestamps as decimal strings (proto3 JSON mapping)
  - SpanKind INTERNAL=1; StatusCode UNSET=0/ERROR=2
  - SeverityNumber DEBUG=5 INFO=9 WARN=13 ERROR=17 FATAL=21
  - Sum aggregation temporality CUMULATIVE=2, monotonic for counters
- `src/prometheus.rs` - Prometheus text exposition format 0.0.4 writer
  (node-contract fallback): `# HELP`/`# TYPE` lines, label/docstring
  escaping (`\\`, `\"`, `\n`), sorted labels, trailing LF.
- `src/structured.rs` - bounded JSON-lines structured log fallback.
- `src/export.rs` - export boundary: the ONLY entry points to the
  serializers. Accepts `RedactedEnvelope` only and re-verifies
  `assert_exportable()` before any byte is produced. There is no API
  that accepts raw observed events.

## Certification boundary (honest)

- OTLP/JSON serialization for traces/metrics/logs: INTERNAL PROVIDER
  CERTIFIED for the exact exercised wire shapes (unit-tested against
  the authoritative proto field names/enum values).
- Prometheus text 0.0.4 writer: FORMAT CERTIFIED for exact exercised
  grammar (HELP/TYPE/sample lines, escaping, LF termination).
- Structured-log fallback: CERTIFIED for exact exercised JSON shape.
- NOT ASSERTED: a Prometheus server, an OpenTelemetry collector, OTLP
  transport over a network, Grafana, GlitchTip, Loki, Tempo, Jaeger,
  incident delivery, or production monitoring deployment. Those belong
  to later milestones.

## Replacement and fallback (node contract)

When external collectors are unavailable, local structured logs and
Prometheus text metrics satisfy the same public contract. The fallback
never simulates success.
