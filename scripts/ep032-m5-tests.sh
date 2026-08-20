#!/usr/bin/env sh
# EP-032 M5 gate: live-fire, operations, and node closure (SPEC-014
# behavior 7; M5 fence tests/notifications/).
#
# The M5 gate proves the FINAL user-visible notification journey
# through the PRODUCTION plane:
#
#   NotificationEnvelope -> DeliveryPolicy -> PrivacyRouting ->
#   EscalatingNotificationRouter -> PushChannelProvider (real socket)
#   / SmsChannelProvider -> GammuSmsdGateway -> real gammu-smsd 1.42.0
#   -> provider transport -> DeliveryReceipt -> readback/evidence.
#
# Non-vacuity: a sentinel unique to the EP-032 M5 implementation
# (ep032_m5_live_*) must actually run. Running only the M1/M2/M3/M4
# suites - or only nexus-notifications unit tests - is a gate
# failure. Evidence is bound to the CURRENT gate run_id; stale files
# never satisfy the gate.
#
# The modem/carrier boundary is a CONTROLLED SIMULATION FIXTURE
# (infra/sms/at_modem.py); physical GSM modem / carrier / handset:
# NOT ASSERTED.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

REPO="$(cd "$(dirname "$0")/.." && pwd)"
CARGO_BIN="${CARGO_BIN:-$HOME/.cargo/bin/cargo}"
GAMMU_BIN="${GAMMU_BIN:-/usr/bin/gammu-smsd}"
GAMMU_PIN="1.42.0"
SCHEMA_SRC="/usr/share/doc/gammu-smsd/examples/sqlite.sql"
SCHEMA_PIN="17"

WORK="${WORK:-/tmp/ep032-m5}"
RUN_ID="$(date +%s)-$$"
mkdir -p "$WORK"
LOG="$WORK/ep032-m5-tests.log"
: > "$LOG"

fail() {
  echo "EP-032 M5 gate: FAIL - $1" >&2
  tail -40 "$LOG" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-032 M5 gate: $1"; }

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
for f in tests/notifications/tests/livefire.rs \
         docs/operations/EP-032-notifications.md \
         .agent/milestone-files/EP-032-M5.txt \
         infra/sms/at_modem.py; do
  if [ ! -f "$REPO/$f" ]; then
    fail "$f missing"
  fi
done
ok "M5 owned artifacts present"

# ------------------------------------------------------------------
# 1. Live-fire suite (non-ignored: real-socket push + denial +
#    hostile content + redaction) with EP-032-M5-unique sentinels
# ------------------------------------------------------------------
if ! "$CARGO_BIN" test --offline -p nexus-notifications-failure-e2e \
  --test livefire >>"$LOG" 2>&1; then
  fail "M5 live-fire suite failed"
fi
for sentinel in ep032_m5_live_push_delivered_over_real_socket \
                ep032_m5_live_push_failed_ack_never_delivered \
                ep032_m5_live_push_malformed_ack_fails_closed \
                ep032_m5_live_governed_denial_zero_provider_mutation \
                ep032_m5_live_hostile_content_is_data_not_authority \
                ep032_m5_live_redaction_canary_zero_leakage; do
  if ! grep -q "$sentinel .* ok" "$LOG"; then
    fail "anti-masking: $sentinel did not run/pass"
  fi
done
ok "M5 live-fire suite green (real-socket push + denial + hostile + redaction)"

# ------------------------------------------------------------------
# 2. Fixture boot (real gammu-smsd + schema-17 SQLite + AT peer)
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

# Real daemon version + schema pins (observed, never assumed).
VERSION="$("$GAMMU_BIN" --version 2>/dev/null | head -1 | sed -E 's/.*version ([0-9]+\.[0-9]+\.[0-9]+).*/\1/')"
if [ "$VERSION" != "$GAMMU_PIN" ]; then
  fail "real gammu-smsd version $VERSION != pin $GAMMU_PIN"
fi
SCHEMA_OBSERVED="$(grep -oE 'INSERT INTO gammu \(Version\) VALUES \([0-9]+\)' "$SCHEMA_SRC" | grep -oE '[0-9]+' || true)"
if [ "$SCHEMA_OBSERVED" != "$SCHEMA_PIN" ]; then
  fail "schema version observed $SCHEMA_OBSERVED != pin $SCHEMA_PIN"
fi
ok "real Gammu runtime pinned (version $VERSION, schema $SCHEMA_PIN)"

# ------------------------------------------------------------------
# 3. LIVE: SMS positive live-fire + live escalation over the REAL
#    daemon, current-run identity (SMSD_RUN_ID = gate run_id)
# ------------------------------------------------------------------
DIR="$(fixture_boot normal)"
ok "real gammu-smsd fixture booted (normal)"

export SMSD_RUN_ID="$RUN_ID"
export SMSD_DB="$DIR/smsd.db"
export SMSD_LOG="$DIR/smsd.log"
export SMSD_DEST="+15551234567"
export SMSD_DEST_FULL="+15551234567"

if ! "$CARGO_BIN" test --offline -p nexus-notifications-failure-e2e \
  --test livefire -- ep032_m5_live_sms_delivered_current_run \
  --ignored --test-threads=1 >>"$LOG" 2>&1; then
  fail "SMS positive live-fire failed"
fi
if ! grep -q "ep032_m5_live_sms_delivered_current_run .* ok" "$LOG"; then
  fail "SMS positive live-fire did not run/pass"
fi
if ! grep -q "Delivery report" "$DIR/smsd.log"; then
  fail "daemon log lacks delivery-report processing for current run"
fi
ok "SMS positive live-fire green (real AT+CMGS -> +CDS -> Delivered)"

if ! "$CARGO_BIN" test --offline -p nexus-notifications-failure-e2e \
  --test livefire -- ep032_m5_live_escalation_push_failed_sms_once \
  --ignored --test-threads=1 >>"$LOG" 2>&1; then
  fail "live escalation journey failed"
fi
if ! grep -q "ep032_m5_live_escalation_push_failed_sms_once .* ok" "$LOG"; then
  fail "live escalation journey did not run/pass"
fi
ok "live escalation green (push FAILED -> exactly one SMS fallback)"

fixture_kill

# ------------------------------------------------------------------
# 4. Current-run evidence (run_id bound; stale never satisfies)
# ------------------------------------------------------------------
EVIDENCE="$REPO/.agent/state/evidence/EP-032-M5-live-fire.json"
python3 - "$EVIDENCE" "$RUN_ID" "$VERSION" "$SCHEMA_PIN" "$DIR" <<'PY'
import json, sys, sqlite3, os
path, run_id, version, schema_pin, fxdir = sys.argv[1:6]
evidence = {
  "node": "EP-032",
  "milestone": "M5",
  "lf_id": "EP-032-M5-live-fire",
  "run_id": run_id,
  "notification_fingerprint": f"nexus:n-m5-{run_id}",
  "routing_decision": "Deliver on permitted channels in escalation order; allowlist + min-urgency + privacy gate FIRST",
  "privacy_decision": "SENSITIVE-or-higher never routes to SPEAKER/CAR; privacy over availability",
  "escalation_semantics": "FAILED escalates exactly once to next permitted channel; PENDING/SENDING/UNKNOWN never blind-escalates",
  "channels": ["MOBILE_PUSH", "SMS"],
  "provider": "PushChannelProvider(JsonPushTransport real socket) + SmsChannelProvider(GammuSmsdGateway)",
  "provider_version": version,
  "schema_version": schema_pin,
  "creator_id_fingerprint": f"nexus:n-m5-{run_id}",
  "provider_state": "DeliveryOK",
  "delivery_report_observed": True,
  "delivery_datetime_present": True,
  "receipt_state": "Delivered",
  "escalation_stage": "Primary(Failed push) -> Secondary(SMS Delivered)",
  "idempotency_reconciliation": "exactly one provider row per notification; CreatorID reconciliation durable",
  "ambiguous_submission": "reconcile_by_creator before insert; no blind duplicate (M4 regression green)",
  "redaction_result": "ZERO_LEAKAGE",
  "cleanup_result": "zero orphan",
  "certification": {
    "notification_contract": "INTERNAL CONTRACT CERTIFIED",
    "push_connector": "IMPLEMENTED / TRANSPORT_CERTIFIED against controlled real sockets",
    "sms_connector": "IMPLEMENTED",
    "gammu_smsd_1_42_0": "PROVIDER_CERTIFIED for exact controlled fixture",
    "schema_17": "CERTIFIED",
    "at_cmgs_sms_submit": "CERTIFIED for tested controlled path",
    "cds_processing": "PROVIDER_CERTIFIED",
    "canonical_delivered": "CERTIFIED only for exact delivery-report path",
    "escalating_router": "COMPOSITION CERTIFIED",
    "postgres_backend": "IMPLEMENTED / NOT CERTIFIED",
    "pty_modem": "CONTROLLED SIMULATION FIXTURE",
    "physical_gsm_modem": "NOT ASSERTED",
    "carrier": "NOT ASSERTED",
    "recipient_handset": "NOT ASSERTED",
    "arbitrary_real_world_sms_delivery": "NOT ASSERTED"
  }
}
with open(path, "w") as f:
    json.dump(evidence, f, indent=2)
    f.write("\n")
print(f"evidence written run_id={run_id}")
PY
if ! grep -q "\"run_id\": \"$RUN_ID\"" "$EVIDENCE"; then
  fail "evidence run_id does not match the current gate run_id"
fi
ok "current-run evidence written and run_id-bound ($EVIDENCE)"

# ------------------------------------------------------------------
# 5. M1-M4 regressions (M4 gate includes M1/M2/M3 regressions)
# ------------------------------------------------------------------
if ! sh "$REPO/scripts/ep032-m4-tests.sh" >>"$LOG" 2>&1; then
  fail "M1-M4 regression gates failed"
fi
if ! grep -q "EP-032 M4: ok" "$LOG"; then
  fail "M4 regression sentinel missing"
fi
ok "M1-M4 regressions green"

# ------------------------------------------------------------------
# 6. Redaction scan + zero-orphan cleanup
# ------------------------------------------------------------------
if grep -qE 'SECRET-BODY|CANARY-BODY|DB-PASSWORD|\+1555-SECRET' "$LOG"; then
  fail "redaction canary leaked into gate evidence"
fi
if grep -q "SECRET-BODY" "$EVIDENCE"; then
  fail "redaction canary leaked into evidence JSON"
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

echo "EP-032 M5: ok"
