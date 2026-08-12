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
  M1) python3 scripts/node-artifact-check.py EP-012 M1 ;;
  M2) python3 scripts/node-artifact-check.py EP-012 M2; cargo test --locked -p nexus-fabric ep012_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-012 M3; cargo test --locked -p nexus-fabric ep012_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-012 M4; cargo test --locked -p nexus-fabric ep012_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-012 M5
      cargo test --locked -p nexus-fabric
      :
      ;;
  *) echo "EP-012: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-012 $mode: ok"
