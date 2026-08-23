# tests/observability - EP-038 M2 provider proofs

Owner: EP-038 M2 (SPEC-007). This crate proves the `nexus-otel`
provider layer through the REAL cargo machinery:

- OTLP/JSON wire shape: camelCase field names, base16 trace/span ids,
  fixed64 timestamps as strings, proto3 enum values (severity,
  SpanKind, aggregation temporality).
- Redaction before egress: secret-shaped canaries are absent from
  OTLP/JSON, Prometheus text, and structured-log output; a
  hand-built envelope that claims `policy_applied` but carries a
  secret is refused at the export boundary.
- Deterministic output: identical inputs produce identical payloads;
  resource attributes are sorted.
- Prometheus text 0.0.4: exact family rendering, label/docstring
  escaping, value formatting (NaN/+Inf/-Inf), name validation.
- Structured-log fallback shape and redaction bookkeeping.

Run: `cargo test -p nexus-observability-tests --locked`
