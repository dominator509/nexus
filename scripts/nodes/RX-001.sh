#!/usr/bin/env sh
# RX-001 node verify — GraphLock V2 completion authority.
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
    # 1. Hostile battery: forged ledger/tag/evidence cannot certify closure.
    sh scripts/rx001-graphlock-v2-tests.sh
    # 2. Remediation register must remain 90/90 and quarantine active.
    bash .agent/remediation/verify-remediation-register.sh
    # 3. AUD-080 non-reproducibility: the EP-043 closure gate MUST fail while
    #    readiness is NOT_READY (exit non-zero with the AUD-080 reason).
    if sh scripts/ep043-m5-tests.sh >/tmp/rx001-aud080-gate.log 2>&1; then
      echo "RX-001: FAIL - EP-043 M5 gate exited 0 while readiness is NOT_READY (AUD-080 reproducible)" >&2
      exit 1
    fi
    if ! grep -q "closure gate: readiness is NOT_READY" /tmp/rx001-aud080-gate.log; then
      echo "RX-001: FAIL - gate failed for wrong reason (AUD-080 not demonstrated)" >&2
      tail -20 /tmp/rx001-aud080-gate.log >&2
      exit 1
    fi
    # 4. Full canonical verification ladder.
    sh scripts/verify.sh
    sh scripts/expected-files.sh RX-001
    echo "RX-001 verify: ok"
    echo "verify: ok"
    ;;
  *) echo "RX-001: FAIL - unknown mode $mode" >&2; exit 2;;
esac
