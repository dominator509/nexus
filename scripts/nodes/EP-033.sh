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
  M1) python3 scripts/node-artifact-check.py EP-033 M1 && sh scripts/ep033-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-033 M2; pnpm --filter @nexus/web exec vitest run -t ep033_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-033 M3; pnpm --filter @nexus/web exec vitest run -t ep033_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-033 M4; pnpm --filter @nexus/web exec vitest run -t ep033_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-033 M5
      pnpm --filter @nexus/web test
      sh scripts/live-fire/LF-005.sh
      ;;
  *) echo "EP-033: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-033 $mode: ok"
