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
  M1) python3 scripts/node-artifact-check.py EP-040 M1 ;;
  M2) python3 scripts/node-artifact-check.py EP-040 M2; sh scripts/verify.sh ;;
  M3) python3 scripts/node-artifact-check.py EP-040 M3; sh scripts/verify.sh ;;
  M4) python3 scripts/node-artifact-check.py EP-040 M4; sh scripts/verify.sh ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-040 M5
      sh scripts/verify.sh
      :
      ;;
  *) echo "EP-040: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-040 $mode: ok"
