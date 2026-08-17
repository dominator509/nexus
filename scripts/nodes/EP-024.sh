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
  M1) python3 scripts/node-artifact-check.py EP-024 M1; sh scripts/ep024-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-024 M2; sh scripts/ep024-m2-tests.sh ;;
  M3) python3 scripts/node-artifact-check.py EP-024 M3; cargo test --locked -p nexus-devices ep024_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-024 M4; cargo test --locked -p nexus-devices ep024_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-024 M5
      cargo test --locked -p nexus-devices
      :
      ;;
  *) echo "EP-024: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-024 $mode: ok"
