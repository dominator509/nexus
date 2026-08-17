#!/usr/bin/env sh
# LF-026 voice-endpoint-transfer (EP-022 M5 live-fire).
#
# Real cross-node composition of the production components:
# nexus-audio (router + transfer + context), nexus-assist-satellite
# (adapter core), nexus-bluetooth-audio (real system-bus D-Bus probe).
# Proves the node contract acceptance obligations:
# - Bluetooth reconnect and endpoint transfer preserve conversation
#   context (real DeterministicTransfer; on this host BlueZ is
#   genuinely absent, so the Bluetooth leg is the real NameHasNoOwner
#   probe failing closed - never a fabricated connect);
# - room satellites remain locally functional (real adapter core);
# - input and output endpoints are selected by person, room, privacy,
#   and availability (real router; sensitive content never routes to a
#   shared-room output).
#
# Machine-readable evidence is written by the E2E suite to
# .agent/state/evidence/EP-022-M5-LF-026-voice-endpoint-transfer.json.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/lf026-e2e.log"
: > "$log"
export EVIDENCE_DIR="$(pwd)/.agent/state/evidence"

if ! cargo test --locked -p nexus-audio-e2e ep022_e2e >>"$log" 2>&1; then
  echo "LF-026: FAIL - nexus-audio-e2e ep022_e2e suite failed" >&2
  tail -40 "$log" >&2
  exit 1
fi

# Vacuity guards (EP-001 gate-masking class): real tests ran and
# passed, and the full journey test is present and green.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "LF-026: FAIL - no ep022_e2e tests ran (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "LF-026: FAIL - no passing ep022_e2e tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi
if ! grep -qE '^test ep022_e2e_full_journey_lf026 \.\.\. ok$' "$log"; then
  echo "LF-026: FAIL - full journey test missing or not ok (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi
if [ ! -f .agent/state/evidence/EP-022-M5-LF-026-voice-endpoint-transfer.json ]; then
  echo "LF-026: FAIL - machine-readable evidence file missing" >&2
  exit 1
fi

tail -6 "$log"
echo "LF-026: ok"
