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
  M1) python3 scripts/node-artifact-check.py EP-004 M1 ;;
  M2) python3 scripts/node-artifact-check.py EP-004 M2; cargo test --locked -p nexus-memory ep004_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-004 M3; cargo test --locked -p nexus-memory ep004_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-004 M4; cargo test --locked -p nexus-memory ep004_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-004 M5
      cargo test --locked -p nexus-memory
      :
      ;;
  *) echo "EP-004: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-004 $mode: ok"
