#!/usr/bin/env sh
set -eu
# Canonical environment: mise shims PATH + non-interactive exports.
. scripts/env.sh
mode="${1:-verify}"
rc=0
case "$mode" in
  M1) python3 scripts/node-artifact-check.py EP-010 M1 && cargo test --locked -p nexus-capabilities ep010_unit || rc=$? ;;
  M2) python3 scripts/node-artifact-check.py EP-010 M2 && cargo test --locked -p nexus-connectors ep010_unit || rc=$? ;;
  M3) python3 scripts/node-artifact-check.py EP-010 M3 && cargo test --locked -p nexus-connectors ep010_integration || rc=$? ;;
  M4) python3 scripts/node-artifact-check.py EP-010 M4 && cargo test --locked -p nexus-connectors ep010_failure && sh scripts/security-check.sh && sh scripts/license-gate.sh || rc=$? ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-010 M5 \
      && cargo test --locked -p nexus-capabilities \
      && cargo test --locked -p nexus-connectors \
      && sh scripts/ep010-vacuity.sh \
      && sh scripts/security-check.sh \
      && sh scripts/license-gate.sh \
      && sh scripts/reality-gate.sh \
      && sh scripts/format-check.sh \
      && sh scripts/lint.sh \
      && sh scripts/ep010-orphan-audit.sh
      rc=$?
      ;;
  *) echo "EP-010: FAIL - unknown mode $mode" >&2; exit 2;;
esac
if [ "$rc" -ne 0 ]; then
  echo "EP-010 $mode: FAIL (exit $rc)" >&2
  exit "$rc"
fi
echo "EP-010 $mode: ok"
