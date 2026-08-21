#!/usr/bin/env sh
# LF-004 multi-user-identity live-fire (EP-034 M5).
#
# Enroll two adults and one restricted user; prove separate context,
# permissions, preferences, and mobile devices. Composed from the REAL
# production nexus_mobile contracts and nexus_mobile_contracts behavior
# layers; evidence is current-run bound
# (.agent/state/evidence/LF-004-ep034-m5.json - stale evidence never
# satisfies).
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep034-m5-tests.sh

echo "LF-004: ok"
