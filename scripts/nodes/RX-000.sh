#!/usr/bin/env sh
# RX-000 node verify - remediation baseline and release quarantine.
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
    bash .agent/remediation/verify-remediation-register.sh
    # quarantine must be active: generation 2, release not allowed
    grep -q '^REMEDIATION_GENERATION=2' .agent/remediation/REMEDIATION_STATE.env \
      || { echo "RX-000: FAIL - REMEDIATION_GENERATION != 2" >&2; exit 1; }
    grep -q '^RELEASE_ALLOWED=false' .agent/remediation/REMEDIATION_STATE.env \
      || { echo "RX-000: FAIL - RELEASE_ALLOWED != false" >&2; exit 1; }
    sh scripts/expected-files.sh RX-000
    echo "RX-000 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-000: FAIL - unknown mode $mode" >&2; exit 2;;
esac
