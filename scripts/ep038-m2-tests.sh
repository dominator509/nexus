#!/usr/bin/env sh
# EP-038 M2 gate: OpenTelemetry provider layer (SPEC-007) through the
# REAL cargo machinery with vacuity guards and anti-masking.
#
# M2 owns infra/otel/ (nexus-otel provider crate) and tests/observability/
# (nexus-observability-tests proof crate). The gate proves:
# - OTLP/JSON wire shape (camelCase names, base16 ids, fixed64 strings)
# - redaction-before-egress with secret canaries
# - Prometheus text 0.0.4 fallback + structured-log fallback
# - deterministic output
# - dependency direction (nexus-otel consumes M1 contracts only)
# - M1 regression stays green
set -eu
export CI=true
export CARGO_TERM_COLOR=never

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

log="/tmp/ep038-m2-tests.log"
: > "$log"

fail() {
  echo "EP-038 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-038 M2 gate: $1"; }

# Vacuity guard 0: M2-owned material must exist.
if [ ! -f infra/otel/Cargo.toml ]; then
  fail "infra/otel/Cargo.toml missing"
fi
for f in src/lib.rs src/otlp.rs src/prometheus.rs src/structured.rs src/export.rs README.md; do
  if [ ! -f "infra/otel/$f" ]; then
    fail "infra/otel/$f missing"
  fi
done
if [ ! -f tests/observability/Cargo.toml ]; then
  fail "tests/observability/Cargo.toml missing"
fi
if [ ! -f tests/observability/tests/ep038_m2_provider.rs ]; then
  fail "tests/observability/tests/ep038_m2_provider.rs missing"
fi
ok "M2-owned material present"

# Real test run through cargo.
if ! sh -c 'cargo test -p nexus-otel -p nexus-observability-tests --locked >> "$1" 2>&1' _ "$log"; then
  fail "cargo test failed" "$log"
fi

# Vacuity guard 1: non-zero passing count observed.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: zero failures.
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 3: zero ignored tests.
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi

# Vacuity guard 4 (anti-masking): EP-038-owned M2 proofs observed.
for sentinel in \
  ep038_unit_otlp_log_severity_mapping_exact \
  ep038_unit_otlp_log_camelcase_wire_names \
  ep038_unit_otlp_span_ids_base16_hex \
  ep038_unit_otlp_metric_counter_is_sum_cumulative \
  ep038_unit_otlp_metric_gauge_shape \
  ep038_unit_otlp_metric_histogram_unsupported_truthful \
  ep038_unit_otlp_resource_attributes_include_tenant_hash_only \
  ep038_unit_redaction_canary_absent_otlp_log \
  ep038_unit_redaction_canary_absent_otlp_span \
  ep038_unit_redaction_canary_absent_structured_log \
  ep038_unit_export_boundary_rejects_non_exportable \
  ep038_unit_export_boundary_no_raw_event_api \
  ep038_unit_prometheus_counter_family_exact \
  ep038_unit_prometheus_label_escaping_exact \
  ep038_unit_prometheus_help_escaping_and_last_lf \
  ep038_unit_prometheus_rejects_invalid_metric_name \
  ep038_unit_prometheus_value_formatting \
  ep038_unit_prometheus_histogram_type_header_truthful \
  ep038_unit_structured_log_json_line_shape \
  ep038_unit_structured_log_redacted_list_recorded \
  ep038_unit_otlp_output_deterministic \
  ep038_unit_otlp_resource_sorted_reproducible \
  ep038_unit_severity_mapping_canonical \
  ep038_unit_otlp_span_rejects_malformed_ids \
; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-038-owned M2 test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-038-owned M2 proofs observed"

# Vacuity guard 5: dependency direction - nexus-otel consumes the M1
# contracts (nexus-observability, nexus-domain) and serde_json only.
# It must not import a vendor telemetry SDK.
bad_dep=$(cargo tree -p nexus-otel --depth 1 2>/dev/null | grep -vE 'nexus-otel|nexus-observability|nexus-domain|serde_json|serde' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-otel: $bad_dep"
fi
for forbidden in opentelemetry-prometheus prometheus-rs opentelemetry-otlp opentelemetry_sdk grafana; do
  if cargo tree -p nexus-otel 2>/dev/null | grep -qi "$forbidden"; then
    fail "provider SDK dependency forbidden in M2: $forbidden"
  fi
done
ok "dependency-direction clean (M1 contracts + serde_json only)"

# Wire-shape canary: the authoritative proto field names must appear in
# the provider source (anti-masking: the serializer is real, not a stub).
for wire in resourceSpans scopeSpans traceId spanId startTimeUnixNano severityNumber stringValue resourceLogs resourceMetrics; do
  if ! grep -q "$wire" infra/otel/src/otlp.rs; then
    fail "authoritative wire field $wire missing from otlp.rs (stub?)"
  fi
done
ok "authoritative OTLP wire fields present in provider"

# clippy + fmt.
if ! sh -c 'cargo clippy -p nexus-otel -p nexus-observability-tests --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"
if ! sh -c 'cargo fmt -p nexus-otel -p nexus-observability-tests -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# M1 regression: the contract layer must stay green.
if ! sh scripts/ep038-m1-tests.sh > /tmp/ep038-m2-m1regress.log 2>&1; then
  fail "M1 regression failed" /tmp/ep038-m2-m1regress.log
fi
ok "M1 regression green"

echo "EP-038 M2 gate: ok"
