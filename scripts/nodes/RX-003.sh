#!/usr/bin/env sh
# RX-003 node verify - GitHub/default-branch/CI authority.
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
    # 1. Regression battery: branch alignment, SHA pinning, release surface.
    sh scripts/rx003-ci-authority-tests.sh
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Full canonical verification ladder.
    sh scripts/verify.sh
    sh scripts/expected-files.sh RX-003
    echo "RX-003 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-003: FAIL - unknown mode $mode" >&2; exit 2;;
esac
