#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
mode="${1:-verify}"
case "$mode" in
  M1)
      python3 scripts/node-artifact-check.py EP-000 M1
      python3 references/tests/test_ep000_contracts.py
      ;;
  M2)
      python3 scripts/node-artifact-check.py EP-000 M2
      python3 scripts/blueprint_validate.py
      python3 references/tests/test_ep000_contracts.py
      ;;
  M3)
      python3 scripts/node-artifact-check.py EP-000 M3
      python3 scripts/blueprint_validate.py
      python3 references/tests/test_ep000_contracts.py
      python3 references/tests/test_ep000_integration.py
      sh scripts/source-verify.sh
      ;;
  M4)
      python3 scripts/node-artifact-check.py EP-000 M4
      python3 scripts/blueprint_validate.py
      python3 references/tests/test_ep000_contracts.py
      sh scripts/security-check.sh
      sh scripts/license-gate.sh
      sh scripts/version-verify.sh
      ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-000 M5
      python3 scripts/blueprint_validate.py
      python3 references/tests/test_ep000_contracts.py
      sh scripts/source-verify.sh
      sh scripts/version-verify.sh
      sh scripts/node-verify.sh EP-000
      :
      ;;
  *) echo "EP-000: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-000 $mode: ok"
