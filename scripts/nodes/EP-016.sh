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
  M1) python3 scripts/node-artifact-check.py EP-016 M1 ;;
  M2) python3 scripts/node-artifact-check.py EP-016 M2; cargo test --locked -p nexus-context ep016_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-016 M3; cargo test --locked -p nexus-context ep016_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-016 M4; cargo test --locked -p nexus-context ep016_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-016 M5
      cargo test --locked -p nexus-context
      :
      ;;
  *) echo "EP-016: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-016 $mode: ok"
