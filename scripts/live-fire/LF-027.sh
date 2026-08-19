#!/usr/bin/env sh
# LF-027 social-lead-to-crm live-fire (EP-029 M5).
#
# Drives the direct-platform connector through the production adapter
# + real HTTP transport against a controlled local fixture over REAL
# sockets and requires current-run machine-readable evidence
# (.agent/state/evidence/LF-027-ep029-m5.json embedding EP029_M5_RUN_ID
# - stale evidence never satisfies).
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep029-m5-tests.sh

echo "LF-027: ok"
