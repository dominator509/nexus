#!/usr/bin/env sh
set -eu
# Canonical environment: mise shims PATH + non-interactive exports.
. scripts/env.sh
mode="${1:-verify}"
rc=0
case "$mode" in
  M1) python3 scripts/node-artifact-check.py EP-004 M1 && cargo test --locked -p nexus-data ep004_unit && uv run --frozen pytest tests/memory -q -o python_functions="ep004_unit_*" || rc=$? ;;
  M2) python3 scripts/node-artifact-check.py EP-004 M2 && cargo test --locked -p nexus-memory ep004_unit || rc=$? ;;
  M3) python3 scripts/node-artifact-check.py EP-004 M3 && cargo test --locked -p nexus-memory ep004_integration || rc=$? ;;
  M4) python3 scripts/node-artifact-check.py EP-004 M4 && cargo test --locked -p nexus-memory ep004_failure && sh scripts/security-check.sh && sh scripts/license-gate.sh || rc=$? ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-004 M5 \
      && cargo test --locked -p nexus-data \
      && cargo test --locked -p nexus-memory \
      && uv run --frozen pytest tests/memory -q -o python_functions="ep004_unit_*" \
      && sh scripts/test-unit.sh \
      && sh scripts/test-failure.sh \
      && sh scripts/test-integration.sh \
      && sh scripts/security-check.sh \
      && sh scripts/license-gate.sh \
      && sh scripts/reality-gate.sh
      rc=$?
      ;;
  *) echo "EP-004: FAIL - unknown mode $mode" >&2; exit 2;;
esac
if [ "$rc" -ne 0 ]; then
  echo "EP-004 $mode: FAIL (exit $rc)" >&2
  exit "$rc"
fi
echo "EP-004 $mode: ok"
