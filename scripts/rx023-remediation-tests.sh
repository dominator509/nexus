#!/usr/bin/env sh
# RX-023 regression battery: final graph closure - the register is 90/90
# VERIFIED_FIXED, every graph node RX-000..RX-022 is V2-DONE, and the tree
# is ready for the ship gate (production-readiness-check.sh path).
# RX-023 owns no findings; its contract is the register-wide closure state
# and graph ALL_DONE reachability.
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

# --- 1. Register: 90/90 findings, every row VERIFIED_FIXED, valid statuses ---
python3 - <<'PY'
import csv, sys
rows = list(csv.DictReader(open('.agent/remediation/AUDIT_FINDINGS.tsv'), delimiter='\t'))
if len(rows) != 90:
    print('FAIL register length', len(rows)); sys.exit(1)
for r in rows:
    if r['status'] != 'VERIFIED_FIXED':
        print('FAIL row open:', r['audit_id'], r['status']); sys.exit(1)
    if not r['regression_test'].strip() or not r['evidence_ref'].strip():
        print('FAIL row missing evidence:', r['audit_id']); sys.exit(1)
print('register 90/90 all VERIFIED_FIXED with evidence')
PY
note "register 90/90 all VERIFIED_FIXED with evidence columns"

# --- 2. Quarantine state: generation 2, release still blocked ---
if grep -q "^REMEDIATION_GENERATION=2" .agent/remediation/REMEDIATION_STATE.env \
  && grep -q "^RELEASE_ALLOWED=false" .agent/remediation/REMEDIATION_STATE.env; then
  note "quarantine active (generation 2, RELEASE_ALLOWED=false)"
else
  bad "quarantine state wrong"
fi

# --- 3. Every graph node RX-000..RX-022 V2-DONE (node-status recompute) ---
failed_nodes=""
d=0
while [ "$d" -le 22 ]; do
  n=$(printf "RX-%03d" "$d")
  if ! sh scripts/node-status-v2.sh "$n" --quiet >/dev/null 2>&1; then
    failed_nodes="$failed_nodes $n"
  fi
  d=$((d + 1))
done
if [ -z "$failed_nodes" ]; then
  note "RX-000..RX-022 all V2-DONE (12 conditions each, recomputed)"
else
  bad "nodes not DONE:$failed_nodes"
fi

# --- 4. Graph scheduler: RX-023 is the dispatch target (all deps DONE) ---
# Before RX-023 closes, graph-next prints RESUME RX-023 (the node itself is
# not yet DONE); after closure it prints ALL_DONE. Either way the scheduler
# must NOT name an earlier node: that proves every dependency is V2-DONE.
next_out=$(sh scripts/graph-next.sh 2>/dev/null)
case "$next_out" in
  *"RX-023"*|ALL_DONE|ALL_DONE_V2)
    note "graph scheduler reached RX-023 (deps all DONE): $next_out"
    ;;
  *)
    bad "graph scheduler did not reach RX-023: $next_out"
    ;;
esac

# --- 5. Expected closure evidence for every closed node ---
missing=0
d=0
while [ "$d" -le 22 ]; do
  n=$(printf "RX-%03d" "$d")
  if [ ! -f ".agent/remediation/evidence/$n/verify.log" ] \
    || [ ! -f ".agent/state/closures/$n.json" ]; then
    missing=$((missing + 1))
  fi
  d=$((d + 1))
done
if [ "$missing" -eq 0 ]; then
  note "RX-000..RX-022 closure manifests + verify.logs all present"
else
  bad "$missing nodes missing closure artifacts"
fi

# --- 6. Scheduler gate: zero P0/P1 findings not VERIFIED_FIXED ---
if python3 - <<'PY'
import csv, sys
rows = list(csv.DictReader(open('.agent/remediation/AUDIT_FINDINGS.tsv'), delimiter='\t'))
bad = [r['audit_id'] for r in rows if r['severity'] in ('P0','P1') and r['status'] != 'VERIFIED_FIXED']
sys.exit(1 if bad else 0)
PY
then
  note "zero P0/P1 findings not VERIFIED_FIXED (scheduler gate)"
else
  bad "P0/P1 findings still open"
fi

echo "---"
echo "RX-023 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
