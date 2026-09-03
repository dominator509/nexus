#!/usr/bin/env sh
# RX-009 node verify - cryptographic supply-chain evidence sealing (AUD-059),
# multi-ecosystem shipped-product SBOM inventory (AUD-060), cryptographic
# release-bundle signature verification (AUD-065).
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
    # 1. AUD-059/AUD-060/AUD-065 remediation battery.
    sh scripts/rx009-remediation-tests.sh
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-files audit for the RX-009 surface.
    sh scripts/expected-files.sh RX-009
    echo "RX-009 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-009: FAIL - unknown mode $mode" >&2; exit 2;;
esac
