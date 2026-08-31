#!/usr/bin/env sh
# RX-010 remediation battery: release integrity truth
# (AUD-076 release tag readiness, AUD-077 artifact SHA-256 over real
#  bytes, AUD-078 manifest_digest binds nested component state,
#  AUD-079 release-evidence digest binds nested certification/drill/
#  review/capability state).
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

# --- AUD-076/077/078/079: full release-evidence suite with hostile proofs ---
out=$( (cd release-evidence && node_modules/.bin/vitest run src/__tests__) 2>&1 | sed 's/\x1b\[[0-9;]*m//g' || true)
n=$(echo "$out" | grep -Eo "Tests +[0-9]+ passed" | grep -Eo "[0-9]+" | head -1)
if [ "${n:-0}" -ge 150 ] && ! echo "$out" | grep -qE "failed"; then
  note "release-evidence suite ($n tests: release tag truth, raw-byte artifact digests, nested manifest digest binding, nested evidence digest binding)"
else
  bad "release-evidence suite"
  echo "$out" | tail -25
fi

# --- typecheck clean (release-evidence) ---
if (cd release-evidence && node_modules/.bin/tsc --noEmit) >/tmp/rx010-tsc.log 2>&1; then
  note "typecheck clean (release-evidence)"
else
  bad "typecheck"
  tail -20 /tmp/rx010-tsc.log
fi

# --- EP-043 M1/M2/M3/M4 gates (fast regressions) ---
for g in m1 m2 m3 m4; do
  if sh "scripts/ep043-$g-tests.sh" >"/tmp/rx010-ep043-$g.log" 2>&1; then
    note "EP-043 $g gate green"
  else
    bad "EP-043 $g gate"
    tail -15 "/tmp/rx010-ep043-$g.log"
  fi
done

# --- EP-043 M5 gate: rollback drill + fresh-clone acceptance + readiness ---
if sh scripts/ep043-m5-tests.sh >/tmp/rx010-ep043-m5.log 2>&1; then
  note "EP-043 M5 gate green (rollback drill, fresh-clone acceptance, readiness rerun)"
else
  bad "EP-043 M5 gate"
  tail -15 /tmp/rx010-ep043-m5.log
fi

# --- workspace check + clippy (security-check surface) ---
if cargo check --workspace >/tmp/rx010-check.log 2>&1; then
  note "workspace check clean"
else
  bad "workspace check (see /tmp/rx010-check.log)"
fi
if cargo clippy --workspace --all-targets --all-features --locked -- -D warnings >/tmp/rx010-clippy.log 2>&1; then
  note "workspace clippy clean (-D warnings)"
else
  bad "clippy (see /tmp/rx010-clippy.log)"
fi

# --- remediation register must pass (90/90, quarantine active) ---
if reg=$(bash .agent/remediation/verify-remediation-register.sh 2>&1); then
  note "remediation register: $(echo "$reg" | tail -1)"
else
  bad "remediation register"
fi

echo "---"
echo "RX-010 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
