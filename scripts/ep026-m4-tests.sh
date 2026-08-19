#!/usr/bin/env sh
# EP-026 M4 gate: real IMAP/SMTP failure plane, abuse cases,
# observability, and mail recovery.
#
# Provisions the real GreenMail fixture (pinned digest), runs the
# real-socket integration suite with --ignored, tears the fixture
# down, audits zero-orphan state, and scans the produced evidence for
# secret canaries. Explicit vacuity guards reject any zero-test green
# or any missing proof class.
#
# Emits ONLY on full observed success:
#   EP-026 M4: ok
set -eu

REPO="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO"
LOG=/tmp/ep026-m4-gate.log
: > "$LOG"
echo "== EP-026 M4 gate start $(date -u +%FT%TZ)" >> "$LOG"

step() { echo "[m4-gate] $*" | tee -a "$LOG"; }
fail() { echo "EP-026 M4: FAIL ($*)" | tee -a "$LOG"; exit 1; }

# 0. Fixture lifecycle: provision, run, teardown (always torn down).
sh infra/mail/provision.sh >> "$LOG" 2>&1 || fail "fixture provision"
FIXTURE_UP=1
cleanup() {
  sh infra/mail/teardown.sh >> "$LOG" 2>&1 || true
}
trap cleanup EXIT

set -a
. /tmp/ep026-mail.env
set +a

# 1. Run the real-socket integration suite (serial: shared fixture,
#    restart test mutates provider state).
/root/.cargo/bin/cargo test -p nexus-imap-smtp --test ep026_m4_mail \
  -- --ignored --test-threads=1 >> "$LOG" 2>&1 || fail "integration suite"
TEST_LOG="$LOG"

# 2. Guard: tests were actually collected and ran (vacuity guard 1).
grep -q "running .* tests" "$TEST_LOG" || fail "no tests collected"
grep -q "test result: ok" "$TEST_LOG" || fail "suite did not pass"

# 3. Guard: real SMTP socket test ran (positive path) (guard 3).
grep -q "m4_smtp_positive_canary_full_chain.*ok$" "$TEST_LOG" \
  || fail "real SMTP socket positive path did not run"
# 4. Guard: real IMAP socket test ran (guard 4).
grep -q "m4_imap_positive_canary_exact_target.*ok$" "$TEST_LOG" \
  || fail "real IMAP socket positive path did not run"
# 5. Guard: real authentication failure ran (guard 5).
grep -q "m4_smtp_auth_failure_no_success.*ok$" "$TEST_LOG" \
  || fail "SMTP auth failure proof did not run"
grep -q "m4_imap_auth_failure.*ok$" "$TEST_LOG" \
  || fail "IMAP auth failure proof did not run"
# 6. Guard: real timeout ran (guard 6).
grep -q "m4_smtp_timeout_silent_peer.*ok$" "$TEST_LOG" \
  || fail "SMTP timeout proof did not run"
grep -q "m4_imap_timeout_silent_peer.*ok$" "$TEST_LOG" \
  || fail "IMAP timeout proof did not run"
# 7. Guard: ambiguous-send proof ran (guard 7).
grep -q "m4_smtp_ambiguous_no_blind_retry.*ok$" "$TEST_LOG" \
  || fail "ambiguous-send proof did not run"
# 8. Guard: hostile-content test ran (guard 8).
grep -q "m4_hostile_content_no_authority.*ok$" "$TEST_LOG" \
  || fail "hostile-content proof did not run"
# 9. Guard: redaction test ran (guard 9).
grep -q "m4_redaction_canary_no_leak.*ok$" "$TEST_LOG" \
  || fail "redaction-canary proof did not run"
# 10. Guard: restart/recovery ran (guard 10).
grep -q "m4_restart_recovery.*ok$" "$TEST_LOG" \
  || fail "restart/recovery proof did not run"

# 11. Redaction scan of produced evidence (directive Z): no fixture
#     credential or canary value leaked into the gate log. The
#     fixture env intentionally contains credentials; the log must not.
CANARY_PW_A="$EP026_MAIL_PASS_A"
CANARY_PW_B="$EP026_MAIL_PASS_B"
if grep -qF "$CANARY_PW_A" "$LOG" || grep -qF "$CANARY_PW_B" "$LOG"; then
  fail "fixture credential leaked into gate evidence"
fi
if grep -qF "EP026M4PW_CANARY_5d" "$LOG"; then
  fail "password canary leaked into gate evidence"
fi

# 12. Unit battery (authority matrix, limiter, redaction, MIME).
/root/.cargo/bin/cargo test -p nexus-imap-smtp --lib >> "$LOG" 2>&1 \
  || fail "unit battery"
grep -q "12 passed" "$LOG" || fail "unit battery count"

# 13. Zero-orphan audit (directive AC; guard 11): AFTER teardown, no
#     fixture containers, no responder children, no leaked processes.
#     (The fixture container legitimately runs DURING the suite; the
#     restart test leaves it running at the end. Teardown runs first,
#     then the audit proves nothing leaked past it.)
trap - EXIT
sh infra/mail/teardown.sh >> "$LOG" 2>&1 || true
ORPHANS=0
if docker ps -aq --filter "name=ep026-mail-" | grep -q .; then
  echo "ERROR: orphan mail fixture container" >> "$LOG"
  ORPHANS=1
fi
if pgrep -f 'tcp_break_proxy.py|silent_listener.py' >/dev/null 2>&1; then
  echo "ERROR: orphan responder process" >> "$LOG"
  ORPHANS=1
fi
if pgrep -f 'cargo test -p nexus-imap-smtp' >/dev/null 2>&1; then
  echo "ERROR: leaked test process" >> "$LOG"
  ORPHANS=1
fi
[ "$ORPHANS" -eq 0 ] || fail "zero-orphan audit"

step "M4 gate complete: $(grep -c 'test .* ok' "$LOG" || true) observed"
echo "EP-026 M4: ok"
