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
  M1) python3 scripts/node-artifact-check.py EP-037 M1 ;;
  M2) python3 scripts/node-artifact-check.py EP-037 M2; cargo test --locked -p nexus-artifacts ep037_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-037 M3; cargo test --locked -p nexus-artifacts ep037_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-037 M4; cargo test --locked -p nexus-artifacts ep037_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-037 M5
      cargo test --locked -p nexus-artifacts
      sh scripts/live-fire/LF-002.sh
      sh scripts/live-fire/LF-020.sh
      ;;
  *) echo "EP-037: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-037 $mode: ok"
