#!/usr/bin/env sh
# RX-015 node verify - network defense truth
# (AUD-025 immutable approval receipt; AUD-026 quarantine binds observed
#  network identity; AUD-027 AdGuard configured blocklist; AUD-028 production
#  NetworkInventory; AUD-029 owner notification; AUD-030 Suricata profile;
#  AUD-031 preauthorization truth; AUD-032 same-indicator confidence;
#  AUD-033 verifier proposal binding; AUD-034 Zeek minute truth;
#  AUD-035 osquery durable endpoint identity; AUD-036 osquery REAL TLS;
#  AUD-037 osquery observation time + collision-proof event ids)
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
    # 1. AUD-025..037 remediation battery (runs the real suites + LF-009/LF-010).
    sh scripts/rx015-remediation-tests.sh
    # 2. Expected-files audit for the RX-015 surface.
    sh scripts/expected-files.sh RX-015
    echo "RX-015 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-015: FAIL - unknown mode $mode" >&2; exit 2;;
esac
