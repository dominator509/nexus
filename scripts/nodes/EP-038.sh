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
  M1) sh scripts/ep038-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-038 M2; cargo test --locked -p nexus-observability ep038_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-038 M3; cargo test --locked -p nexus-observability ep038_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-038 M4; cargo test --locked -p nexus-observability ep038_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-038 M5
      cargo test --locked -p nexus-observability
      :
      ;;
  *) echo "EP-038: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-038 $mode: ok"
