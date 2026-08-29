#!/usr/bin/env sh
# RX-004 node verify - build, test and reality-gate truth.
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
    # 1. Regression battery: build/security/consecutive-verify truth.
    sh scripts/rx004-build-test-reality-tests.sh
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Full canonical verification ladder.
    sh scripts/verify.sh
    sh scripts/expected-files.sh RX-004
    echo "RX-004 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-004: FAIL - unknown mode $mode" >&2; exit 2;;
esac
