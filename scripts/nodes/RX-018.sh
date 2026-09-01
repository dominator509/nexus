#!/usr/bin/env sh
# RX-018 node verify - artifact storage + DR truth
# (AUD-049 Local/NAS tenant boundary on a shared root; AUD-050 shared-content
#  delete preserves still-referenced objects; AUD-051 encryption-before-egress
#  verified against real ciphertext, not metadata alone; AUD-052 signed backup
#  manifests; AUD-015 self-contained DR bundle; AUD-053 destroyed-host DR proof)
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
    # 1. AUD-049..051 remediation battery (real suites + live-fire gates).
    sh scripts/rx018-remediation-tests.sh
    # 2. Expected-files audit for the RX-018 surface.
    sh scripts/expected-files.sh RX-018
    echo "RX-018 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-018: FAIL - unknown mode $mode" >&2; exit 2;;
esac
