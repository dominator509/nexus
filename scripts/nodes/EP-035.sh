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
  M1)
    sh scripts/ep035-m1-tests.sh
    ;;
  M2) python3 scripts/node-artifact-check.py EP-035 M2; cargo test --locked -p nexus-setup ep035_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-035 M3; cargo test --locked -p nexus-setup ep035_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-035 M4; cargo test --locked -p nexus-setup ep035_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-035 M5
      cargo test --locked -p nexus-setup
      sh scripts/live-fire/LF-001.sh
      ;;
  *) echo "EP-035: FAIL - unknown mode $mode" >&2; exit 2;;
esac
rc=$?
if [ "$rc" -eq 0 ]; then echo "EP-035 $mode: ok"; fi
exit "$rc"
