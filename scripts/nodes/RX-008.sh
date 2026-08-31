#!/usr/bin/env sh
# RX-008 node verify - runtime telemetry bootstrap (AUD-083) +
# canonical control-plane composition root (AUD-084).
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
    # 1. AUD-083/AUD-084 remediation battery.
    sh scripts/rx008-remediation-tests.sh
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-files audit for the RX-008 surface.
    sh scripts/expected-files.sh RX-008
    echo "RX-008 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-008: FAIL - unknown mode $mode" >&2; exit 2;;
esac
