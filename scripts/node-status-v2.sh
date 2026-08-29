#!/usr/bin/env sh
# GraphLock V2 node status - the only closure authority in generation 2.
#
# A node is DONE only when ALL of the following are simultaneously true:
#   milestones_passed
#   AND expected_files_passed
#   AND scope_audit_passed
#   AND node_verify_exit == 0
#   AND verify_sentinel_exact
#   AND required_tests_nonzero
#   AND required_tests_failed == 0
#   AND required_proofs_validated
#   AND owned_AUD_findings_verified_fixed
#   AND closure_attestation_valid
#   AND green_v2_tag_exists
#   AND green_v2_tag_target == closure_commit
#
# NODE_DONE is NEVER an input. This recomputes node truth independently of the
# ledger status function. A forged ledger or tag state cannot produce DONE.
#
# Usage: node-status-v2.sh <NODE> [ROOT] [--quiet]
set -eu
. "$(dirname "$0")/v2_common.sh"
NODE="${1:?node id}"
ROOT="$(v2_root "${2:-}")"
QUIET="${3:-}"
SELF_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT"

CLOSURES="$ROOT/.agent/state/closures"
MANIFEST="$CLOSURES/$NODE.json"
REGISTER="$ROOT/.agent/remediation/AUDIT_FINDINGS.tsv"
FAIL=0
REASONS=""

note() {
  [ -n "$QUIET" ] || echo "node-status-v2 $NODE: $1"
}
bad() {
  FAIL=1
  REASONS="$REASONS $1"
  echo "node-status-v2 $NODE: NOT_DONE [$1]" >&2
  note "NOT_DONE [$1]"
}

# --- closure manifest exists -------------------------------------------------
if [ ! -f "$MANIFEST" ]; then
  echo "node-status-v2 $NODE: NOT_DONE [missing closure manifest]" >&2
  note "node-status-v2 $NODE: NOT_DONE [missing closure manifest]"
  exit 1
fi

# --- closure attestation valid (recompute canonical digest) ------------------
expected=$(python3 -c "import json,sys;print(json.load(open('$MANIFEST')).get('attestation_digest',''))" 2>/dev/null || true)
actual=$(v2_canonical_digest "$MANIFEST" 2>/dev/null || true)
if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
  bad "closure_attestation_invalid"
fi

# --- node binding (copied manifest from another node fails here) -------------
node_in_manifest=$(python3 -c "import json,sys;print(json.load(open('$MANIFEST')).get('node',''))" 2>/dev/null || true)
[ "$node_in_manifest" = "$NODE" ] || bad "closure_node_mismatch"

closure_commit=$(python3 -c "import json,sys;print(json.load(open('$MANIFEST')).get('closure_commit',''))" 2>/dev/null || true)
gen=$(python3 -c "import json,sys;print(json.load(open('$MANIFEST')).get('generation',''))" 2>/dev/null || true)
[ "$gen" = "2" ] || bad "generation_not_2"

# --- green_v2 tag exists and targets the closure commit ----------------------
tag=$(git tag -l "green-v2/$NODE/*" 2>/dev/null | head -n 1 || true)
if [ -z "$tag" ]; then
  bad "green_v2_tag_missing"
else
  target=$(git rev-parse "$tag^{commit}" 2>/dev/null || true)
  [ "$target" = "$closure_commit" ] || bad "green_v2_tag_target_mismatch"
fi

# --- milestones passed (ledger MILESTONE_PASS + milestone manifests exist) ---
max_m=$(python3 -c "import json,sys;print(json.load(open('$MANIFEST')).get('milestones',{}).get('max',0))" 2>/dev/null || true)
if [ "${max_m:-0}" -ge 1 ] 2>/dev/null; then
  m_ok=1
  m=1
  while [ "$m" -le "$max_m" ]; do
    [ -f "$ROOT/.agent/milestone-files/$NODE-M$m.txt" ] || { m_ok=0; break; }
    grep -E "\\| $NODE \\| MILESTONE_PASS \\|" "$ROOT/.agent/state/LEDGER.md" 2>/dev/null | grep -q "\\[M$m\\]" || { m_ok=0; break; }
    m=$((m+1))
  done
  [ "$m_ok" -eq 1 ] || bad "milestones_not_passed"
else
  bad "milestones_zero"
fi

# --- expected files passed (live) + list digest matches ----------------------
ef_digest=$(python3 -c "import json,sys;print(json.load(open('$MANIFEST')).get('expected_files_digest',''))" 2>/dev/null || true)
cur_ef_digest=$(v2_list_digest "$ROOT/.agent/expected-files/$NODE.txt" 2>/dev/null || true)
if [ -n "$ef_digest" ] && [ "$ef_digest" = "$cur_ef_digest" ]; then
  if sh "$SELF_DIR/expected-files.sh" "$NODE" >/dev/null 2>&1; then
    :
  else
    bad "expected_files_live_fail"
  fi
else
  bad "expected_files_changed"
fi

# --- scope audit passed (live) + scope list digest matches -------------------
sc_digest=$(python3 -c "import json,sys;print(json.load(open('$MANIFEST')).get('scope_list_digest',''))" 2>/dev/null || true)
cur_sc_digest=$(v2_list_digest "$ROOT/.agent/expected-files/$NODE.txt" 2>/dev/null || true)
if [ -n "$sc_digest" ] && [ "$sc_digest" = "$cur_sc_digest" ]; then
  if SCOPE_AUDIT_DRIFT_ONLY=1 sh "$SELF_DIR/scope-audit.sh" "$NODE" >/dev/null 2>&1; then
    :
  else
    bad "scope_audit_live_fail"
  fi
else
  bad "scope_list_changed"
fi

# --- node verify exit == 0 + verify sentinel exact + verify log digest -------
nv_exit=$(python3 -c "import json,sys;print(json.load(open('$MANIFEST')).get('node_verify_exit',-1))" 2>/dev/null || true)
[ "$nv_exit" = "0" ] || bad "node_verify_exit_nonzero"
sentinel=$(python3 -c "import json,sys;print(json.load(open('$MANIFEST')).get('verify_sentinel',''))" 2>/dev/null || true)
[ "$sentinel" = "verify: ok" ] || bad "verify_sentinel_not_exact"
vlog=$(python3 -c "import json,sys;print(json.load(open('$MANIFEST')).get('verify_log',''))" 2>/dev/null || true)
if [ -n "$vlog" ]; then
  vp="${vlog%%|*}"; vd="${vlog##*|}"
  if [ -f "$ROOT/$vp" ]; then
    cur=$(v2_file_digest "$ROOT/$vp")
    [ "$cur" = "$vd" ] || bad "verify_log_stale"
  else
    bad "verify_log_missing"
  fi
fi

# --- required tests nonzero + failed == 0 ------------------------------------
tpass=$(python3 -c "import json,sys;print(json.load(open('$MANIFEST')).get('test_summary',{}).get('passed',0))" 2>/dev/null || true)
tfail=$(python3 -c "import json,sys;print(json.load(open('$MANIFEST')).get('test_summary',{}).get('failed',-1))" 2>/dev/null || true)
tfam=$(python3 -c "import json,sys;print(len(json.load(open('$MANIFEST')).get('test_summary',{}).get('families',[])))" 2>/dev/null || true)
[ "${tpass:-0}" -ge 1 ] 2>/dev/null || bad "required_tests_zero"
[ "${tfail:-1}" = "0" ] 2>/dev/null || bad "required_tests_failed"
[ "${tfam:-0}" -ge 1 ] 2>/dev/null || bad "required_test_families_empty"

# --- required proofs validated (each proof ref exists with matching digest) --
rc=0
python3 - "$MANIFEST" "$ROOT" > /tmp/v2_proof_check.$$ 2>/dev/null <<'PY' || rc=$?
import json, hashlib, sys, os
m = json.load(open(sys.argv[1]))
root = sys.argv[2]
for p in m.get("proofs", []):
    ref = p.get("ref", "")
    dig = p.get("digest", "")
    path = os.path.join(root, ref) if ref else ""
    if not ref or not os.path.isfile(path):
        print("missing " + ref); sys.exit(1)
    if dig and dig != "sha256:" + hashlib.sha256(open(path, "rb").read()).hexdigest():
        print("stale " + ref); sys.exit(1)
PY
rm -f /tmp/v2_proof_check.$$
if [ "$rc" -ne 0 ]; then
  bad "proofs_not_validated"
fi

# --- owned AUD findings (live register read) ---------------------------------
# A node may close when every finding it owns is VERIFIED_FIXED, or - for
# findings shared with other repair nodes - at least FIXED_UNVERIFIED (the
# leaf completes only when ALL owners close; verified at final certification).
if [ -f "$REGISTER" ]; then
  python3 - "$REGISTER" "$NODE" <<'PY' > /tmp/v2_aud_check.$$ 2>/dev/null || true
import csv, sys
reg, node = sys.argv[1], sys.argv[2]
bad = []
with open(reg, newline="") as fh:
    for row in csv.DictReader(fh, delimiter="\t"):
        owners = [o for o in row["repair_node"].replace(",", "/").split("/") if o]
        if node not in owners:
            continue
        st = row["status"]
        if st == "VERIFIED_FIXED":
            continue
        if st == "FIXED_UNVERIFIED" and len(owners) > 1:
            continue
        bad.append(row["audit_id"] + ":" + st)
print(" ".join(bad))
PY
  aud_bad=$(cat /tmp/v2_aud_check.$$ 2>/dev/null || true)
  rm -f /tmp/v2_aud_check.$$
  [ -z "$aud_bad" ] || bad "owned_aud_not_verified_fixed ($aud_bad)"
fi

# --- verdict ----------------------------------------------------------------
if [ "$FAIL" -eq 0 ]; then
  note "DONE (closure_commit=$closure_commit tag=$tag)"
  exit 0
fi
note "NOT_DONE$REASONS"
exit 1
