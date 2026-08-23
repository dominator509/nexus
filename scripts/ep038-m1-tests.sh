#!/usr/bin/env sh
# EP-038 M1 gate: run the nexus-observability contract suite through the
# REAL cargo machinery with vacuity guards (EP-001 gate-masking class).
#
# M1 owns crates/nexus-observability/ (provider-neutral observability
# contract crate) and alerts/ (contract/config only). The authoritative
# gate is the crate suite plus clippy/fmt, alerts content validation,
# deny-unknown vocabulary proofs, redaction fail-closed proofs, health/
# fleet/SLO truthfulness proofs, and dependency-direction proof.
#
# Vacuous green is impossible: `cargo test -t <filter>` exits 0 on a
# zero-match filter, so a green M1 must observe real non-zero passing
# counts, EP-038-owned test names, and zero failed/ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

# Ensure cargo is available to `sh -c` subshells (the interactive alias
# is not inherited). ~/.cargo/env appends cargo's bin dir to PATH.
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

log="/tmp/ep038-m1-tests.log"
: > "$log"

fail() {
  echo "EP-038 M1 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-038 M1 gate: $1"; }

# Vacuity guard 0: the crate must exist with its owned sources.
if [ ! -f crates/nexus-observability/Cargo.toml ]; then
  fail "crates/nexus-observability/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/error.rs \
  src/vocabulary.rs \
  src/model.rs \
  src/port.rs; do
  if [ ! -f "crates/nexus-observability/$f" ]; then
    fail "crates/nexus-observability/$f missing"
  fi
done
for f in README.md catalog.yaml redaction-policy.yaml slo-catalog.yaml; do
  if [ ! -f "alerts/$f" ]; then
    fail "alerts/$f missing"
  fi
done
ok "nexus-observability crate and alerts/ M1-owned files present"

# Real test run through cargo, captured to the log for raw sentinels
# (rtk-tee compresses interactive cargo output).
if ! sh -c 'cargo test -p nexus-observability --locked >> "$1" 2>&1' _ "$log"; then
  fail "cargo test failed" "$log"
fi

# Vacuity guard 1: every suite reported a non-zero pass.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: zero failures observed.
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 3: zero ignored tests (no required test may be skipped).
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi

# Vacuity guard 4 (anti-masking): EP-038-owned contract tests observed.
# One sentinel per interface + the cross-cutting invariants.
for sentinel in \
  ep038_unit_vocabulary_deny_unknown_severity \
  ep038_unit_vocabulary_serde_rejects_unknown_wire_value \
  ep038_unit_telemetry_context_rejects_secret_shaped_field \
  ep038_unit_redaction_hashes_secret_shaped_values \
  ep038_unit_redaction_raw_payload_denied_by_default \
  ep038_unit_redaction_unclassified_fails_closed \
  ep038_unit_metric_catalog_deny_unknown_and_cardinality \
  ep038_unit_metric_catalog_rejects_unsafe_label_values \
  ep038_unit_trace_present_not_exported_not_safe \
  ep038_unit_health_configured_not_ready_and_stale_not_healthy \
  ep038_unit_health_partial_dependencies_degraded \
  ep038_unit_incident_dedupe_and_escalation \
  ep038_unit_incident_redacted_body_and_state_transitions \
  ep038_unit_fleet_stale_node_not_healthy \
  ep038_unit_fleet_unknown_critical_unsafe_to_claim \
  ep038_unit_slo_no_events_never_met; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-038-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-038-owned contract tests observed (all 8 interfaces)"

# Vacuity guard 5: dependency direction - the contract crate must depend
# only on nexus-domain, serde, serde_json, and sha2. No Prometheus,
# Grafana, OpenTelemetry SDK, Datadog, Honeycomb, Sentry, Loki, Tempo,
# Jaeger, or cloud SDKs in M1.
bad_dep=$(cargo tree -p nexus-observability --depth 1 2>/dev/null | grep -vE 'nexus-observability|nexus-domain|serde|serde_json|sha2' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-observability: $bad_dep"
fi
for forbidden in prometheus grafana opentelemetry datadog honeycomb sentry loki tempo jaeger aws-sdk azure google-cloud; do
  if cargo tree -p nexus-observability 2>/dev/null | grep -qi "$forbidden"; then
    fail "provider SDK dependency forbidden in M1: $forbidden"
  fi
done
ok "dependency-direction clean (nexus-domain + serde + sha2 only)"

# Vacuity guard 6: alerts/ contract content is real (not placeholder).
if grep -qiE 'placeholder|TODO|fake|sample only' alerts/catalog.yaml alerts/redaction-policy.yaml alerts/slo-catalog.yaml; then
  fail "alerts/ contains placeholder content"
fi
if ! grep -q "nexus.storage.provider_unavailable" alerts/catalog.yaml; then
  fail "alerts/catalog.yaml missing canonical rule"
fi
if ! grep -q "fail_closed: true" alerts/redaction-policy.yaml; then
  fail "alerts/redaction-policy.yaml missing fail-closed invariant"
fi
ok "alerts/ contract content validated"

# Clippy -D warnings and fmt on the owned crate.
if ! sh -c 'cargo clippy -p nexus-observability --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-observability -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

echo "EP-038 M1 gate: ok"
