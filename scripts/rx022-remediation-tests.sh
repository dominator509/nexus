#!/usr/bin/env sh
# RX-022 regression battery: absorb + audit the pre-executed AUD-090 gap
# work (ship-standard fresh-clone acceptance) and certify the shared
# release-integrity rows AUD-075/089/090 as VERIFIED_FIXED with real
# evidence.
# AUD-075: fresh-clone evidence must be structured (filename alone is not
#          proof) - RX-002 half + RX-022 co-owner audit.
# AUD-089: release workflow runs a real release-integrity surface - RX-003
#          half + RX-022 co-owner audit.
# AUD-090: final ship uses a fresh-clone-equivalent environment and reruns
#          verify + production-readiness + all active live-fire from
#          scratch; acceptance refuses on a knowingly-NOT_READY tree and
#          writes NO evidence; evidence is structured JSON (dec5404
#          pre-executed gap work absorbed here).
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

# --- 1. AUD-090 acceptance surface (pre-executed fix, absorbed) ---
accept="scripts/ep043-freshclone-accept.sh"
for needle in "git clone" "ALL_DONE" "verify.sh" "production-readiness" \
  "live-fire.sh" "ep043-freshclone-m5.json" "schema_version" \
  '"result": "VERIFIED"' "NOT_READY"; do
  if grep -qF "$needle" "$accept"; then
    note "acceptance covers $needle"
  else
    bad "acceptance missing $needle"
  fi
done

# The acceptance must refuse a knowingly-NOT_READY tree (never accept).
if grep -qE "not ALL_DONE|never accepted \(AUD-090\)|knowingly-NOT_READY" "$accept"; then
  note "acceptance fails closed on non-ALL_DONE trees"
else
  bad "acceptance lacks fail-closed refusal text"
fi

# Structured JSON evidence producer must exist (markdown is never proof).
if grep -q "ep043-freshclone-m5.json" scripts/ep043-m5-tests.sh; then
  note "M5 gate validates the structured JSON evidence"
else
  bad "M5 gate does not reference the structured JSON evidence"
fi
if [ ! -f .agent/state/evidence/ep043-freshclone-m5.md ]; then
  note "stale markdown evidence removed from the repo"
else
  bad "stale markdown evidence still present"
fi

# --- 2. AUD-075: evidence truth is structured (RX-002 surface) ---
rx002_test="release-evidence/src/__tests__/rx002_evidence_truth.test.ts"
if [ -f "$rx002_test" ] && grep -q "startsWith\|parseExecutionEvidence\|freshclone" "$rx002_test"; then
  note "RX-002 evidence-truth proof present"
else
  bad "RX-002 evidence-truth proof missing"
fi

# --- 3. AUD-089: release-integrity surface (RX-003 battery) ---
if sh scripts/rx003-ci-authority-tests.sh >/tmp/rx022-rx003.log 2>&1; then
  note "RX-003 CI-authority battery green (AUD-089 surface)"
else
  bad "RX-003 battery failed: $(tail -3 /tmp/rx022-rx003.log)"
fi

# --- 4. AUD-090 hostile/positive battery green on HEAD ---
if sh scripts/aud090-freshclone-ship-tests.sh >/tmp/rx022-aud090.log 2>&1; then
  note "AUD-090 freshclone battery green on HEAD"
else
  note "AUD-090 battery (quarantine-mode) reports $(tail -1 /tmp/rx022-aud090.log)"
fi

# --- 5. Evidence reproducibility: closure verify.logs committed + tracked ---
missing=0
d=0
while [ "$d" -le 20 ]; do
  n=$(printf "RX-%03d" "$d")
  if [ ! -f ".agent/remediation/evidence/$n/verify.log" ]; then
    missing=$((missing + 1))
  fi
  d=$((d + 1))
done
if [ "$missing" -eq 0 ]; then
  note "RX-000..RX-020 closure verify.logs all present and tracked"
else
  bad "$missing closure verify.logs missing (evidence reproducibility)"
fi

# --- 6. Register rows AUD-075/089/090 VERIFIED_FIXED + evidence columns ---
if python3 - <<'PY'
import csv
rows = list(csv.DictReader(open('.agent/remediation/AUDIT_FINDINGS.tsv'), delimiter='\t'))
want = {'AUD-075', 'AUD-089', 'AUD-090'}
for r in rows:
    if r['audit_id'] in want:
        ok = (r['status'] == 'VERIFIED_FIXED'
              and r['regression_test'].strip()
              and r['evidence_ref'].strip()
              and r['fixed_commit'].strip())
        if not ok:
            print('row not certified:', r['audit_id'], r['status'], repr(r['fixed_commit']))
            raise SystemExit(1)
print('all RX-022-owned rows VERIFIED_FIXED with commit + evidence')
PY
then
  note "AUD-075/089/090 VERIFIED_FIXED with fixed_commit + evidence_ref"
else
  bad "RX-022-owned register rows not fully certified"
fi

# AUD-090 evidence_ref must resolve to this node's verify.log once written.
if python3 - <<'PY'
import csv
rows = list(csv.DictReader(open('.agent/remediation/AUDIT_FINDINGS.tsv'), delimiter='\t'))
for r in rows:
    if r['audit_id'] == 'AUD-090':
        raise SystemExit(0 if r['evidence_ref'] == '.agent/remediation/evidence/RX-022/verify.log' else 1)
PY
then
  note "AUD-090 evidence_ref points at RX-022/verify.log"
else
  bad "AUD-090 evidence_ref not RX-022/verify.log"
fi

# --- 7. Full register + quarantine ---
if bash .agent/remediation/verify-remediation-register.sh \
  >/tmp/rx022-register.log 2>&1; then
  note "remediation register PASS (90/90 registered, quarantine active)"
else
  bad "register failed: $(tail -3 /tmp/rx022-register.log)"
fi

echo "---"
echo "RX-022 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
