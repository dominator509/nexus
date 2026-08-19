#!/usr/bin/env sh
# LF-030 fax-lifecycle live-fire (EP-027 M5).
#
# Drives the REAL outbound fax lifecycle through the production
# nexus-hylafax connector against the real pinned HylaFAX fixture and
# requires current-run machine-readable evidence
# (.agent/state/evidence/LF-030-ep027-m5.json embedding
# EP027_M5_RUN_ID - stale evidence never satisfies).
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep027-m5-tests.sh

echo "LF-030: ok"
