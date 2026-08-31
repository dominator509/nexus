#!/usr/bin/env sh
# RX-010 node verify - release integrity truth (AUD-076 release tag
# readiness, AUD-077 artifact SHA-256 over real bytes, AUD-078 manifest
# digest binds nested component state, AUD-079 evidence digest binds
# nested certification/drill/review/capability state).
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
    # 1. AUD-076/AUD-077/AUD-078/AUD-079 remediation battery.
    sh scripts/rx010-remediation-tests.sh
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-files audit for the RX-010 surface.
    sh scripts/expected-files.sh RX-010
    echo "RX-010 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-010: FAIL - unknown mode $mode" >&2; exit 2;;
esac
