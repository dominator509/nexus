#!/usr/bin/env sh
# RX-017 node verify - onboarding/provisioning truth
# (AUD-042 Setup/secure-enrollment product via EP-035 gates + LF-001 live-fire;
#  AUD-043 enrollment claim binds bootstrap secret in the same atomic UPDATE;
#  AUD-044 owner ladder enforced at the durable boundary; AUD-045 retry-safe
#  reconciliation requires explicit negative mutation observation;
#  AUD-046 GenericSshProvider real transport; AUD-047 placement requires
#  observed capacity + health; AUD-048 cost ceiling enforced)
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
    # 1. AUD-042..048 remediation battery (real suites + live-fire gates).
    sh scripts/rx017-remediation-tests.sh
    # 2. Expected-files audit for the RX-017 surface.
    sh scripts/expected-files.sh RX-017
    echo "RX-017 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-017: FAIL - unknown mode $mode" >&2; exit 2;;
esac
