#!/usr/bin/env sh
set -eu
# Canonical environment: mise shims PATH + non-interactive exports.
. scripts/env.sh
mode="${1:-verify}"
rc=0
case "$mode" in
  M1) python3 scripts/node-artifact-check.py EP-008 M1 && cargo test --locked -p nexus-policy ep008_unit || rc=$? ;;
  M2) python3 scripts/node-artifact-check.py EP-008 M2 && cargo test --locked -p nexus-policy ep008_unit && cargo test --locked -p nexus-action-gateway ep008_unit || rc=$? ;;
  M3) python3 scripts/node-artifact-check.py EP-008 M3 && uv run --frozen pytest tests/policy -q --tb=native -o python_functions="ep008_integration_*" || rc=$? ;;
  M4) python3 scripts/node-artifact-check.py EP-008 M4 && cargo test --locked -p nexus-action-gateway ep008_failure && uv run --frozen pytest tests/policy -q --tb=native -o python_functions="ep008_failure_*" && sh scripts/security-check.sh && sh scripts/license-gate.sh || rc=$? ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-008 M5 \
      && cargo test --locked -p nexus-policy \
      && cargo test --locked -p nexus-action-gateway \
      && uv run --frozen pytest tests/policy -q --tb=native -o python_functions="ep008_failure_*" \
      && sh scripts/test-unit.sh \
      && sh scripts/test-failure.sh \
      && sh scripts/test-integration.sh \
      && sh scripts/security-check.sh \
      && sh scripts/license-gate.sh \
      && sh scripts/reality-gate.sh
      rc=$?
      ;;
  *) echo "EP-008: FAIL - unknown mode $mode" >&2; exit 2;;
esac
if [ "$rc" -ne 0 ]; then
  echo "EP-008 $mode: FAIL (exit $rc)" >&2
  exit "$rc"
fi
echo "EP-008 $mode: ok"
