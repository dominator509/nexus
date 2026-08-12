#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
mode="${1:-verify}"
rc=0
case "$mode" in
  M1) python3 scripts/node-artifact-check.py EP-001 M1 || rc=$? ;;
  M2) python3 scripts/node-artifact-check.py EP-001 M2 && sh scripts/lint.sh && sh scripts/typecheck.sh && sh scripts/test-unit.sh || rc=$? ;;
  M3) python3 scripts/node-artifact-check.py EP-001 M3 && sh scripts/lint.sh && sh scripts/typecheck.sh && sh scripts/test-unit.sh || rc=$? ;;
  M4) python3 scripts/node-artifact-check.py EP-001 M4 && sh scripts/lint.sh && sh scripts/typecheck.sh && sh scripts/test-unit.sh && sh scripts/test-failure.sh || rc=$? ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-001 M5 \
      && sh scripts/lint.sh \
      && sh scripts/typecheck.sh \
      && sh scripts/test-unit.sh \
      && sh scripts/test-failure.sh \
      && sh scripts/test-integration.sh \
      && sh scripts/security-check.sh \
      && sh scripts/license-gate.sh \
      && sh scripts/reality-gate.sh
      rc=$?
      ;;
  *) echo "EP-001: FAIL - unknown mode $mode" >&2; exit 2;;
esac
if [ "$rc" -ne 0 ]; then
  echo "EP-001 $mode: FAIL (exit $rc)" >&2
  exit "$rc"
fi
echo "EP-001 $mode: ok"
