#!/usr/bin/env sh
# EP-038 M5 gate: dashboards, final observability/operations proof, and
# node closure (SPEC-007; node contract; ExecPlan M5).
#
# M5 owns dashboards/ (nexus-dashboards validated Grafana-format config
# backed by M1/M4 catalog semantics) plus node closure. The gate proves:
#   - dashboards/ material present (3 real dashboards + README + ops doc)
#   - anti-phantom: no phantom runner delegation (the gate runs the
#     real suites directly; no external runner indirection)
#   - the REAL dashboard validator runs and every dashboards/*.json
#     validates against the canonical catalog (CLI exit 0)
#   - nexus-dashboards unit proofs green (13 tests, zero ignored)
#   - anti-masking: EP-038-owned ep038_m5_* sentinels observed
#   - dashboard JSON parses as real Grafana-format documents
#     (uid/title/panels/datasource/templating/targets) - proven by the
#     validator model + tests, plus a raw JSON syntax check here
#   - green-on-no-data is impossible (validator rejects it; negative
#     proof in tests)
#   - no secret-shaped literals in dashboards (validator scan + grep)
#   - current-run evidence freshness + node/milestone/run_id binding +
#     redaction scan
#   - expected-files EP-038 full list green (dashboards/ closes the list)
#   - M1+M2+M3+M4 regressions green
#   - clippy -D warnings + fmt clean on owned crates
#   - orphan/resource guard: zero EP-038-owned residue
set -eu
export CI=true
export CARGO_TERM_COLOR=never
umask 077

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

OWN="nexus-ep038m5"
LOG="/tmp/${OWN}-gate.log"
EVID="/tmp/${OWN}-evidence.json"
: > "$LOG"

fail() {
  echo "EP-038 M5 gate: FAIL - $1" >&2
  tail -40 "${2:-$LOG}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-038 M5 gate: $1"; }

# ------------------------------------------------------------- teardown
cleanup() {
  # M5 starts no containers; the M1-M4 regression gates own their own
  # fixtures and clean up. This trap only removes M5 temp files.
  rm -f "$EVID"
  rm -f /tmp/ep038-m5-*.json
  rm -rf /tmp/nexus-ep038m5-*
}
trap cleanup EXIT

# ------------------------------------------------------------- material
for f in Cargo.toml src/lib.rs src/bin/dashboard-validate.rs tests/ep038_m5_dashboards.rs README.md operations.md; do
  if [ ! -f "dashboards/$f" ]; then
    fail "dashboards/$f missing"
  fi
done
for f in nexus-health-overview.json nexus-incidents-slo.json nexus-metrics-ops.json; do
  if [ ! -f "dashboards/$f" ]; then
    fail "dashboards/$f missing"
  fi
done
ok "dashboards/ material present (3 dashboards + crate + docs)"

# Anti-phantom: the node branch must run this gate directly, never
# delegate to a phantom runner.
if grep -q "proof-runner\|nexus-cli" scripts/nodes/EP-038.sh 2>/dev/null; then
  fail "scripts/nodes/EP-038.sh still references the phantom proof-runner/nexus-cli"
fi
ok "no proof-runner/nexus-cli references"

# No placeholder content in owned docs.
if grep -qiE 'placeholder|TODO|FIXME|sample only|example only' dashboards/README.md dashboards/operations.md; then
  fail "dashboards/ docs contain placeholder content"
fi
ok "dashboards/ docs are real content"

# Raw JSON syntax check for every dashboard (independent of the Rust
# validator; both must pass).
for f in dashboards/*.json; do
  python3 -c "import json,sys; json.load(open('$f'))" || fail "$f is not valid JSON"
done
ok "all dashboard JSON files parse"

# ------------------------------------------------------ real validation
if ! sh -c 'cargo test -p nexus-dashboards --locked >> "$1" 2>&1' _ "$LOG"; then
  fail "nexus-dashboards test suite failed" "$LOG"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$LOG"; then
  fail "no dashboard tests ran (vacuity guard)" "$LOG"
fi
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$LOG"; then
  fail "observed failed dashboard tests (vacuity guard)" "$LOG"
fi
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$LOG"; then
  fail "required dashboard tests were ignored (vacuity guard)" "$LOG"
fi
ok "nexus-dashboards unit proofs green"

# Anti-masking: every owned proof observed in the log.
for sentinel in \
  ep038_m5_catalog_contains_m4_ops_metrics \
  ep038_m5_catalog_contains_m1_canonical_and_alert_slo_ids \
  ep038_m5_valid_dashboard_passes \
  ep038_m5_all_real_dashboards_validate \
  ep038_m5_contract_metric_builds_valid_definitions \
  ep038_m5_rejects_green_on_nodata \
  ep038_m5_rejects_unknown_metric \
  ep038_m5_rejects_secret_shaped_literal \
  ep038_m5_rejects_missing_required_fields \
  ep038_m5_rejects_no_panels \
  ep038_m5_rejects_high_cardinality_raw_label \
  ep038_m5_rejects_empty_expr \
  ep038_m5_rejects_malformed_json_file \
; do
  if ! grep -q "$sentinel" "$LOG"; then
    fail "EP-038-owned proof $sentinel did not run (anti-masking)" "$LOG"
  fi
done
ok "EP-038-owned M5 proofs observed (13/13)"

# The REAL validator CLI must pass over the real files.
if ! sh -c 'cargo run -p nexus-dashboards --locked --bin dashboard-validate > /tmp/ep038-m5-validate.out 2>>"$1"' _ "$LOG"; then
  fail "dashboard-validate CLI failed" /tmp/ep038-m5-validate.out
fi
grep -q "dashboard validate: ok" /tmp/ep038-m5-validate.out || fail "dashboard-validate did not report ok" /tmp/ep038-m5-validate.out
grep -q "3 dashboards validated" /tmp/ep038-m5-validate.out || fail "dashboard-validate did not validate 3 dashboards" /tmp/ep038-m5-validate.out
ok "dashboard-validate CLI green over real dashboards (3/3)"

# Redaction scan of dashboard JSON (defense in depth; validator does it
# too, this is an independent raw-text scan).
if grep -qEi 'AKIA[0-9A-Z]{16}|BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY|ghp_[0-9A-Za-z]{20,}|sk-[A-Za-z0-9]{20,}|dsn[=: ].{24,}' dashboards/*.json dashboards/*.md; then
  fail "secret-shaped literal found in dashboards/"
fi
ok "dashboards/ redaction scan clean"

# ------------------------------------------------------- current-run evidence
run_id="ep038-m5-$(date +%s)-$(od -An -N4 -tx4 /dev/urandom | tr -d ' \n' | cut -c1-8)"
git_commit=$(/usr/bin/git rev-parse HEAD 2>/dev/null || echo unknown)
cat > "$EVID" <<EOF
{
  "node": "EP-038",
  "milestone": "M5",
  "lf_id": "EP-038-M5",
  "run_id": "$run_id",
  "git_commit": "$git_commit",
  "dashboard_count": 3,
  "dashboard_validation": "ALL_VALIDATED",
  "unit_proofs": 13,
  "green_on_nodata_rejected": true,
  "redaction_scan": "CLEAN",
  "certification_boundary": {
    "dashboards_config": "VALIDATED (syntax, required Grafana fields, catalog-backed selectors, redaction-safe, no green-on-no-data)",
    "grafana_server": "NOT ASSERTED - no real Grafana exercised",
    "prometheus_server": "NOT ASSERTED - no real Prometheus exercised",
    "otel_collector": "NOT ASSERTED",
    "loki_tempo_jaeger": "NOT ASSERTED",
    "pagerduty_slack_email": "NOT ASSERTED",
    "fleet_telemetry": "NOT ASSERTED",
    "production_monitoring": "NOT ASSERTED"
  }
}
EOF
cp "$EVID" .agent/state/evidence/EP-038-M5-dashboards.json
age=$(( $(date +%s) - $(stat -c %Y .agent/state/evidence/EP-038-M5-dashboards.json) ))
if [ "$age" -gt 120 ]; then
  fail "evidence write did not refresh (age ${age}s)"
fi
grep -q '"node": "EP-038"' .agent/state/evidence/EP-038-M5-dashboards.json || fail "evidence not node-bound"
grep -q '"milestone": "M5"' .agent/state/evidence/EP-038-M5-dashboards.json || fail "evidence not milestone-bound"
grep -q '"run_id"' .agent/state/evidence/EP-038-M5-dashboards.json || fail "evidence missing run_id"
if grep -qi "secret_key\|access_key\|password\|dsn\|token" .agent/state/evidence/EP-038-M5-dashboards.json; then
  fail "evidence leaks credential-shaped content"
fi
ok "current-run evidence fresh + node/milestone/run_id bound + redacted"

# ------------------------------------------------------- expected-files
if ! sh scripts/expected-files.sh EP-038 > /tmp/ep038-m5-expected.log 2>&1; then
  fail "expected-files EP-038 failed (dashboards/ must close the list)" /tmp/ep038-m5-expected.log
fi
ok "expected-files EP-038 full list green"

# ------------------------------------------------------- M1-M4 regressions
if ! sh scripts/ep038-m1-tests.sh > /tmp/ep038-m5-m1regress.log 2>&1; then
  fail "M1 regression failed" /tmp/ep038-m5-m1regress.log
fi
if ! sh scripts/ep038-m2-tests.sh > /tmp/ep038-m5-m2regress.log 2>&1; then
  fail "M2 regression failed" /tmp/ep038-m5-m2regress.log
fi
if ! sh scripts/ep038-m3-tests.sh > /tmp/ep038-m5-m3regress.log 2>&1; then
  fail "M3 regression failed" /tmp/ep038-m5-m3regress.log
fi
if ! sh scripts/ep038-m4-tests.sh > /tmp/ep038-m5-m4regress.log 2>&1; then
  fail "M4 regression failed" /tmp/ep038-m5-m4regress.log
fi
ok "M1+M2+M3+M4 regression green"

# ------------------------------------------------------- clippy + fmt
if ! sh -c 'cargo clippy -p nexus-dashboards --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$LOG"; then
  fail "clippy -D warnings failed (nexus-dashboards)" "$LOG"
fi
ok "clippy -D warnings clean (nexus-dashboards)"
if ! sh -c 'cargo fmt -p nexus-dashboards -- --check >> "$1" 2>&1' _ "$LOG"; then
  fail "cargo fmt check failed (nexus-dashboards)" "$LOG"
fi
ok "cargo fmt clean (nexus-dashboards)"

# ------------------------------------------------------- orphan guard
leftovers=$(docker ps -aq --filter "name=nexus-ep038" 2>/dev/null | wc -l)
[ "$leftovers" -eq 0 ] || fail "leftover nexus-ep038-* containers: $leftovers"
vol_left=$(docker volume ls -q --filter "name=nexus-ep038" 2>/dev/null | wc -l)
[ "$vol_left" -eq 0 ] || fail "leftover nexus-ep038-* volumes: $vol_left"
net_left=$(docker network ls -q --filter "name=nexus-ep038" 2>/dev/null | wc -l)
[ "$net_left" -eq 0 ] || fail "leftover nexus-ep038-* networks: $net_left"
ok "orphan guard clean (zero owned containers/volumes/networks)"

echo "EP-038 M5 gate: ok"
