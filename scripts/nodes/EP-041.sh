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
  M1) sh scripts/ep041-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-041 M2; uv run --frozen pytest tests/microbrain -q -k ep041_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-041 M3; uv run --frozen pytest tests/microbrain -q -k ep041_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-041 M4; uv run --frozen pytest tests/microbrain -q -k ep041_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-041 M5
      uv run --frozen pytest tests/microbrain -q -k ep041
      :
      ;;
  *) echo "EP-041: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-041 $mode: ok"
