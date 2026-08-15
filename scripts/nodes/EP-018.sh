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
  M1) python3 scripts/node-artifact-check.py EP-018 M1 && test -s crates/nexus-skills/tests/ep018_unit_contract.rs && sh scripts/ep018-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-018 M2; cargo test --locked -p nexus-skills ep018_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-018 M3; cargo test --locked -p nexus-skills ep018_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-018 M4; cargo test --locked -p nexus-skills ep018_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-018 M5
      cargo test --locked -p nexus-skills
      sh scripts/live-fire/LF-018.sh
      ;;
  *) echo "EP-018: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-018 $mode: ok"
