#!/usr/bin/env sh
# LF-010 network-diagnosis live-fire (EP-030 M5).
#
# Drives the sentinel core through the production OPNsense + OpenWrt +
# AdGuard Home connectors (real HTTP transports) against controlled
# local fixtures over REAL sockets and requires current-run
# machine-readable evidence (.agent/state/evidence/LF-010-ep030-m5.json
# embedding EP030_M5_RUN_ID - stale evidence never satisfies).
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep030-m5-tests.sh

echo "LF-010: ok"
