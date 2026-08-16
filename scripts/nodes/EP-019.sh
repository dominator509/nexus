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
  M1) python3 scripts/node-artifact-check.py EP-019 M1 && test -s crates/nexus-healing/tests/ep019_unit_contract.rs && sh scripts/ep019-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-019 M2 && test -s packages/workflows/src/incidents/index.ts && sh scripts/ep019-m2-tests.sh ;;
  M3) python3 scripts/node-artifact-check.py EP-019 M3 && sh scripts/ep019-m3-tests.sh ;;
  M4) python3 scripts/node-artifact-check.py EP-019 M4 && test -s crates/nexus-healing/tests/ep019_failure_suite.rs && sh scripts/ep019-m4-tests.sh ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-019 M5
      cargo test --locked -p nexus-healing
      sh scripts/live-fire/LF-019.sh
      ;;
  *) echo "EP-019: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-019 $mode: ok"
