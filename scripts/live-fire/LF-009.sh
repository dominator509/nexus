#!/usr/bin/env sh
# LF-009 sentinel-quarantine live-fire (EP-031 M5).
#
# Drives the advanced sentinel through the production Zeek + CrowdSec +
# Wazuh + osquery connectors (real transports) and the OPNsense
# containment engine against controlled local fixtures over REAL
# sockets, and requires current-run machine-readable evidence
# (.agent/state/evidence/LF-009-ep031-m5.json embedding
# EP031_M5_RUN_ID - stale evidence never satisfies).
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep031-m5-tests.sh

echo "LF-009: ok"
