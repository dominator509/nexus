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
  M1) python3 scripts/node-artifact-check.py EP-021 M1 && sh scripts/ep021-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-021 M2 && sh scripts/ep021-m2-tests.sh ;;
  M3) python3 scripts/node-artifact-check.py EP-021 M3; uv run --frozen pytest tests/voice/core -q -k ep021_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-021 M4; uv run --frozen pytest tests/voice/core -q -k ep021_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-021 M5
      uv run --frozen pytest tests/voice/core -q -k ep021
      sh scripts/live-fire/LF-028.sh
      ;;
  *) echo "EP-021: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-021 $mode: ok"
