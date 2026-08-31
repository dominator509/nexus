#!/usr/bin/env sh
# RX-014 node verify - provider adapter truth
# (AUD-009 Gmail wire-format + draft-id recipient; AUD-010 real attachment
#  enumeration Gmail/Graph/IMAP; AUD-018 delivery policy enforced;
#  AUD-019 router destination-aware SMS; AUD-020 ICTFax destination/document
#  binding; AUD-021 consent before recording; AUD-024 X reply binds the
#  mention thread via the official reply object).
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
    # 1. AUD-009/010/018/019/020/021/024 remediation battery.
    sh scripts/rx014-remediation-tests.sh
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. Expected-files audit for the RX-014 surface.
    sh scripts/expected-files.sh RX-014
    echo "RX-014 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-014: FAIL - unknown mode $mode" >&2; exit 2;;
esac
