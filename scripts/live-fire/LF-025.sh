#!/usr/bin/env sh
# LF-025 ceo-business-brief live-fire (EP-028 M5).
#
# Drives the permission-filtered executive brief (CRM + social +
# communications + finance + operational sources with provenance and
# data freshness) through the production adapter + HTTP transport
# against a controlled local fixture over REAL sockets and requires
# current-run machine-readable evidence
# (.agent/state/evidence/LF-025-ep028-m5.json embedding
# EP028_M5_RUN_ID - stale evidence never satisfies).
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep028-m5-tests.sh

echo "LF-025: ok"
