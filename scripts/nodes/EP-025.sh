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
  M1) python3 scripts/node-artifact-check.py EP-025 M1; sh scripts/ep025-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-025 M2; sh scripts/ep025-m2-tests.sh ;;
  M3) python3 scripts/node-artifact-check.py EP-025 M3; sh scripts/ep025-m3-tests.sh ;;
  M4) python3 scripts/node-artifact-check.py EP-025 M4; sh scripts/ep025-m4-tests.sh ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-025 M5
      cargo test --locked -p nexus-telephony
      sh scripts/live-fire/LF-012.sh
      ;;
  *) echo "EP-025: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-025 $mode: ok"
