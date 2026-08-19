#!/usr/bin/env sh
# LF-014 social-campaign live-fire (EP-029 M5).
#
# Drives the social command center through the production Postiz +
# direct-platform adapters (real HTTP transports) against controlled
# local fixtures over REAL sockets and requires current-run
# machine-readable evidence (.agent/state/evidence/LF-014-ep029-m5.json
# embedding EP029_M5_RUN_ID - stale evidence never satisfies).
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep029-m5-tests.sh

echo "LF-014: ok"
