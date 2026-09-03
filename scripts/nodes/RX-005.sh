#!/usr/bin/env sh
# RX-005 node verify - remediation truth: PostgreSQL/NATS adapters +
# Temporal retry classification (AUD-007, AUD-008, AUD-023).
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
    # 1. Remediation battery: real live-fire adapters + retry classification.
    sh scripts/rx005-remediation-tests.sh
    echo "verify: ok"
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-files audit for the RX-005 surface.
    sh scripts/expected-files.sh RX-005
    echo "RX-005 verify: ok"
    ;;
  *)
    echo "RX-005 node: unknown mode $mode" >&2
    exit 1
    ;;
esac
