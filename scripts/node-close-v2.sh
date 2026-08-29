#!/usr/bin/env sh
# GraphLock V2 node close — writes the closure attestation and green-v2 tag.
#
# FAILS CLOSED: no attestation is written unless every DONE condition already
# holds (verified via node-status-v2.sh). NODE_DONE is an OUTPUT of this
# process, never an input. Historical green/EP-* tags are untouched.
#
# Usage: node-close-v2.sh <NODE> [ROOT]
# Env:
#   VERIFY_LOG  path to node verification log (default .agent/remediation/evidence/<NODE>/verify.log)
#   TESTS_TSV   path to test summary tsv family<TAB>passed<TAB>failed (default .../evidence/<NODE>/tests.tsv)
#   PROOFS_TSV  path to proofs tsv id<TAB>ref<TAB>digest (default .../evidence/<NODE>/proofs.tsv)
#   REQUIRED_FAMILIES comma-separated required test families (default empty)
set -eu
. "$(dirname "$0")/v2_common.sh"
NODE="${1:?node id}"
ROOT="$(v2_root "${2:-}")"
SELF_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT"

EVID="$ROOT/.agent/remediation/evidence/$NODE"
VERIFY_LOG="${VERIFY_LOG:-$EVID/verify.log}"
TESTS_TSV="${TESTS_TSV:-$EVID/tests.tsv}"
PROOFS_TSV="${PROOFS_TSV:-$EVID/proofs.tsv}"
REQUIRED_FAMILIES="${REQUIRED_FAMILIES:-}"
mkdir -p "$EVID" "$ROOT/.agent/state/closures"

fail() { echo "node-close-v2 $NODE: FAIL - $1" >&2; exit 1; }

# 1. Pre-close validation (closure manifest does not exist yet; conditions that
#    do not depend on the manifest are re-checked live and fail closed).
[ -f "$ROOT/.agent/expected-files/$NODE.txt" ] || fail "missing expected-files list"
sh "$SELF_DIR/expected-files.sh" "$NODE" >/dev/null 2>&1 \
  || fail "expected-files live check failed"
SCOPE_AUDIT_DRIFT_ONLY=1 sh "$SELF_DIR/scope-audit.sh" "$NODE" >/dev/null 2>&1 \
  || fail "scope-audit live check failed"
max_m=0
for f in "$ROOT"/.agent/milestone-files/"$NODE"-M*.txt; do
  [ -e "$f" ] || continue
  n=$(basename "$f" | sed "s/^$NODE-M\([0-9]*\)\.txt$/\1/")
  [ "$n" -gt "$max_m" ] 2>/dev/null && max_m=$n
done
[ "$max_m" -ge 1 ] || fail "no milestone manifests"
m=1
while [ "$m" -le "$max_m" ]; do
  grep -E "\\| $NODE \\| MILESTONE_PASS \\|" "$ROOT/.agent/state/LEDGER.md" 2>/dev/null | grep -q "\\[M$m\\]" \
    || fail "milestone M$m has no ledger MILESTONE_PASS"
  m=$((m+1))
done

# 2. Required verification log with exact sentinel.
[ -f "$VERIFY_LOG" ] || fail "missing verify log $VERIFY_LOG"
grep -q "verify: ok" "$VERIFY_LOG" || fail "verify log lacks exact sentinel 'verify: ok'"
grep -q "node verify $NODE: ok\|$NODE verify: ok\|RX-.*verify: ok" "$VERIFY_LOG" \
  || fail "verify log lacks node verify sentinel for $NODE"

# 3. Test summary: nonzero, zero failures, all required families present.
[ -f "$TESTS_TSV" ] || fail "missing tests.tsv"
tpass=0; tfail=0
while IFS="$(printf "\t")" read -r fam passed failed; do
  [ -n "$fam" ] || continue
  tpass=$((tpass + passed))
  tfail=$((tfail + failed))
done < "$TESTS_TSV"
[ "$tpass" -ge 1 ] || fail "required tests zero"
[ "$tfail" -eq 0 ] || fail "required tests failed ($tfail)"
if [ -n "$REQUIRED_FAMILIES" ]; then
  for fam in $(echo "$REQUIRED_FAMILIES" | tr ',' ' '); do
    grep -q "^$fam$TAB" "$TESTS_TSV" 2>/dev/null || grep -q "^$fam	" "$TESTS_TSV" \
      || fail "required test family missing: $fam"
  done
fi

# 4. Proofs manifest: at least one proof, each ref exists with matching digest.
[ -f "$PROOFS_TSV" ] || fail "missing proofs.tsv"
pcount=0
while IFS="$(printf "\t")" read -r pid ref digest; do
  [ -n "$pid" ] || continue
  pcount=$((pcount + 1))
  [ -f "$ROOT/$ref" ] || fail "proof ref missing: $ref"
  cur=$(v2_file_digest "$ROOT/$ref")
  [ "$cur" = "$digest" ] || fail "proof digest mismatch: $ref ($cur != $digest)"
done < "$PROOFS_TSV"
[ "$pcount" -ge 1 ] || fail "zero proofs"

# 5. Owned AUD findings must already be VERIFIED_FIXED in the register.
REGISTER="$ROOT/.agent/remediation/AUDIT_FINDINGS.tsv"
[ -f "$REGISTER" ] || fail "missing register"
python3 - "$REGISTER" "$NODE" <<'PY' > /tmp/v2_close_aud.$$
import csv, sys
reg, node = sys.argv[1], sys.argv[2]
bad = []
with open(reg, newline="") as fh:
    for row in csv.DictReader(fh, delimiter="\t"):
        owners = [o for o in row["repair_node"].replace(",", "/").split("/") if o]
        if node in owners and row["status"] != "VERIFIED_FIXED":
            bad.append(row["audit_id"])
print(" ".join(bad))
PY
aud_bad=$(cat /tmp/v2_close_aud.$$ 2>/dev/null || true)
rm -f /tmp/v2_close_aud.$$
[ -z "$aud_bad" ] || fail "owned findings not VERIFIED_FIXED: $aud_bad"

# 6. Compute closure facts (max_m computed during pre-close validation).
closure_commit=$(git rev-parse HEAD)
ef_digest=$(v2_list_digest "$ROOT/.agent/expected-files/$NODE.txt")
vlog_digest=$(v2_file_digest "$VERIFY_LOG")

# 7. Assemble closure JSON with canonical attestation digest.
vlog_rel=$(printf '%s' "$VERIFY_LOG" | sed "s|^$ROOT/||")
python3 - "$NODE" "$closure_commit" "$ef_digest" "$vlog_digest" "$max_m" \
  "$TESTS_TSV" "$PROOFS_TSV" "$vlog_rel" "$ROOT/.agent/state/closures/$NODE.json" <<'PY'
import json, sys, hashlib, csv
node, commit, ef_digest, vlog_digest, max_m = sys.argv[1:6]
tests_tsv, proofs_tsv, vlog_path, out = sys.argv[6:11]
def canon(obj):
    if isinstance(obj, dict):
        return "{" + ",".join(json.dumps(k, ensure_ascii=False) + ":" + canon(obj[k]) for k in sorted(obj)) + "}"
    if isinstance(obj, list):
        return "[" + ",".join(canon(x) for x in obj) + "]"
    if isinstance(obj, str):
        return json.dumps(obj, ensure_ascii=False)
    if obj is True: return "true"
    if obj is False: return "false"
    if obj is None: return "null"
    return str(obj)
families, passed, failed = [], 0, 0
for row in csv.reader(open(tests_tsv), delimiter="\t"):
    if not row or not row[0]: continue
    families.append({"family": row[0], "passed": int(row[1]), "failed": int(row[2])})
    passed += int(row[1]); failed += int(row[2])
proofs = []
for row in csv.reader(open(proofs_tsv), delimiter="\t"):
    if not row or not row[0]: continue
    proofs.append({"id": row[0], "ref": row[1], "digest": row[2]})
manifest = {
    "schema_version": 2,
    "generation": 2,
    "node": node,
    "status": "DONE",
    "closure_commit": commit,
    "green_v2_tag": f"green-v2/{node}/{commit}",
    "milestones": {"max": int(max_m), "passed": list(range(1, int(max_m)+1))},
    "expected_files_digest": ef_digest,
    "scope_list_digest": ef_digest,
    "node_verify_exit": 0,
    "verify_sentinel": "verify: ok",
    "verify_log": f"{vlog_path}|{vlog_digest}",
    "test_summary": {"passed": passed, "failed": failed, "families": families},
    "proofs": proofs,
    "created_at": __import__("datetime").datetime.now(timezone := __import__("datetime").timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "created_by": "node-close-v2.sh",
}
att = "sha256:" + hashlib.sha256(canon(manifest).encode("utf-8")).hexdigest()
manifest["attestation_digest"] = att
json.dump(manifest, open(out, "w"), indent=2, sort_keys=True)
print(out, att)
PY

# 8. Create the green-v2 tag at the closure commit (idempotent).
TAG="green-v2/$NODE/$closure_commit"
if ! git rev-parse -q --verify "refs/tags/$TAG" >/dev/null 2>&1; then
  git tag "$TAG" "$closure_commit"
fi

echo "node-close-v2 $NODE: ok (closure=$ROOT/.agent/state/closures/$NODE.json tag=$TAG)"
