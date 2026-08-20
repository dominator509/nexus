#!/usr/bin/env sh
# LF-005 cross-device-continuity live-fire (EP-033 M5).
#
# Start an objective by voice, continue in the web dashboard, approve
# on mobile (FOUR_EYES, two distinct principals), and receive the
# final artifact in the same task graph. Composed from the REAL
# production @nexus/web, @nexus/desktop, and @nexus/ui components;
# the accessibility proof is machine-observed in REAL headless Chrome
# with the axe-core WCAG 2.2 A/AA rule set. Evidence is current-run
# bound (.agent/state/evidence/LF-005-ep033-m5.json and
# LF-005-ep033-m5-lf005.json - stale evidence never satisfies).
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep033-m5-tests.sh

echo "LF-005: ok"
