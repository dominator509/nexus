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
  M1) python3 scripts/node-artifact-check.py EP-011 M1 ;;
  M2) python3 scripts/node-artifact-check.py EP-011 M2; cargo test --locked -p nexus-connector-sdk && pnpm --filter @nexus/connector-sdk test && uv run --frozen pytest tests/connectors/python -q ;;
  M3) python3 scripts/node-artifact-check.py EP-011 M3; cargo test --locked -p nexus-connector-sdk && pnpm --filter @nexus/connector-sdk test && uv run --frozen pytest tests/connectors/python -q ;;
  M4) python3 scripts/node-artifact-check.py EP-011 M4; cargo test --locked -p nexus-connector-sdk && pnpm --filter @nexus/connector-sdk test && uv run --frozen pytest tests/connectors/python -q ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-011 M5
      cargo test --locked -p nexus-connector-sdk && pnpm --filter @nexus/connector-sdk test && uv run --frozen pytest tests/connectors/python -q
      sh scripts/live-fire/LF-023.sh
      ;;
  *) echo "EP-011: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-011 $mode: ok"
