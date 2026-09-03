#!/usr/bin/env sh
# RX-021 node verify - AUD-063 leaf closure gate (real runtime observation).
# AUD-063: EP-040's performance certification never measures runtime
#          performance (shared RX-004/RX-021; RX-004's M5 shell half
#          committed; RX-021's leaf half = crate-level real observation
#          producer + hostile proofs + live runtime evidence).
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
    # 1. The RX-021 battery: crate proofs + live runtime observation.
    sh scripts/rx021-remediation-tests.sh
    # 2. Register 90/90 with quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-file audit.
    sh scripts/expected-files.sh RX-021
    echo "RX-021 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-021: FAIL - unknown mode $mode" >&2; exit 2;;
esac
