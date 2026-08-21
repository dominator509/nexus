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
  M1) sh scripts/ep036-m1-tests.sh ;;
  M2) sh scripts/ep036-m2-tests.sh ;;
  M3)
      # Anti-phantom guard: the obsolete masking path must never return.
      if [ -f tests/infra/test_ep036.py ]; then
        echo "EP-036: FAIL - phantom tests/infra/test_ep036.py must not be the M3 proof owner" >&2
        exit 1
      fi
      sh scripts/ep036-m3-tests.sh
      ;;
  M4) python3 scripts/node-artifact-check.py EP-036 M4; python3 tests/infra/test_ep036.py ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-036 M5
      python3 tests/infra/test_ep036.py
      :
      ;;
  *) echo "EP-036: FAIL - unknown mode $mode" >&2; exit 2;;
esac
rc=$?
if [ "$rc" -eq 0 ]; then echo "EP-036 $mode: ok"; fi
exit "$rc"
