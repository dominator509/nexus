#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
mode="${1:-verify}"
case "$mode" in
  M1) python3 scripts/node-artifact-check.py EP-026 M1; sh scripts/ep026-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-026 M2; cargo test --locked -p nexus-email ep026_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-026 M3; cargo test --locked -p nexus-email ep026_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-026 M4; cargo test --locked -p nexus-email ep026_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-026 M5
      cargo test --locked -p nexus-email
      sh scripts/live-fire/LF-011.sh
      ;;
  *) echo "EP-026: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-026 $mode: ok"
