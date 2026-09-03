#!/usr/bin/env sh
# RX-019 remediation battery: EP-019 IncidentEngine + EP-038 GlitchTip truth
# + EP-041 Microbrain promotion evidence (AUD-017 production IncidentEngine;
#  AUD-055 GlitchTip https:// DSNs sent over plaintext; AUD-056
#  correlation/trace context stripped before GlitchTip delivery; AUD-057
#  GlitchTip-outage quarantine process-local; AUD-058 acknowledge/resolve
#  no-ops; AUD-064 Microbrain promotion without declared evidence)
#
# The battery runs the REAL gates that own each repair surface:
#   - AUD-017 (M1)  -> EP-019 gates M1-M4 (nexus-healing engine contract,
#     incident workflow vitest, healing integration pytest, failure suite)
#   - AUD-055/056 (M2/M3) -> EP-038 M3 gate (real ephemeral GlitchTip 6.1.8
#     fixture, production adapter unit suite, real-provider integration,
#     stopped-provider phase) + EP-038 M4 gate (ops runtime unit suite)
#   - AUD-057 (M4)  -> EP-038 M4 gate (real provider failure suite +
#     ops runtime unit suite; durable quarantine proofs live in the ops lib)
#   - AUD-058 (M5)  -> EP-038 M3 gate (sink acknowledge/resolve state truth)
#   - AUD-064 (M6)  -> EP-041 M5 gate + aud064 declared-evidence proofs
# plus clippy/fmt on the touched crate surface.
set -eu
cd "$(dirname "$0")/.."
. ./scripts/env.sh

pass=0
fail=0
note() { echo "ok - $1"; pass=$((pass + 1)); }
bad() { echo "FAIL - $1"; fail=$((fail + 1)); }

# --- M1: AUD-017 production IncidentEngine (EP-019 gates) ---
for g in ep019-m1-tests.sh ep019-m2-tests.sh ep019-m3-tests.sh ep019-m4-tests.sh; do
  if timeout 500 sh "scripts/$g" >"/tmp/rx019-$g.log" 2>&1; then
    note "$g (AUD-017 IncidentEngine)"
  else
    bad "$g"
    tail -15 "/tmp/rx019-$g.log"
  fi
done

# --- M2/M3/M5: AUD-055 https TLS + AUD-056 correlation context
#               + AUD-058 ack/resolve state truth (EP-038 M3 gate) ---
if timeout 550 sh scripts/ep038-m3-tests.sh >/tmp/rx019-ep038-m3.log 2>&1; then
  note "ep038-m3-tests.sh (AUD-055 https TLS + AUD-056 correlation context + AUD-058 ack/resolve)"
else
  bad "ep038-m3-tests.sh"
  tail -20 /tmp/rx019-ep038-m3.log
fi

# --- M4: AUD-057 durable quarantine (EP-038 M4 gate) ---
if timeout 550 sh scripts/ep038-m4-tests.sh >/tmp/rx019-ep038-m4.log 2>&1; then
  note "ep038-m4-tests.sh (AUD-057 durable quarantine)"
else
  bad "ep038-m4-tests.sh"
  tail -20 /tmp/rx019-ep038-m4.log
fi

# --- M6: AUD-064 promotion requires declared evidence (EP-041 M5 gate + aud064) ---
if timeout 550 sh scripts/ep041-m5-tests.sh >/tmp/rx019-ep041-m5.log 2>&1; then
  note "ep041-m5-tests.sh (AUD-064 declared promotion/evaluation evidence gate)"
else
  bad "ep041-m5-tests.sh"
  tail -20 /tmp/rx019-ep041-m5.log
fi
# aud064 proofs run explicitly (the EP-041 gate filters ep041_unit_* only).
if timeout 300 uv run --frozen pytest tests/microbrain/test_aud064_promotion_evidence.py \
  -q --tb=short -o python_functions="aud064_*" >/tmp/rx019-aud064.log 2>&1; then
  aud064_count=$(grep -Eo '^[0-9]+ passed' /tmp/rx019-aud064.log | grep -Eo '[0-9]+' | head -1)
  if [ "${aud064_count:-0}" -lt 10 ]; then
    bad "aud064 proof count ${aud064_count:-0} < 10"
    tail -20 /tmp/rx019-aud064.log
  else
    note "aud064 proofs $aud064_count/10 (declared promotion evidence gate)"
  fi
else
  bad "aud064 proofs"
  tail -30 /tmp/rx019-aud064.log
fi
if grep -Eq '[1-9][0-9]* failed|[1-9][0-9]* error' /tmp/rx019-aud064.log; then
  bad "aud064 failures present"
fi

# --- clippy + fmt on the touched crate surface ---
if timeout 300 cargo clippy -p nexus-healing -p nexus-glitchtip \
  -p nexus-observability-ops -p nexus-observability \
  --all-targets --locked -- -D warnings >/tmp/rx019-clippy.log 2>&1; then
  note "clippy -D warnings clean"
else
  bad "clippy -D warnings"
  tail -15 /tmp/rx019-clippy.log
fi

if timeout 120 cargo fmt -p nexus-healing -p nexus-glitchtip \
  -p nexus-observability-ops -p nexus-observability \
  -- --check >/tmp/rx019-fmt.log 2>&1; then
  note "cargo fmt clean"
else
  bad "cargo fmt"
  tail -15 /tmp/rx019-fmt.log
fi

echo
echo "RX-019 battery: $pass ok, $fail fail"
[ "$fail" -eq 0 ]
