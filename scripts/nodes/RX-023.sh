#!/usr/bin/env sh
# RX-023 node verify - final graph closure gate.
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
    # 1. The RX-023 battery: register 90/90 + all nodes V2-DONE + ALL_DONE.
    sh scripts/rx023-remediation-tests.sh
    # 2. Register 90/90 with quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-file audit.
    sh scripts/expected-files.sh RX-023
    echo "RX-023 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-023: FAIL - unknown mode $mode" >&2; exit 2;;
esac
