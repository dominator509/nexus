#!/usr/bin/env sh
# RX-011 remediation battery: offline update/rollback truth
# (AUD-066 rollback digest verification, AUD-067 atomic switch preserves
#  the current install, AUD-068 installer payloads bound to the validated
#  release manifest, AUD-069 durable idempotency/duplicate guard).
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

# --- AUD-066/067/068/069: installer failure suite with hostile proofs ---
out=$( (cd tests/release && node_modules/.bin/vitest run --config vitest.failure.config.ts) 2>&1 | sed 's/\x1b\[[0-9;]*m//g' || true)
n=$(echo "$out" | grep -Eo "Tests +[0-9]+ passed" | grep -Eo "[0-9]+" | head -1)
if [ "${n:-0}" -ge 27 ] && ! echo "$out" | grep -qE "failed"; then
  note "installer failure suite ($n tests: rollback digest deny, cross-device atomic switch preserve, manifest-bound payloads, idempotency guard)"
else
  bad "installer failure suite"
  echo "$out" | tail -25
fi

# --- hostile sentinels present (the AUD-066..069 proofs must exist) ---
for sentinel in \
  ep042_failure_rollback_wrong_digest_source_denied \
  ep042_failure_atomic_switch_cross_device_preserves_install \
  ep042_failure_component_digest_not_bound_to_manifest \
  ep042_failure_extra_component_not_declared_denied \
  ep042_failure_release_id_not_bound_to_manifest \
  ep042_failure_foreign_journal_owner_denied \
  ep042_failure_duplicate_request_conflict; do
  if grep -rq "$sentinel" tests/release/src/failure/; then
    :
  else
    bad "hostile sentinel $sentinel missing"
  fi
done
[ "$fail" -eq 0 ] || { echo "$fail hostile sentinels missing"; exit 1; }
note "hostile AUD-066/067/068/069 sentinels present"

# --- unit suite + typechecks ---
if (cd installers && node_modules/.bin/vitest run src/__tests__) >/tmp/rx011-unit.log 2>&1; then
  note "installer unit suite green"
else
  bad "installer unit suite"
  tail -15 /tmp/rx011-unit.log
fi
if (cd installers && node_modules/.bin/tsc --noEmit) >/tmp/rx011-tsc.log 2>&1; then
  note "typecheck clean (installers)"
else
  bad "typecheck (installers)"
  tail -20 /tmp/rx011-tsc.log
fi
if (cd tests/release && node_modules/.bin/tsc --noEmit -p tsconfig.json) >/tmp/rx011-tsc-rel.log 2>&1; then
  note "typecheck clean (tests/release)"
else
  bad "typecheck (tests/release)"
  tail -20 /tmp/rx011-tsc-rel.log
fi

# --- EP-042 M1/M2/M3/M4/M5 gates (regression surface for the installer) ---
for g in m1 m2 m3 m4 m5; do
  if SCOPE_AUDIT_DRIFT_ONLY=1 sh "scripts/ep042-$g-tests.sh" >"/tmp/rx011-ep042-$g.log" 2>&1; then
    note "EP-042 $g gate green"
  else
    bad "EP-042 $g gate"
    tail -15 "/tmp/rx011-ep042-$g.log"
  fi
done

# --- workspace check + clippy (security-check surface) ---
if cargo check --workspace >/tmp/rx011-check.log 2>&1; then
  note "workspace check clean"
else
  bad "workspace check (see /tmp/rx011-check.log)"
fi
if cargo clippy --workspace --all-targets --all-features --locked -- -D warnings >/tmp/rx011-clippy.log 2>&1; then
  note "workspace clippy clean (-D warnings)"
else
  bad "clippy (see /tmp/rx011-clippy.log)"
fi

# --- remediation register must pass (90/90, quarantine active) ---
if reg=$(bash .agent/remediation/verify-remediation-register.sh 2>&1); then
  note "remediation register: $(echo "$reg" | tail -1)"
else
  bad "remediation register"
fi

echo "---"
echo "RX-011 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
