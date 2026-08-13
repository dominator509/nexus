#!/usr/bin/env sh
set -eu
# Canonical environment: mise shims PATH + non-interactive exports.
. scripts/env.sh
mode="${1:-verify}"
rc=0
case "$mode" in
  M1) python3 scripts/node-artifact-check.py EP-005 M1 && cargo test --locked -p nexus-events ep005_unit || rc=$? ;;
  M2) python3 scripts/node-artifact-check.py EP-005 M2 && cargo test --locked -p nexus-events ep005_unit && cargo test --locked -p nexus-nats ep005_unit || rc=$? ;;
  M3) python3 scripts/node-artifact-check.py EP-005 M3 && cargo test --locked -p nexus-nats ep005_integration || rc=$? ;;
  M4) python3 scripts/node-artifact-check.py EP-005 M4 && cargo test --locked -p nexus-events ep005_failure && sh scripts/security-check.sh && sh scripts/license-gate.sh || rc=$? ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-005 M5 \
      && cargo test --locked -p nexus-events \
      && sh scripts/test-unit.sh \
      && sh scripts/test-failure.sh \
      && sh scripts/test-integration.sh \
      && sh scripts/security-check.sh \
      && sh scripts/license-gate.sh \
      && sh scripts/reality-gate.sh
      rc=$?
      ;;
  *) echo "EP-005: FAIL - unknown mode $mode" >&2; exit 2;;
esac
if [ "$rc" -ne 0 ]; then
  echo "EP-005 $mode: FAIL (exit $rc)" >&2
  exit "$rc"
fi
echo "EP-005 $mode: ok"
