#!/usr/bin/env sh
# LF-022 mobile-step-up live-fire (EP-034 M5).
#
# Request a high-risk action by voice, refuse voice-only authorization,
# approve with the mobile step-up path, execute, and verify. Composed
# from the REAL production nexus_mobile contracts and
# nexus_mobile_contracts behavior layers; native biometric/passkey
# verification is NOT ASSERTED (deferred native milestone). Evidence is
# current-run bound (.agent/state/evidence/LF-022-ep034-m5.json - stale
# evidence never satisfies).
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep034-m5-tests.sh

echo "LF-022: ok"
