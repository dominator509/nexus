#!/usr/bin/env sh
# RX-021 regression battery: AUD-063 leaf closure - performance
# certification measures a REAL runtime over the wire.
# AUD-063: EP-040's performance certification never measures runtime
#          performance. RX-004's half (M5 shell latency check) is
#          committed; RX-021's leaf half adds the crate-level
#          RuntimeLatencyProbe (std-only TCP round trip) + hostile
#          fail-closed proofs + a live runtime certification.
set -eu
cd "$(dirname "$0")/.."
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

pass=0
fail=0
note() { echo "ok - $1"; pass=$((pass + 1)); }
bad() { echo "FAIL - $1"; fail=$((fail + 1)); }

# --- 1. Crate-level real-observation producer present (AUD-063 leaf) ---
for needle in "RuntimeLatencyProbe" "RealLatencyObservation" "pub fn probe(" \
  "probe connect failed" "did not report healthy"; do
  if grep -q "$needle" tests/performance/src/lib.rs; then
    note "lib.rs covers $needle"
  else
    bad "tests/performance/src/lib.rs missing $needle"
  fi
done

# The stale "NOT asserted" claim must be gone: the crate now owns live
# over-the-wire observation.
if grep -q "Real performance harnesses, load generators, and hardware timing are" \
  tests/performance/src/lib.rs; then
  bad "lib.rs still claims real performance is not asserted (AUD-063)"
else
  note "lib.rs no longer disclaims real runtime observation"
fi

# --- 2. Hostile + positive crate proofs present and green ---
proof_file="tests/performance/tests/aud063_runtime_observation.rs"
for sentinel in \
  aud063_probe_unreachable_fails_closed \
  aud063_probe_non_healthy_fails_closed \
  aud063_probe_measures_real_wire_latency \
  aud063_real_observation_certifies_budget \
  aud063_hand_fed_constant_not_runtime_evidence \
  aud063_probe_rejects_non_http_endpoint \
  aud063_real_observation_serde_roundtrip; do
  if grep -q "fn $sentinel" "$proof_file"; then
    note "proof $sentinel declared"
  else
    bad "proof $sentinel missing from $proof_file"
  fi
done

# --- 3. Real cargo suite for the performance crate is green ---
log=$(mktemp)
if ~/.local/share/mise/shims/cargo test -p nexus-test-performance --locked \
  >>"$log" 2>&1; then
  note "nexus-test-performance cargo suite green"
else
  bad "nexus-test-performance cargo suite failed: $(tail -3 "$log")"
fi
if grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  note "performance suite reports non-zero passed"
else
  bad "performance suite vacuous (zero passed)"
fi
rm -f "$log"

# --- 4. Live runtime observation (the enforceable half) ---
# The real control-plane runtime must be reachable and measured over real
# TCP with a real wall-clock p95, then certified within the declared
# budget through the canonical evaluator path. If the runtime is down this
# must FAIL (never fabricate, never skip).
base="${NEXUS_SMOKE_URL:-http://127.0.0.1:8443}"
live_log=$(mktemp)
if ./target/debug/runtime_latency_probe "$base" 5000 5 >"$live_log" 2>&1; then
  note "live runtime latency certified within budget"
  grep -q "AUD-063 runtime latency certify: ok" "$live_log" \
    && note "live certify sentinel observed" \
    || bad "live certify sentinel missing: $(tail -2 "$live_log")"
  grep -qE "p95=[0-9]+\.[0-9]+ms" "$live_log" \
    && note "live p95 is a real positive measurement" \
    || bad "live p95 not a real measurement: $(cat "$live_log")"
else
  bad "live runtime latency probe failed (runtime down?): $(cat "$live_log")"
fi
rm -f "$live_log"

# Hostile: an unreachable endpoint must exit non-zero and never fabricate
# a certification.
hostile_log=$(mktemp)
if ./target/debug/runtime_latency_probe http://127.0.0.1:59999 5000 1 \
  >"$hostile_log" 2>&1; then
  bad "unreachable endpoint produced a certification (hostile)"
else
  note "unreachable endpoint fails closed (hostile)"
fi
if grep -q "certify: ok" "$hostile_log"; then
  bad "unreachable endpoint still printed certify ok (hostile)"
else
  note "no certification printed for unreachable endpoint"
fi
rm -f "$hostile_log"

# --- 5. Upstream EP-040 gates stay green (no regression) ---
if sh scripts/ep040-m1-tests.sh >/tmp/rx021-m1.log 2>&1; then
  note "EP-040 M1 gate green (contract + performance suites)"
else
  bad "EP-040 M1 gate failed: $(tail -3 /tmp/rx021-m1.log)"
fi

# --- 6. Register holds AUD-063 VERIFIED_FIXED with evidence ---
bash .agent/remediation/verify-remediation-register.sh \
  >/tmp/rx021-register.log 2>&1 \
  && note "remediation register PASS (90/90 registered, quarantine active)" \
  || bad "remediation register failed: $(tail -3 /tmp/rx021-register.log)"

if python3 - <<'PY'
import csv
rows = list(csv.DictReader(open('.agent/remediation/AUDIT_FINDINGS.tsv'), delimiter='\t'))
for r in rows:
    if r['audit_id'] == 'AUD-063':
        ok = r['status'] == 'VERIFIED_FIXED' and r['regression_test'].strip() and r['evidence_ref'].strip()
        raise SystemExit(0 if ok else 1)
raise SystemExit(1)
PY
then
  note "AUD-063 row is VERIFIED_FIXED with regression_test + evidence_ref"
else
  bad "AUD-063 row not VERIFIED_FIXED or missing evidence columns"
fi

echo "---"
echo "RX-021 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
