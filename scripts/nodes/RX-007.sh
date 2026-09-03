#!/usr/bin/env sh
# RX-007 node verify - sandboxed skill execution truth (AUD-011) +
# bounded, deadlock-free subprocess execution (AUD-022).
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
    # 1. AUD-011/AUD-022 remediation battery: real OS sandbox hostile
    #    probes + concurrent-drain/deadline hostile regressions.
    sh scripts/rx007-remediation-tests.sh
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-files audit for the RX-007 surface.
    sh scripts/expected-files.sh RX-007
    echo "RX-007 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-007: FAIL - unknown mode $mode" >&2; exit 2;;
esac
