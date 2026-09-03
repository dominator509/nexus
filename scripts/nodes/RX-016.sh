#!/usr/bin/env sh
# RX-016 node verify - desktop/mobile client truth
# (AUD-038 actual React PWA entry; AUD-039 desktop high-risk authorization
#  resolves approval class from the registered capability profile, never the
#  wire; AUD-040 real native mobile security channel Android/iOS;
#  AUD-041 step-up enforcement + identity/tenant binding VERIFIED_FIXED)
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
    # 1. AUD-038..041 remediation battery (real suites + live-fire gates).
    sh scripts/rx016-remediation-tests.sh
    # 2. Expected-files audit for the RX-016 surface.
    sh scripts/expected-files.sh RX-016
    echo "RX-016 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-016: FAIL - unknown mode $mode" >&2; exit 2;;
esac
