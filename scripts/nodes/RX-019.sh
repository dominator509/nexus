#!/usr/bin/env sh
# RX-019 node verify - EP-019 IncidentEngine + EP-038 GlitchTip truth
# (AUD-017 production IncidentEngine implementation; AUD-055 GlitchTip
#  https:// DSNs silently sent over plaintext; AUD-056 incident
#  correlation/trace context stripped before GlitchTip delivery;
#  AUD-057 GlitchTip-outage quarantine only process-local)
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
    # 1. AUD-017/055/056/057 remediation battery (real suites + gates).
    sh scripts/rx019-remediation-tests.sh
    # 2. Expected-files audit for the RX-019 surface.
    sh scripts/expected-files.sh RX-019
    echo "RX-019 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-019: FAIL - unknown mode $mode" >&2; exit 2;;
esac
