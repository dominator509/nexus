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
  M1) python3 scripts/node-artifact-check.py EP-023 M1 && sh scripts/ep023-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-023 M2 && test -s connectors/frigate/tests/ep023_unit_frigate.rs && sh scripts/ep023-m2-tests.sh ;;
  M3) python3 scripts/node-artifact-check.py EP-023 M3 && sh scripts/ep023-m3-tests.sh ;;
  M4) python3 scripts/node-artifact-check.py EP-023 M4 && sh scripts/ep023-m4-tests.sh ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-023 M5
      cargo test --locked -p nexus-vision
      sh scripts/live-fire/LF-008.sh
      ;;
  *) echo "EP-023: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-023 $mode: ok"
