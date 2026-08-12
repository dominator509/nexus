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
  M1) python3 scripts/node-artifact-check.py EP-006 M1 ;;
  M2) python3 scripts/node-artifact-check.py EP-006 M2; pnpm --filter @nexus/workflows exec vitest run -t ep006_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-006 M3; pnpm --filter @nexus/workflows exec vitest run -t ep006_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-006 M4; pnpm --filter @nexus/workflows exec vitest run -t ep006_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-006 M5
      pnpm --filter @nexus/workflows test
      sh scripts/live-fire/LF-017.sh
      ;;
  *) echo "EP-006: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-006 $mode: ok"
