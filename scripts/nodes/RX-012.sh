#!/usr/bin/env sh
# RX-012 node verify - canary promotion authority truth (AUD-070 manual
# production promotion authority: real signed approval records with
# authenticated approver, policy lookup, expiry, requester/approver
# separation, record binding - never a bare approval string).
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
    # 1. AUD-070 remediation battery.
    sh scripts/rx012-remediation-tests.sh
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-files audit for the RX-012 surface.
    sh scripts/expected-files.sh RX-012
    echo "RX-012 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-012: FAIL - unknown mode $mode" >&2; exit 2;;
esac
