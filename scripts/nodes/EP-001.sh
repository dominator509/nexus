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
  M1) python3 scripts/node-artifact-check.py EP-001 M1 ;;
  M2) python3 scripts/node-artifact-check.py EP-001 M2; sh scripts/lint.sh && sh scripts/typecheck.sh && sh scripts/test-unit.sh ;;
  M3) python3 scripts/node-artifact-check.py EP-001 M3; sh scripts/lint.sh && sh scripts/typecheck.sh && sh scripts/test-unit.sh ;;
  M4) python3 scripts/node-artifact-check.py EP-001 M4; sh scripts/lint.sh && sh scripts/typecheck.sh && sh scripts/test-unit.sh ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-001 M5
      sh scripts/lint.sh && sh scripts/typecheck.sh && sh scripts/test-unit.sh
      :
      ;;
  *) echo "EP-001: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-001 $mode: ok"
