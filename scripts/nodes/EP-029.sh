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
  M1) python3 scripts/node-artifact-check.py EP-029 M1 ;;
  M2) python3 scripts/node-artifact-check.py EP-029 M2; cargo test --locked -p nexus-social ep029_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-029 M3; cargo test --locked -p nexus-social ep029_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-029 M4; cargo test --locked -p nexus-social ep029_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-029 M5
      cargo test --locked -p nexus-social
      sh scripts/live-fire/LF-014.sh
      sh scripts/live-fire/LF-027.sh
      ;;
  *) echo "EP-029: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-029 $mode: ok"
