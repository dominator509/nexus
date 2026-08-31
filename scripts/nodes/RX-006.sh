#!/usr/bin/env sh
# RX-006 node verify - Headscale mesh identity truth (AUD-012).
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
    # 1. AUD-012 remediation battery: identity binding, resolvable
    #    references, real X25519 + OpenBao live-fire, hostile regressions.
    sh scripts/rx006-remediation-tests.sh
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-files audit for the RX-006 surface.
    sh scripts/expected-files.sh RX-006
    echo "RX-006 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-006: FAIL - unknown mode $mode" >&2; exit 2;;
esac
