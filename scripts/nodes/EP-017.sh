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
  M1) python3 scripts/node-artifact-check.py EP-017 M1 && cargo test --locked -p nexus-agents ep017_unit ;;
  M2) python3 scripts/node-artifact-check.py EP-017 M2 && test -s crates/nexus-harness-adapters/tests/ep017_unit_orchestrator.rs && cargo test --locked -p nexus-harness-adapters ep017_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-017 M3; cargo test --locked -p nexus-agents ep017_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-017 M4; cargo test --locked -p nexus-agents ep017_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-017 M5
      cargo test --locked -p nexus-agents
      sh scripts/live-fire/LF-016.sh
      ;;
  *) echo "EP-017: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-017 $mode: ok"
