#!/usr/bin/env sh
set -eu
# Canonical environment: mise shims PATH + non-interactive exports.
. scripts/env.sh
mode="${1:-verify}"
rc=0
case "$mode" in
  M1) python3 scripts/node-artifact-check.py EP-012 M1 && cargo test --locked -p nexus-fabric ep012_unit || rc=$? ;;
  M2) python3 scripts/node-artifact-check.py EP-012 M2 && cargo test --locked -p nexus-fabric ep012_unit && cargo test --locked -p nexus-mcp ep012_unit || rc=$? ;;
  M3) python3 scripts/node-artifact-check.py EP-012 M3 && cargo test --locked -p nexus-a2a ep012_integration || rc=$? ;;
  M4) python3 scripts/node-artifact-check.py EP-012 M4 && cargo test --locked -p nexus-mcp ep012_failure && cargo test --locked -p nexus-a2a ep012_failure || rc=$? ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-012 M5 \
      && cargo test --locked -p nexus-fabric \
      && cargo test --locked -p nexus-mcp \
      && cargo test --locked -p nexus-a2a \
      && cargo test --locked -p nexus-gateway \
      && cargo run --locked -p nexus-gateway --example composed_gateway_evidence \
      && test -s .agent/state/evidence/ep012-m5/ep012-m5-composed-gateway.json \
      && :
      rc=$?
      ;;
  *) echo "EP-012: FAIL - unknown mode $mode" >&2; exit 2;;
esac
if [ "$rc" -ne 0 ]; then
  echo "EP-012 $mode: FAIL (exit $rc)" >&2
  exit "$rc"
fi
echo "EP-012 $mode: ok"
