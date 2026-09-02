#!/usr/bin/env sh
# RX-022 node verify - AUD-090 absorb/audit + AUD-075/089 certification gate.
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
    # 1. The RX-022 battery: AUD-090 surface + AUD-075/089/090 register proof.
    sh scripts/rx022-remediation-tests.sh
    # 2. Register 90/90 with quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-file audit.
    sh scripts/expected-files.sh RX-022
    echo "RX-022 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-022: FAIL - unknown mode $mode" >&2; exit 2;;
esac
