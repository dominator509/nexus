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
  M1) python3 scripts/node-artifact-check.py EP-003 M1 && cargo test --locked -p nexus-identity ep003_unit || rc=$? ;;
  M2) python3 scripts/node-artifact-check.py EP-003 M2 && cargo test --locked -p nexus-identity ep003_unit && cargo test --locked -p nexus-presence ep003_unit || rc=$? ;;
  M3) python3 scripts/node-artifact-check.py EP-003 M3 && cargo test --locked -p nexus-identity ep003_integration || rc=$? ;;
  M4) python3 scripts/node-artifact-check.py EP-003 M4 && cargo test --locked -p nexus-identity ep003_failure && sh scripts/security-check.sh && sh scripts/license-gate.sh || rc=$? ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-003 M5 \
      && cargo test --locked -p nexus-identity \
      && cargo test --locked -p nexus-presence \
      && sh scripts/test-unit.sh \
      && sh scripts/test-failure.sh \
      && sh scripts/test-integration.sh \
      && sh scripts/security-check.sh \
      && sh scripts/license-gate.sh \
      && sh scripts/reality-gate.sh
      rc=$?
      ;;
  *) echo "EP-003: FAIL - unknown mode $mode" >&2; exit 2;;
esac
if [ "$rc" -ne 0 ]; then
  echo "EP-003 $mode: FAIL (exit $rc)" >&2
  exit "$rc"
fi
echo "EP-003 $mode: ok"
