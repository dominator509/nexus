#!/usr/bin/env sh
# RX-013 node verify - release manifest + deploy handoff truth
# (AUD-082 release manifest bound to REAL product artifacts not fixture
#  strings; AUD-081 deploy.sh is a real deploy command through the
#  transactional installer).
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
    # 1. AUD-082/AUD-081 remediation battery.
    sh scripts/rx013-remediation-tests.sh
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-files audit for the RX-013 surface.
    sh scripts/expected-files.sh RX-013
    echo "RX-013 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-013: FAIL - unknown mode $mode" >&2; exit 2;;
esac
