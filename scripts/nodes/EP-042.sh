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
  M1) sh scripts/ep042-m1-tests.sh ;;
  M2) sh scripts/ep042-m2-tests.sh ;;
  M3) sh scripts/ep042-m3-tests.sh ;;
  M4) python3 scripts/node-artifact-check.py EP-042 M4; cargo test --locked -p nexus-release ep042_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-042 M5
      cargo test --locked -p nexus-release
      :
      ;;
  *) echo "EP-042: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-042 $mode: ok"
