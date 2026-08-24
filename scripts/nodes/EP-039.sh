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
  M1) sh scripts/ep039-m1-tests.sh ;;
  M2) sh scripts/ep039-m2-tests.sh ;;
  M3) sh scripts/ep039-m3-tests.sh ;;
  M4) sh scripts/ep039-m4-tests.sh ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-039 M5
      python3 tests/infra/test_ep039.py
      :
      ;;
  *) echo "EP-039: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-039 $mode: ok"
