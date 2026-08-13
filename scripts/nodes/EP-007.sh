#!/usr/bin/env sh
set -eu
# Canonical environment: mise shims PATH + non-interactive exports.
. scripts/env.sh
mode="${1:-verify}"
rc=0
case "$mode" in
  M1) python3 scripts/node-artifact-check.py EP-007 M1 && cargo test --locked -p nexus-auth ep007_unit || rc=$? ;;
  M2) python3 scripts/node-artifact-check.py EP-007 M2 && cargo test --locked -p nexus-auth ep007_unit && cargo test --locked -p nexus-keycloak ep007_unit || rc=$? ;;
  M3) python3 scripts/node-artifact-check.py EP-007 M3 && cargo test --locked -p nexus-auth ep007_integration || rc=$? ;;
  M4) python3 scripts/node-artifact-check.py EP-007 M4 && cargo test --locked -p nexus-auth ep007_failure && uv run --frozen pytest tests/auth -q -o python_functions="ep007_failure_*" && sh scripts/security-check.sh && sh scripts/license-gate.sh || rc=$? ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-007 M5 \
      && cargo test --locked -p nexus-auth \
      && uv run --frozen pytest tests/auth -q -o python_functions="ep007_failure_*" \
      && sh scripts/test-unit.sh \
      && sh scripts/test-failure.sh \
      && sh scripts/test-integration.sh \
      && sh scripts/security-check.sh \
      && sh scripts/license-gate.sh \
      && sh scripts/reality-gate.sh
      rc=$?
      ;;
  *) echo "EP-007: FAIL - unknown mode $mode" >&2; exit 2;;
esac
if [ "$rc" -ne 0 ]; then
  echo "EP-007 $mode: FAIL (exit $rc)" >&2
  exit "$rc"
fi
echo "EP-007 $mode: ok"
