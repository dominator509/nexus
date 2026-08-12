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
  M1) python3 scripts/node-artifact-check.py EP-015 M1 ;;
  M2) python3 scripts/node-artifact-check.py EP-015 M2; cargo test --locked -p nexus-model-router ep015_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-015 M3; cargo test --locked -p nexus-model-router ep015_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-015 M4; cargo test --locked -p nexus-model-router ep015_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-015 M5
      cargo test --locked -p nexus-model-router
      sh scripts/live-fire/LF-021.sh
      ;;
  *) echo "EP-015: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-015 $mode: ok"
