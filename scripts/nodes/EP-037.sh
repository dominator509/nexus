#!/usr/bin/env sh
# EP-037 node driver: every milestone runs the REAL owned gate with rc
# propagation. No masking path may ever return success without running
# the gate.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
mode="${1:-verify}"
case "$mode" in
  M1) sh scripts/ep037-m1-tests.sh ;;
  M2) sh scripts/ep037-m2-tests.sh ;;
  M3) sh scripts/ep037-m3-tests.sh ;;
  M4) sh scripts/ep037-m4-tests.sh ;;
  M5|verify)
      sh scripts/ep037-m5-tests.sh
      ;;
  *) echo "EP-037: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-037 $mode: ok"
