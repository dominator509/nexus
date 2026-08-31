#!/usr/bin/env sh
# RX-014 remediation battery: provider adapter truth
# (AUD-009 Gmail wire-format + draft-id recipient; AUD-010 real attachment
#  enumeration Gmail/Graph/IMAP; AUD-018 delivery policy enforced;
#  AUD-019 router destination-aware SMS; AUD-020 ICTFax destination/document
#  binding; AUD-021 consent before recording; AUD-024 X reply binds the
#  mention thread via the official reply object).
#
# The battery runs the REAL test suites that prove each milestone, the
# workspace gates, and the remediation register. The EP-025 M5 LIVE-FIRE
# gate requires the pinned Asterisk container + voice engines and is a
# separate certification; its strengthened assertion surface is checked
# statically here (the orchestrator negative-evidence contract that M6
# introduced), and the orchestrator itself is py_compile verified.
set -eu
cd "$(dirname "$0")/.."
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

pass=0
fail=0
note() { echo "ok - $1"; pass=$((pass + 1)); }
bad() { echo "FAIL - $1"; fail=$((fail + 1)); }

run_suite() {
  # $1 = label, $2 = cargo package, $3 = expected minimum pass count
  out=$(cargo test -p "$2" 2>&1 || true)
  n=$(echo "$out" | grep -Eo "test result: ok\. [0-9]+ passed" | grep -Eo "[0-9]+" | awk '{s+=$1} END{print s+0}')
  if [ "${n:-0}" -ge "$3" ] && ! echo "$out" | grep -q "FAILED\|error\["; then
    note "$1 ($n passed)"
  else
    bad "$1"
    echo "$out" | tail -30
  fi
}

# --- M1: AUD-009 Gmail real wire format + stored-draft send ---
run_suite "nexus-gmail (AUD-009 real wire format + send_draft)" nexus-gmail 17

# --- M2: AUD-010 real attachment enumeration ---
run_suite "nexus-microsoft-mail (AUD-010 Graph attachments)" nexus-microsoft-mail 41
run_suite "nexus-imap-smtp (AUD-010 BODYSTRUCTURE attachments)" nexus-imap-smtp 13
run_suite "nexus-gmail attachments (AUD-010 payload.parts)" nexus-gmail 17

# --- M3/M4: AUD-018 + AUD-019 notification router ---
run_suite "nexus-notifications (AUD-018 policy + AUD-019 router SMS)" nexus-notifications 37
run_suite "nexus-sms-connector (AUD-019 destination-aware)" nexus-sms-connector 22
out=$(cargo test -p nexus-notifications-failure-e2e 2>&1 || true)
n=$(echo "$out" | grep -Eo "test result: ok\. [0-9]+ passed" | grep -Eo "[0-9]+" | awk '{s+=$1} END{print s+0}')
if [ "${n:-0}" -ge 11 ] && ! echo "$out" | grep -q "FAILED\|error\["; then
  note "notifications e2e + livefire ($n passed, future-dated fixtures)"
else
  bad "notifications e2e + livefire"
  echo "$out" | tail -30
fi

# --- M5: AUD-020 ICTFax binds destination/document ---
run_suite "nexus-ictfax (AUD-020 real media + binding)" nexus-ictfax 12

# --- M6: AUD-021 consent BEFORE recording ---
run_suite "nexus-asterisk (AUD-021 governed consent)" nexus-asterisk 64
run_suite "nexus-telephony (AUD-021 policy surface)" nexus-telephony 34
if python3 -m py_compile infra/asterisk/fixture/lf012_orchestrator.py 2>/tmp/rx014-pycompile.log; then
  note "lf012_orchestrator.py py_compile ok"
else
  bad "orchestrator py_compile"
  tail -10 /tmp/rx014-pycompile.log
fi
# EP-025 M5 gate assertion surface (the strengthened negative-evidence
# contract from M6) must be present in the gate script.
if grep -q '"recording_started": false' scripts/ep025-m5-tests.sh \
   && grep -q '"caller_recording_bytes": 0' scripts/ep025-m5-tests.sh \
   && grep -q '"stt_skipped": "recording not consented"' scripts/ep025-m5-tests.sh \
   && grep -q '"stt_transcript"' scripts/ep025-m5-tests.sh; then
  note "EP-025 M5 gate asserts consent-first negative evidence (recording_started false, bytes 0, stt_skipped, transcript absent)"
else
  bad "EP-025 M5 gate missing strengthened assertions"
fi

# --- M7: AUD-024 X reply binds mention thread ---
run_suite "nexus-social-direct-connector (AUD-024 official reply object)" nexus-social-direct-connector 17

# --- workspace gates ---
if cargo check --workspace >/tmp/rx014-check.log 2>&1; then
  note "workspace check clean"
else
  bad "workspace check (see /tmp/rx014-check.log)"
fi
if cargo clippy --workspace --all-targets --all-features --locked -- -D warnings >/tmp/rx014-clippy.log 2>&1; then
  note "workspace clippy clean (-D warnings)"
else
  bad "clippy (see /tmp/rx014-clippy.log)"
fi

# --- remediation register must pass ---
if reg=$(bash .agent/remediation/verify-remediation-register.sh 2>&1); then
  note "remediation register: $(echo "$reg" | tail -1)"
else
  bad "remediation register"
  echo "$reg" | tail -5
fi

echo "---"
echo "RX-014 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
