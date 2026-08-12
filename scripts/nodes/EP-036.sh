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
  M1) python3 scripts/node-artifact-check.py EP-036 M1 ;;
  M2) python3 scripts/node-artifact-check.py EP-036 M2; python3 tests/infra/test_ep036.py ;;
  M3) python3 scripts/node-artifact-check.py EP-036 M3; python3 tests/infra/test_ep036.py ;;
  M4) python3 scripts/node-artifact-check.py EP-036 M4; python3 tests/infra/test_ep036.py ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-036 M5
      python3 tests/infra/test_ep036.py
      :
      ;;
  *) echo "EP-036: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-036 $mode: ok"
