#!/usr/bin/env sh
# RX-012 remediation battery: canary promotion authority truth
# (AUD-070 manual production promotion authority reducible to any
#  nonempty string -> real signed approval records with authenticated
#  approver, policy lookup, expiry, requester/approver separation,
#  record binding, and real Ed25519 signature verification).
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

# --- AUD-070: canary promotion suite with hostile proofs ---
out=$( (cd tests/release && node_modules/.bin/vitest run src/__tests__/ep042_unit_canary_promotion.test.ts) 2>&1 | sed 's/\x1b\[[0-9;]*m//g' || true)
n=$(echo "$out" | grep -Eo "Tests +[0-9]+ passed" | grep -Eo "[0-9]+" | head -1)
if [ "${n:-0}" -ge 23 ] && ! echo "$out" | grep -qE "failed"; then
  note "canary promotion suite ($n tests: signed approval records, bare-string denial, policy lookup, expiry, separation, signature tamper/wrong-key)"
else
  bad "canary promotion suite"
  echo "$out" | tail -25
fi

# --- hostile sentinels present (the AUD-070 proofs must exist) ---
for sentinel in \
  ep042_unit_approval_bare_string_is_not_authority \
  ep042_unit_approval_unauthorized_approver_denied \
  ep042_unit_approval_requester_approver_separation \
  ep042_unit_approval_expired_denied \
  ep042_unit_approval_future_dated_denied \
  ep042_unit_approval_tampered_signature_denied \
  ep042_unit_approval_wrong_key_denied \
  ep042_unit_approval_record_binding_denied \
  ep042_unit_manual_promotion_requires_signature; do
  if grep -rq "$sentinel" tests/release/src/__tests__/ep042_unit_canary_promotion.test.ts; then
    :
  else
    bad "hostile sentinel $sentinel missing"
  fi
done
[ "$fail" -eq 0 ] || { echo "$fail hostile sentinels missing"; exit 1; }
note "hostile AUD-070 sentinels present (bare-string/unauthorized/separation/expiry/future/tamper/wrong-key/binding/unsigned)"

# --- typechecks ---
if (cd apps/setup && node_modules/.bin/tsc --noEmit) >/tmp/rx012-tsc-setup.log 2>&1; then
  note "typecheck clean (apps/setup)"
else
  bad "typecheck (apps/setup)"
  tail -20 /tmp/rx012-tsc-setup.log
fi
if (cd tests/release && node_modules/.bin/tsc --noEmit -p tsconfig.json) >/tmp/rx012-tsc-rel.log 2>&1; then
  note "typecheck clean (tests/release)"
else
  bad "typecheck (tests/release)"
  tail -20 /tmp/rx012-tsc-rel.log
fi

# --- EP-042 M1/M2/M3/M4/M5 gates (regression surface) ---
for g in m1 m2 m3 m4 m5; do
  if SCOPE_AUDIT_DRIFT_ONLY=1 sh "scripts/ep042-$g-tests.sh" >"/tmp/rx012-ep042-$g.log" 2>&1; then
    note "EP-042 $g gate green"
  else
    bad "EP-042 $g gate"
    tail -15 "/tmp/rx012-ep042-$g.log"
  fi
done

# --- workspace check + clippy (security-check surface) ---
if cargo check --workspace >/tmp/rx012-check.log 2>&1; then
  note "workspace check clean"
else
  bad "workspace check (see /tmp/rx012-check.log)"
fi
if cargo clippy --workspace --all-targets --all-features --locked -- -D warnings >/tmp/rx012-clippy.log 2>&1; then
  note "workspace clippy clean (-D warnings)"
else
  bad "clippy (see /tmp/rx012-clippy.log)"
fi

# --- remediation register must pass (90/90, quarantine active) ---
if reg=$(bash .agent/remediation/verify-remediation-register.sh 2>&1); then
  note "remediation register: $(echo "$reg" | tail -1)"
else
  bad "remediation register"
fi

echo "---"
echo "RX-012 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
