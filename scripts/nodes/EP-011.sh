#!/usr/bin/env sh
set -eu
# Canonical environment: mise shims PATH + non-interactive exports.
. scripts/env.sh
mode="${1:-verify}"
rc=0
case "$mode" in
  M1) python3 scripts/node-artifact-check.py EP-011 M1 && cargo test --locked -p nexus-connector-sdk ep011_unit || rc=$? ;;
  M2) python3 scripts/node-artifact-check.py EP-011 M2; cargo test --locked -p nexus-connector-sdk && pnpm --filter @nexus/connector-sdk test && uv run --frozen pytest tests/connectors/python -q || rc=$? ;;
  M3) python3 scripts/node-artifact-check.py EP-011 M3; cargo test --locked -p nexus-connector-sdk && pnpm --filter @nexus/connector-sdk test && uv run --frozen pytest tests/connectors/python -q || rc=$? ;;
  M4) python3 scripts/node-artifact-check.py EP-011 M4; cargo test --locked -p nexus-connector-sdk && pnpm --filter @nexus/connector-sdk test && uv run --frozen pytest tests/connectors/python -q || rc=$? ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-011 M5 \
      && cargo test --locked -p nexus-connector-sdk \
      && pnpm --filter @nexus/connector-sdk test \
      && uv run --frozen pytest tests/connectors/python -q \
      && sh scripts/live-fire/LF-023.sh
      rc=$?
      ;;
  *) echo "EP-011: FAIL - unknown mode $mode" >&2; exit 2;;
esac
if [ "$rc" -ne 0 ]; then
  echo "EP-011 $mode: FAIL (exit $rc)" >&2
  exit "$rc"
fi
echo "EP-011 $mode: ok"
