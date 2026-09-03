#!/usr/bin/env sh
# RX-020 remediation battery: register-wide P0/P1 closure gate.
#
# RX-020 is a structural node: it owns no AUD finding directly. Its
# contract (graph-next-v2.sh) is that NO P0/P1 finding register-wide may
# remain not-VERIFIED_FIXED before the EP-042/043/044 tail nodes
# (RX-021/022/023) may schedule. The pre-RX-020 state had 10 blockers:
# 9 stale FIXED_UNVERIFIED rows whose fixing nodes were already DONE with
# green attestation (AUD-001/054/071/072/073/074/075/087/089), plus the
# genuinely OPEN AUD-090 (sole owner RX-022, downstream - sequencing
# trap). This battery re-proves the live evidence for every one of those
# rows on current HEAD, then asserts the register-wide invariant the
# scheduler enforces:
#   - register 90/90 with valid ids/owners/statuses,
#   - ZERO P0/P1 rows that are not VERIFIED_FIXED,
#   - quarantine still active (generation 2, release not allowed),
#   - every VERIFIED_FIXED row carries regression_test + evidence_ref.
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

REPO=$(pwd)

# --- 1. upstream batteries for the flipped shared rows (fresh on HEAD) ---
run_battery() {
  name="$1"; script="$2"
  if [ -x "$script" ] || [ -f "$script" ]; then
    if sh "$script" >"/tmp/rx020-$name.log" 2>&1; then
      note "$name battery green on HEAD"
    else
      bad "$name battery failed (see /tmp/rx020-$name.log)"
      tail -15 "/tmp/rx020-$name.log" >&2 2>/dev/null || true
    fi
  else
    bad "missing battery $script"
  fi
}

# AUD-001 (RX-001/RX-016) + AUD-085 hostiles
run_battery rx001 scripts/rx001-graphlock-v2-tests.sh
# AUD-071/072/073/074/075/087 (+ AUD-002/003/004 evidence truth)
if (cd release-evidence && npx vitest run) >/tmp/rx020-rx002.log 2>&1; then
  note "RX-002 release-evidence vitest green on HEAD"
else
  bad "RX-002 release-evidence vitest failed (see /tmp/rx020-rx002.log)"
  tail -15 /tmp/rx020-rx002.log >&2 2>/dev/null || true
fi
# AUD-089 (RX-003) + ci-authority hostiles
run_battery rx003 scripts/rx003-ci-authority-tests.sh
# AUD-054 (RX-008/RX-019) battery
run_battery rx008 scripts/rx008-remediation-tests.sh

# --- 2. AUD-090 pre-executed proof (fresh-clone ship-standard run) ---
run_battery aud090 scripts/aud090-freshclone-ship-tests.sh

# --- 3. register-wide invariants ---------------------------------------------
reg=$(bash .agent/remediation/verify-remediation-register.sh 2>&1 || true)
case "$reg" in
  *"VERIFY_REMEDIATION_REGISTER: PASS"*)
    note "remediation register PASS (90/90 registered, quarantine active)" ;;
  *) bad "remediation register: $reg" ;;
esac

# --- 4. the RX-020 scheduler gate recomputed exactly as graph-next-v2.sh ---
open_p1=$(python3 - .agent/remediation/AUDIT_FINDINGS.tsv <<'PY'
import csv, sys
n = 0
with open(sys.argv[1], newline="") as fh:
    for row in csv.DictReader(fh, delimiter="\t"):
        if row["severity"] in ("P0", "P1") and row["status"] != "VERIFIED_FIXED":
            n += 1
print(n)
PY
)
if [ "$open_p1" = "0" ]; then
  note "zero P0/P1 findings remain not-VERIFIED_FIXED (scheduler gate recomputed)"
else
  bad "P0/P1 findings not verified fixed: $open_p1"
fi

# --- 5. every VERIFIED_FIXED row carries regression test + evidence ref -------
python3 - .agent/remediation/AUDIT_FINDINGS.tsv <<'PY'
import csv, sys
missing = []
with open(sys.argv[1], newline="") as fh:
    for row in csv.DictReader(fh, delimiter="\t"):
        if row["status"] == "VERIFIED_FIXED":
            if not row.get("regression_test", "").strip():
                missing.append(row["audit_id"] + ":no-regression-test")
            if not row.get("evidence_ref", "").strip():
                missing.append(row["audit_id"] + ":no-evidence-ref")
if missing:
    raise SystemExit("missing: " + ", ".join(missing))
PY
if [ $? -eq 0 ]; then
  note "every VERIFIED_FIXED row carries regression_test + evidence_ref"
else
  bad "VERIFIED_FIXED rows missing regression_test/evidence_ref"
fi

# --- 6. quarantine still active (release remains blocked) ----------------------
grep -q '^REMEDIATION_GENERATION=2' .agent/remediation/REMEDIATION_STATE.env \
  && grep -q '^RELEASE_ALLOWED=false' .agent/remediation/REMEDIATION_STATE.env \
  && note "quarantine active: generation 2, release not allowed" \
  || bad "quarantine state not active"

echo "---"
echo "RX-020 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
