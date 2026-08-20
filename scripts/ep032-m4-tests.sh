#!/usr/bin/env sh
# EP-032 M4 gate: forced failures, abuse cases, observability, recovery.
#
# The M4 fence is tests/notifications/ + connectors/sms failure
# additions + infra/sms fixture modes + scripts/ep032-m4-tests.sh +
# scripts/sms-diag.sh + node script + plan files. The authoritative
# proof: the production notification plane (PUSH + SMS) forced through
# ambiguity, provider/backend failure, privacy abuse, duplicate/
# restart conditions, truthful escalation, redaction, and recovery -
# with the REAL gammu-smsd 1.42.0 fixture for every provider-behavior
# claim.
#
# The modem/carrier boundary is a CONTROLLED SIMULATION FIXTURE
# (infra/sms/at_modem.py); physical GSM modem / carrier / handset:
# NOT ASSERTED. A host-side skipped run never satisfies certification.
#
# Anti-masking: the gate requires an EP-032-M4-unique sentinel
# (ep032_failure_* tests) to actually run; running only the M1/M2/M3
# suites is a gate failure.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

REPO="$(cd "$(dirname "$0")/.." && pwd)"
CARGO_BIN="${CARGO_BIN:-$HOME/.cargo/bin/cargo}"
GAMMU_BIN="${GAMMU_BIN:-/usr/bin/gammu-smsd}"
GAMMU_PIN="1.42.0"
SCHEMA_SRC="/usr/share/doc/gammu-smsd/examples/sqlite.sql"
SCHEMA_PIN="17"

WORK="${WORK:-/tmp/ep032-m4}"
RUN_ID="$(date +%s)-$$"
mkdir -p "$WORK"
LOG="$WORK/ep032-m4-tests.log"
: > "$LOG"

fail() {
  echo "EP-032 M4 gate: FAIL - $1" >&2
  tail -40 "$LOG" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-032 M4 gate: $1"; }

DAEMON_PID=""
AT_PID=""
SOCAT_PID=""
CURRENT_DB=""

cleanup() {
  if [ -n "$DAEMON_PID" ]; then
    kill "$DAEMON_PID" 2>/dev/null || true
    sleep 0.3
    kill -9 "$DAEMON_PID" 2>/dev/null || true
  fi
  [ -n "$AT_PID" ] && kill -9 "$AT_PID" 2>/dev/null || true
  [ -n "$SOCAT_PID" ] && kill -9 "$SOCAT_PID" 2>/dev/null || true
  pkill -9 -f "gammu-smsd.*$WORK" 2>/dev/null || true
  pkill -9 -f "at_modem.py $WORK" 2>/dev/null || true
  pkill -9 -f "socat.*$WORK" 2>/dev/null || true
  if [ -n "$CURRENT_DB" ]; then
    chmod 644 "$CURRENT_DB" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

# ------------------------------------------------------------------
# 0. Required artifacts present
# ------------------------------------------------------------------
for f in tests/notifications/Cargo.toml scripts/sms-diag.sh \
         infra/sms/at_modem.py connectors/sms/Cargo.toml; do
  if [ ! -f "$REPO/$f" ]; then
    fail "$f missing"
  fi
done
ok "M4 owned artifacts present"

# ------------------------------------------------------------------
# 1. Contract/unit phase: failure e2e crate + connector suite
# ------------------------------------------------------------------
if ! "$CARGO_BIN" test --offline -p nexus-notifications-failure-e2e --all-targets >>"$LOG" 2>&1; then
  fail "nexus-notifications-failure-e2e failed" "$LOG"
fi
if ! grep -qE 'ep032_failure_.* ok' "$LOG"; then
  fail "no ep032_failure_* e2e test ran (anti-masking)" "$LOG"
fi
E2E_COUNT="$(grep -oE 'test result: ok\. [0-9]+ passed' "$LOG" | awk '{s+=$4} END {print s}')"
ok "notification failure e2e suite green ($E2E_COUNT tests)"

if ! "$CARGO_BIN" test --offline -p nexus-sms-connector --lib >>"$LOG" 2>&1; then
  fail "nexus-sms-connector unit suite failed" "$LOG"
fi
if ! grep -q 'ep032_failure_unit_sms_gateway_submit_reconciled_no_blind_duplicate .* ok' "$LOG"; then
  fail "ambiguous-submission unit proof did not run" "$LOG"
fi
if ! grep -q 'ep032_failure_unit_sms_db_schema_drift_fails_closed .* ok' "$LOG"; then
  fail "schema-drift fail-closed proof did not run" "$LOG"
fi
ok "connector unit suite green (incl. ambiguity + schema drift units)"

# ------------------------------------------------------------------
# 2. Fixture boot helper (fresh schema-17 DB + socat PTY + AT peer)
# ------------------------------------------------------------------
fixture_boot() {
  MODE="$1"
  DIR="$WORK/fx-$MODE-$RUN_ID"
  rm -rf "$DIR"
  mkdir -p "$DIR"
  cp "$SCHEMA_SRC" "$DIR/sqlite.sql"
  sqlite3 "$DIR/smsd.db" < "$DIR/sqlite.sql" 2>>"$LOG" || python3 - "$DIR/smsd.db" "$DIR/sqlite.sql" >>"$LOG" 2>&1 <<'PY'
import sqlite3, sys
conn = sqlite3.connect(sys.argv[1])
conn.executescript(open(sys.argv[2]).read())
conn.commit()
PY

  socat -d -d "pty,raw,echo=0,link=$DIR/modem" \
    "pty,raw,echo=0,link=$DIR/modem-ctrl" >"$DIR/socat.log" 2>&1 &
  SOCAT_PID=$!
  sleep 0.5
  [ -e "$DIR/modem" ] || fail "socat modem link missing for $MODE"

  MODE_ENV=""
  [ "$MODE" = "no-report" ] && MODE_ENV="SMSD_NO_REPORT=1"
  [ "$MODE" = "fail-report" ] && MODE_ENV="SMSD_FAILURE_REPORT=1"
  [ "$MODE" = "unmatched-report" ] && MODE_ENV="SMSD_UNMATCHED_REPORT=1"
  # shellcheck disable=SC2086
  env $MODE_ENV python3 "$REPO/infra/sms/at_modem.py" "$DIR/modem-ctrl" >"$DIR/at_modem.log" 2>&1 &
  AT_PID=$!
  sleep 0.5
  kill -0 "$AT_PID" 2>/dev/null || fail "AT peer failed to start for $MODE"

  cat > "$DIR/smsdrc" <<EOF
[gammu]
port = $DIR/modem
connection = at115200
model = AT
logformat = text

[smsd]
service = sql
driver = sqlite3
dbdir = $DIR
database = smsd.db
logfile = $DIR/smsd.log
debuglevel = 255
commtimeout = 1
sendtimeout = 2
loopsleep = 0
maxretries = 1
skipsmscnumber = yes
EOF

  nohup "$GAMMU_BIN" -c "$DIR/smsdrc" >"$DIR/smsd-console.log" 2>&1 &
  DAEMON_PID=$!
  sleep 1.5
  kill -0 "$DAEMON_PID" 2>/dev/null || fail "gammu-smsd failed to start for $MODE"
  CURRENT_DB="$DIR/smsd.db"
  echo "$DIR"
}

fixture_kill() {
  [ -n "$DAEMON_PID" ] && kill "$DAEMON_PID" 2>/dev/null || true
  sleep 0.5
  [ -n "$DAEMON_PID" ] && kill -9 "$DAEMON_PID" 2>/dev/null || true
  DAEMON_PID=""
  [ -n "$AT_PID" ] && kill -9 "$AT_PID" 2>/dev/null || true
  AT_PID=""
  [ -n "$SOCAT_PID" ] && kill -9 "$SOCAT_PID" 2>/dev/null || true
  SOCAT_PID=""
  CURRENT_DB=""
  sleep 0.3
}

run_live() {
  NAME="$1"
  DIR="$2"
  export SMSD_RUN_ID="$RUN_ID"
  export SMSD_DB="$DIR/smsd.db"
  export SMSD_LOG="$DIR/smsd.log"
  export SMSD_DEST="+15551234567"
  export SMSD_DEST_FULL="+15551234567"
  if ! "$CARGO_BIN" test --offline -p nexus-sms-connector \
    --test ep032_failure_smsd -- "$NAME" --ignored --test-threads=1 >>"$LOG" 2>&1; then
    fail "live failure test $NAME failed" "$LOG"
  fi
  if ! grep -q "$NAME .* ok" "$LOG"; then
    fail "live failure test $NAME did not run/pass" "$LOG"
  fi
  ok "$NAME green"
}

# ------------------------------------------------------------------
# 3. LIVE phase A: normal fixture - ambiguity, durable idempotency,
#    daemon unavailable, backend unavailable, provider restart
# ------------------------------------------------------------------
DIR="$(fixture_boot normal)"
ok "real gammu-smsd fixture booted (normal)"

run_live ep032_failure_ambiguous_submission_reconciles_exactly_one_row "$DIR"
run_live ep032_failure_durable_idempotency_across_process_restart "$DIR"

# Daemon unavailable: stop the real daemon; queue acceptance must
# never fabricate Delivered.
fixture_kill
run_live ep032_failure_daemon_unavailable_truthful_failure_no_fake_delivered "$DIR"

# Backend unavailable: replace the DB file with a directory so a
# fresh open fails (SQLite: unable to open database file) - a genuine
# backend-unavailable, canonical fail-closed path.
mv "$DIR/smsd.db" "$DIR/smsd.db.real"
mkdir "$DIR/smsd.db"
run_live ep032_failure_backend_unavailable_fails_closed_then_recovers "$DIR"
rmdir "$DIR/smsd.db"
mv "$DIR/smsd.db.real" "$DIR/smsd.db"

# Provider restart: reboot the real daemon; a new connector instance
# reconciles the exact same durable identity - no duplicate row.
DIR="$(fixture_boot restart)"
ok "real gammu-smsd restarted"
run_live ep032_failure_provider_restart_reconciles_and_recovers "$DIR"
fixture_kill

# ------------------------------------------------------------------
# 4. LIVE phase B: no delivery report (SendingOK, no +CDS)
# ------------------------------------------------------------------
DIR="$(fixture_boot no-report)"
run_live ep032_failure_no_delivery_report_never_delivered "$DIR"
fixture_kill

# ------------------------------------------------------------------
# 5. LIVE phase C: real failure delivery report (+CDS TP-Status 0x29)
# ------------------------------------------------------------------
DIR="$(fixture_boot fail-report)"
run_live ep032_failure_delivery_failure_report_maps_to_failed "$DIR"
fixture_kill

# ------------------------------------------------------------------
# 6. LIVE phase D: unmatched delivery report (exact-target)
# ------------------------------------------------------------------
DIR="$(fixture_boot unmatched-report)"
run_live ep032_failure_unmatched_report_never_satisfies_target "$DIR"
fixture_kill

# ------------------------------------------------------------------
# 7. Push regression (M2) + M1/M2/M3 regressions
# ------------------------------------------------------------------
if ! "$CARGO_BIN" test --offline -p nexus-push-connector --all-targets >>"$LOG" 2>&1; then
  fail "M2 push regression failed" "$LOG"
fi
if ! grep -q 'ep032_unit_push_transport_malformed_ack_fails_closed .* ok' "$LOG"; then
  fail "push malformed-ack regression did not run" "$LOG"
fi
if ! grep -q 'ep032_unit_push_transport_peer_closed_fails_closed .* ok' "$LOG"; then
  fail "push closed-peer regression did not run" "$LOG"
fi
ok "M2 push regression green (malformed ack + closed peer + truthfulness)"

if ! "$CARGO_BIN" test --offline -p nexus-notifications --all-targets >>"$LOG" 2>&1; then
  fail "M1 regression failed" "$LOG"
fi
if ! grep -q 'ep032_unit_envelope_constructs_valid .* ok' "$LOG"; then
  fail "M1 contract regression did not run" "$LOG"
fi
ok "M1 contract regression green"

# M3 regression: full real daemon lifecycle gate.
if ! sh "$REPO/scripts/ep032-m3-tests.sh" >>"$LOG" 2>&1; then
  fail "M3 regression gate failed" "$LOG"
fi
ok "M3 regression green (real Gammu lifecycle)"

# ------------------------------------------------------------------
# 8. Ops diagnostic: truthful classification (never config-only)
# ------------------------------------------------------------------
if ! "$REPO/scripts/sms-diag.sh" -c "$DIR/smsdrc" -d "$DIR/smsd.db" >>"$LOG" 2>&1; then
  fail "sms-diag failed on a healthy fixture" "$LOG"
fi
if ! grep -q "provider_db=yes" "$LOG"; then
  fail "sms-diag did not assert provider_db" "$LOG"
fi
# A missing config must fail closed (rc=3), never report healthy.
MISSING="$WORK/missing-smsdrc"
rm -f "$MISSING"
if "$REPO/scripts/sms-diag.sh" -c "$MISSING" >>"$LOG" 2>&1; then
  fail "sms-diag must fail closed when config is missing"
fi
ok "sms-diag truthful classification green (healthy never from config alone)"

# ------------------------------------------------------------------
# 9. Redaction + orphan guard
# ------------------------------------------------------------------
if grep -qE 'SECRET-BODY|CANARY-BODY|\+1555-SECRET|DB-PASSWORD' "$LOG"; then
  fail "redaction canary leaked into gate log"
fi
ok "redaction zero leakage across gate evidence"

cleanup
sleep 0.5
if pgrep -f "gammu-smsd.*$WORK" >/dev/null 2>&1; then
  fail "orphan gammu-smsd remains"
fi
if pgrep -f "at_modem.py $WORK" >/dev/null 2>&1; then
  fail "orphan AT peer remains"
fi
if pgrep -f "socat.*$WORK" >/dev/null 2>&1; then
  fail "orphan socat remains"
fi
ok "zero-orphan cleanup green"

echo "EP-032 M4: ok"
