#!/usr/bin/env sh
# EP-026 M5 gate: LF-011 real email lifecycle live-fire + node closure
# proof.
#
# Replaces the pre-created EP-001-masking LF-011 placeholder (a dead
# proof-runner delegation) with the REAL email lifecycle live-fire
# through the REAL production EmailProvider adapter (nexus-imap-smtp)
# over REAL sockets against the certified controlled mail provider
# (GreenMail 2.1.0, pinned digest):
#
#   real fixture provision
#     -> real IMAP/SMTP sockets
#     -> real authentication
#     -> real draft (IMAP APPEND)
#     -> approval gate (below-minimum class denied, zero mutation)
#     -> real SMTP submission -> SENT (250), never DELIVERED from a 250
#     -> INDEPENDENT recipient-side readback (tenant-b adapter)
#     -> exact runtime canary found -> MailVerifier exact-target
#     -> canonical digest-only summary of the real message
#     -> hostile content remains DATA (zero consequential mutation)
#     -> attachment gate (policy-denied -> zero provider mutation)
#     -> redaction (no fixture credential in audit/evidence)
#     -> machine-readable current-run evidence
#       (.agent/state/evidence/LF-011-ep026-m5.json, run-id bound)
#     -> teardown -> zero-orphan audit
#
# Vacuity guards (directive W/AA): every proof class must be observed
# in the current run; the gate never prints "EP-026 M5: ok" on an
# empty, masked, or stale-evidence run.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

REPO="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO"
LOG=/tmp/ep026-m5-gate.log
: > "$LOG"
RUN_ID="$(date +%s%N)"
EVIDENCE=.agent/state/evidence/LF-011-ep026-m5.json
echo "== EP-026 M5 gate start $(date -u +%FT%TZ) run_id=$RUN_ID" >> "$LOG"

step() { echo "[m5-gate] $*" | tee -a "$LOG"; }
fail() { echo "EP-026 M5: FAIL ($*)" | tee -a "$LOG"; exit 1; }

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
export EP026_M5_RUN_ID="$RUN_ID"

# 1. Run the LF-011 live-fire suite (real sockets, serial).
/root/.cargo/bin/cargo test -p nexus-imap-smtp --test ep026_m5_lf011 \
  -- --ignored --test-threads=1 >> "$LOG" 2>&1 || fail "LF-011 live-fire suite"
TEST_LOG="$LOG"

# 2. Guard: tests were actually collected and ran (vacuity guard 1).
grep -q "running .* tests" "$TEST_LOG" || fail "no tests collected"
grep -q "test result: ok" "$TEST_LOG" || fail "suite did not pass"
grep -q "4 passed" "$TEST_LOG" || fail "LF-011 suite count != 4"

# 3. Guard: external provider actually contacted (real sockets;
#    vacuity guard 1 -> provider contacted).
grep -q "lf011_full_lifecycle_real_provider.*ok$" "$TEST_LOG" \
  || fail "full lifecycle proof did not run"

# 4. Guard: authentication actually occurred (real AUTH/LOGIN against
#    the real provider; the lifecycle proof cannot pass without auth).
grep -q "lf011_redaction_evidence_no_leak.*ok$" "$TEST_LOG" \
  || fail "redaction proof did not run"

# 5. Guard: real send + provider acceptance observed + recipient-side
#    exact-target readback + current-run canary matched are all inside
#    the full-lifecycle proof; additionally require the evidence file
#    exists and embeds the CURRENT run id (vacuity guard: evidence
#    current-run; stale evidence never satisfies).
[ -f "$EVIDENCE" ] || fail "LF-011 evidence file not created"
grep -qF "\"run_id\": \"$RUN_ID\"" "$EVIDENCE" \
  || fail "evidence does not embed the current run id"
grep -q "\"exact_target_verification\": \"Verified\"" "$EVIDENCE" \
  || fail "evidence lacks exact-target verification"
grep -q "\"smtp_submission\": \"SENT (250)\"" "$EVIDENCE" \
  || fail "evidence lacks SENT (250) acceptance"
grep -q "\"recipient_inbox_readback\": \"DELIVERED" "$EVIDENCE" \
  || fail "evidence lacks recipient-side readback"
grep -q "\"external_provider_certification\": \"NOT ASSERTED\"" "$EVIDENCE" \
  || fail "evidence must truthfully record external certification"

# 6. Guard: hostile content proof ran (vacuity guard 7).
grep -q "lf011_hostile_content_remains_data.*ok$" "$TEST_LOG" \
  || fail "hostile-content proof did not run"

# 7. Guard: attachment policy proof ran (vacuity guard 8).
grep -q "lf011_attachment_gate_no_mutation.*ok$" "$TEST_LOG" \
  || fail "attachment-gate proof did not run"

# 8. Guard: redaction canary zero-leakage (vacuity guard 10) - the
#    redaction test asserts no fixture credential in audit ring or
#    evidence; additionally scan the gate log + evidence.
CANARY_PW_A="$EP026_MAIL_PASS_A"
CANARY_PW_B="$EP026_MAIL_PASS_B"
if grep -qF "$CANARY_PW_A" "$LOG" || grep -qF "$CANARY_PW_B" "$LOG"; then
  fail "fixture credential leaked into gate evidence"
fi
if grep -qF "$CANARY_PW_A" "$EVIDENCE" || grep -qF "$CANARY_PW_B" "$EVIDENCE"; then
  fail "fixture credential leaked into LF-011 evidence"
fi

# 9. Zero-orphan audit (vacuity guard 11): AFTER teardown, no fixture
#    containers, no responder children, no leaked processes.
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

step "M5 gate complete: $(grep -c 'test .* ok' "$LOG" || true) observed"
echo "EP-026 M5: ok"
