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
  M1) python3 scripts/node-artifact-check.py EP-007 M1 ;;
  M2) python3 scripts/node-artifact-check.py EP-007 M2; cargo test --locked -p nexus-auth ep007_unit ;;
  M3) python3 scripts/node-artifact-check.py EP-007 M3; cargo test --locked -p nexus-auth ep007_integration ;;
  M4) python3 scripts/node-artifact-check.py EP-007 M4; cargo test --locked -p nexus-auth ep007_failure ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-007 M5
      cargo test --locked -p nexus-auth
      sh scripts/live-fire/LF-003.sh
      ;;
  *) echo "EP-007: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-007 $mode: ok"
