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
  M1) python3 scripts/node-artifact-check.py EP-020 M1 && test -s crates/nexus-home/tests/ep020_unit_contract.rs && sh scripts/ep020-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-020 M2 && test -s connectors/home-assistant/tests/ep020_unit_adapter.rs && sh scripts/ep020-m2-tests.sh ;;
  M3) python3 scripts/node-artifact-check.py EP-020 M3 && test -s infra/home-assistant/tests/test_ep020_integration_home_assistant.py && sh scripts/ep020-m3-tests.sh ;;
  M4) python3 scripts/node-artifact-check.py EP-020 M4 && test -s crates/nexus-home/tests/ep020_failure_forced.rs && sh scripts/ep020-m4-tests.sh ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-020 M5
      cargo test --locked -p nexus-home
      sh scripts/live-fire/LF-006.sh
      sh scripts/live-fire/LF-007.sh
      sh scripts/live-fire/LF-024.sh
      ;;
  *) echo "EP-020: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-020 $mode: ok"
