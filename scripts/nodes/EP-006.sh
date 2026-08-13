#!/usr/bin/env sh
set -eu
# Canonical environment: mise shims PATH + non-interactive exports.
. scripts/env.sh
export NO_COLOR=1
mode="${1:-verify}"
rc=0

# Run a named ep006 test filter and fail closed if no test actually ran.
# vitest exits 0 when a name filter matches zero tests (EP-001 gate-masking
# class), so the summary must show at least one passed test before the
# milestone sentinel may be printed.
run_ep006_tests() {
  filter="$1"
  log="/tmp/ep006-vitest-${filter}.log"
  if ! pnpm --filter @nexus/workflows exec vitest run -t "$filter" >"$log" 2>&1; then
    echo "EP-006: FAIL - vitest filter '$filter' failed" >&2
    tail -20 "$log" >&2
    return 1
  fi
  if ! sed 's/\x1b\[[0-9;]*m//g' "$log" | grep -q "passed ("; then
    echo "EP-006: FAIL - no passing tests matched filter '$filter' (vacuity guard)" >&2
    tail -10 "$log" >&2
    return 1
  fi
  tail -6 "$log"
}

case "$mode" in
  M1) python3 scripts/node-artifact-check.py EP-006 M1 && run_ep006_tests ep006_unit || rc=$? ;;
  M2) python3 scripts/node-artifact-check.py EP-006 M2 && run_ep006_tests ep006_unit || rc=$? ;;
  M3) python3 scripts/node-artifact-check.py EP-006 M3 && run_ep006_tests ep006_integration || rc=$? ;;
  M4) python3 scripts/node-artifact-check.py EP-006 M4 && run_ep006_tests ep006_failure && sh scripts/security-check.sh && sh scripts/license-gate.sh || rc=$? ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-006 M5 \
      && pnpm --filter @nexus/workflows test \
      && sh scripts/live-fire/LF-017.sh
      rc=$?
      ;;
  *) echo "EP-006: FAIL - unknown mode $mode" >&2; exit 2;;
esac
if [ "$rc" -ne 0 ]; then
  echo "EP-006 $mode: FAIL (exit $rc)" >&2
  exit "$rc"
fi
echo "EP-006 $mode: ok"
