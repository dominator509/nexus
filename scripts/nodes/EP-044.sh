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
  M1) python3 scripts/node-artifact-check.py EP-044 M1 && cargo test --locked -p nexus-control-plane ep044_unit ;;
  M2) python3 scripts/node-artifact-check.py EP-044 M2; cargo test --locked -p nexus-control-plane ep044_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-044 M3; cargo test --locked -p nexus-control-plane ep044_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-044 M4; cargo test --locked -p nexus-control-plane ep044_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-044 M5
      cargo test --locked -p nexus-control-plane
      sh scripts/live-fire/LF-029.sh
      sh tests/runtime/smoke-gate-regression.sh
      :
      ;;
  *) echo "EP-044: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-044 $mode: ok"
