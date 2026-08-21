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
  M1) python3 scripts/node-artifact-check.py EP-034 M1 && sh scripts/ep034-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-034 M2 && sh scripts/ep034-m2-tests.sh ;;
  M3) python3 scripts/node-artifact-check.py EP-034 M3 && sh scripts/ep034-m3-tests.sh ;;
  M4) python3 scripts/node-artifact-check.py EP-034 M4; (cd apps/mobile && flutter test --plain-name "EP-034 failure") ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-034 M5
      (cd apps/mobile && flutter test)
      sh scripts/live-fire/LF-004.sh
      sh scripts/live-fire/LF-022.sh
      ;;
  *) echo "EP-034: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-034 $mode: ok"
