#!/usr/bin/env sh
# RX-011 node verify - offline update/rollback truth (AUD-066 rollback
# digest verification, AUD-067 atomic switch preserves current install,
# AUD-068 installer payloads bound to validated release manifest,
# AUD-069 durable idempotency/duplicate guard).
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
    # 1. AUD-066/AUD-067/AUD-068/AUD-069 remediation battery.
    sh scripts/rx011-remediation-tests.sh
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-files audit for the RX-011 surface.
    sh scripts/expected-files.sh RX-011
    echo "RX-011 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-011: FAIL - unknown mode $mode" >&2; exit 2;;
esac
