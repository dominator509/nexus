#!/usr/bin/env sh
# RX-020 node verify - register-wide P0/P1 closure gate.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
mode="${1:-verify}"
case "$mode" in
  verify)
    # 1. The RX-020 battery: upstream batteries fresh + register-wide closure.
    sh scripts/rx020-remediation-tests.sh
    # 2. Register 90/90 with quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-file audit.
    sh scripts/expected-files.sh RX-020
    echo "RX-020 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-020: FAIL - unknown mode $mode" >&2; exit 2;;
esac
