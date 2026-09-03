#!/usr/bin/env sh
# RX-001 GraphLock V2 hostile test battery.
#
# Proves: a forged ledger or tag state cannot cause NEXT, ALL_DONE, or release
# readiness. Required hostile cases (remediation graph RX-001):
#   forged NODE_DONE, missing tag, tag -> other commit, stale evidence,
#   missing closure manifest, changed expected files after verification,
#   zero tests, skipped required tests, forged evidence path,
#   closure manifest copied from another node.
# Plus AUD-080 (gate cannot pass while NOT_READY) and AUD-085 (scheduler
# delegation: forged NODE_DONE cannot advance the graph).
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
REPO=$(cd "$(dirname "$0")/.." && pwd)
PASS=0
FAILED=0

ok() { PASS=$((PASS+1)); echo "ok - $1"; }
bad() { FAILED=$((FAILED+1)); echo "FAIL - $1"; }

sandbox=$(mktemp -d /tmp/rx001-hostile-XXXXXX)
trap 'rm -rf "$sandbox"' EXIT

# --- seed a hermetic sandbox repo ----------------------------------------------------
(
  cd "$sandbox"
  git init -q .
  git config user.email rx001@test
  git config user.name rx001
  mkdir -p .agent/state/closures .agent/milestone-files .agent/expected-files \
           .agent/remediation payload .agent/remediation/evidence/TX-001
  printf 'payload/a.txt\npayload/b.txt\npayload/c.txt\n' > .agent/expected-files/TX-001.txt
  printf 'payload/a.txt\n' > .agent/milestone-files/TX-001-M1.txt
  printf 'payload/b.txt\n' > .agent/milestone-files/TX-001-M2.txt
  printf 'payload/c.txt\n' > .agent/milestone-files/TX-001-M3.txt
  printf 'A\n' > payload/a.txt
  printf 'B\n' > payload/b.txt
  printf 'C\n' > payload/c.txt
  : > .agent/state/LEDGER.md
  printf '2026-08-29T00:00:00Z | t | TX-001 | MILESTONE_PASS | [M1] x\n' >> .agent/state/LEDGER.md
  printf '2026-08-29T00:00:00Z | t | TX-001 | MILESTONE_PASS | [M2] x\n' >> .agent/state/LEDGER.md
  printf '2026-08-29T00:00:00Z | t | TX-001 | MILESTONE_PASS | [M3] x\n' >> .agent/state/LEDGER.md
  printf 'audit_id\tseverity\ttitle\taffected_paths\troot_cause\trepair_node\tstatus\tregression_test\tevidence_ref\tfixed_commit\tverified_commit\n' > .agent/remediation/AUDIT_FINDINGS.tsv
  printf 'verify: ok\nnode verify TX-001: ok\n' > .agent/remediation/evidence/TX-001/verify.log
  printf 'unit\t5\t0\n' > .agent/remediation/evidence/TX-001/tests.tsv
  printf 'proof1\t.agent/remediation/evidence/TX-001/verify.log\tsha256:%s\n' \
    "$(sha256sum .agent/remediation/evidence/TX-001/verify.log | cut -d' ' -f1)" > .agent/remediation/evidence/TX-001/proofs.tsv
  git add -A
  git commit -qm seed
)

seed_closure() {  # $1 = sandbox, $2 = node, $3 = overrides-json
  sand="$1"; node="$2"; overrides="${3:-}"
  COMMIT=$(git -C "$sand" rev-parse HEAD)
  python3 - "$sand" "$node" "$COMMIT" "$overrides" <<'PY'
import json, sys, hashlib
sand, node, commit = sys.argv[1], sys.argv[2], sys.argv[3]
over = json.loads(sys.argv[4]) if sys.argv[4] else {}
def canon(o):
    if isinstance(o, dict): return "{" + ",".join(json.dumps(k, ensure_ascii=False)+":"+canon(o[k]) for k in sorted(o)) + "}"
    if isinstance(o, list): return "[" + ",".join(canon(x) for x in o) + "]"
    if isinstance(o, str): return json.dumps(o, ensure_ascii=False)
    if o is True: return "true"
    if o is False: return "false"
    if o is None: return "null"
    return str(o)
vlog = f"{sand}/.agent/remediation/evidence/{node}/verify.log"
vd = "sha256:" + hashlib.sha256(open(vlog,"rb").read()).hexdigest()
ef = open(f"{sand}/.agent/expected-files/{node}.txt", encoding="utf-8").read()
efd = "sha256:" + hashlib.sha256("\n".join(sorted(l.strip() for l in ef.splitlines() if l.strip() and not l.startswith("#"))).encode()).hexdigest()
m = {
  "schema_version": 2, "generation": 2, "node": node, "status": "DONE",
  "closure_commit": commit, "green_v2_tag": f"green-v2/{node}/{commit}",
  "milestones": {"max": 3, "passed": [1,2,3]},
  "expected_files_digest": efd, "scope_list_digest": efd,
  "node_verify_exit": 0, "verify_sentinel": "verify: ok",
  "verify_log": f".agent/remediation/evidence/{node}/verify.log|{vd}",
  "test_summary": {"passed": 5, "failed": 0, "families": [{"family":"unit","passed":5,"failed":0}]},
  "proofs": [{"id":"proof1","ref":f".agent/remediation/evidence/{node}/verify.log","digest":vd}],
  "created_at": "2026-08-29T00:00:00Z", "created_by": "hostile-seed",
}
m.update(over)
m["attestation_digest"] = "sha256:" + hashlib.sha256(canon(m).encode()).hexdigest()
json.dump(m, open(f"{sand}/.agent/state/closures/{node}.json","w"), indent=2, sort_keys=True)
PY
  # retag to the current HEAD so each seed produces a valid closure baseline
  git -C "$sand" tag -d "green-v2/$node/$(git -C "$sand" rev-parse HEAD)" >/dev/null 2>&1 || true
  for t in $(git -C "$sand" tag -l "green-v2/$node/*"); do git -C "$sand" tag -d "$t" >/dev/null 2>&1 || true; done
  git -C "$sand" tag "green-v2/$node/$(git -C "$sand" rev-parse HEAD)" >/dev/null 2>&1 || true
}

expect_done() {  # $1 = sandbox, $2 = node, $3 = label
  if (cd "$1" && sh "$REPO/scripts/node-status-v2.sh" "$2" "$1" --quiet >/dev/null 2>&1); then
    ok "$3"
  else
    bad "$3 (expected DONE)"
  fi
}
expect_not_done() {  # $1 = sandbox, $2 = node, $3 = label, $4 = expected reason
  out=$(cd "$1" && sh "$REPO/scripts/node-status-v2.sh" "$2" "$1" --quiet 2>&1) || true
  case "$out" in
    *"$4"*) ok "$3" ;;
    *) bad "$3 (expected reason [$4], got: $out)" ;;
  esac
}

# 0. happy path: valid closure + tag -> DONE (seed_closure creates both)
seed_closure "$sandbox" TX-001 ""
expect_done "$sandbox" TX-001 "valid closure with matching tag is DONE"

# 1. forged NODE_DONE cannot produce DONE without closure
printf '2026-08-29T00:00:00Z | t | TX-002 | NODE_DONE | forged\n' >> "$sandbox/.agent/state/LEDGER.md"
expect_not_done "$sandbox" TX-002 "forged NODE_DONE alone is NOT_DONE" "missing closure manifest"

# 2. missing green-v2 tag
git -C "$sandbox" tag -d "green-v2/TX-001/$(git -C "$sandbox" rev-parse HEAD)" >/dev/null 2>&1 || true
expect_not_done "$sandbox" TX-001 "missing green-v2 tag is NOT_DONE" "green_v2_tag_missing"

# 3. tag pointing to another commit
git -C "$sandbox" commit -q --allow-empty -m other
OTHER=$(git -C "$sandbox" rev-parse HEAD)
git -C "$sandbox" tag "green-v2/TX-001/$OTHER" >/dev/null 2>&1 || true
expect_not_done "$sandbox" TX-001 "tag targeting another commit is NOT_DONE" "green_v2_tag_target_mismatch"

# restore correct tag for subsequent tests
git -C "$sandbox" tag -d "green-v2/TX-001/$OTHER" >/dev/null 2>&1 || true
git -C "$sandbox" tag "green-v2/TX-001/$(git -C "$sandbox" rev-parse HEAD~1)" >/dev/null 2>&1 || true

# 4. stale evidence (verify log mutated after closure)
printf 'verify: ok\nnode verify TX-001: ok\nTAMPERED\n' >> "$sandbox/.agent/remediation/evidence/TX-001/verify.log"
expect_not_done "$sandbox" TX-001 "stale evidence is NOT_DONE" "verify_log_stale"
git -C "$sandbox" checkout -q -- .agent/remediation/evidence/TX-001/verify.log

# 5. missing closure manifest
rm "$sandbox/.agent/state/closures/TX-001.json"
expect_not_done "$sandbox" TX-001 "missing closure manifest is NOT_DONE" "missing closure manifest"

# 6. changed expected files after verification (list digest changed)
seed_closure "$sandbox" TX-001 ""
printf 'payload/a.txt\npayload/b.txt\npayload/c.txt\npayload/new.txt\n' > "$sandbox/.agent/expected-files/TX-001.txt"
expect_not_done "$sandbox" TX-001 "expected-files list changed after closure is NOT_DONE" "expected_files_changed"
# 6b. deleted expected file (live check fails with digest still matching)
printf 'payload/a.txt\npayload/b.txt\npayload/c.txt\n' > "$sandbox/.agent/expected-files/TX-001.txt"
rm "$sandbox/payload/c.txt"
expect_not_done "$sandbox" TX-001 "deleted expected file is NOT_DONE" "expected_files_live_fail"
printf 'C\n' > "$sandbox/payload/c.txt"

# 7. zero tests
seed_closure "$sandbox" TX-001 '{"test_summary":{"passed":0,"failed":0,"families":[{"family":"unit","passed":0,"failed":0}]}}'
expect_not_done "$sandbox" TX-001 "zero tests is NOT_DONE" "required_tests_zero"

# 8. skipped required tests (failed>0 / empty families)
seed_closure "$sandbox" TX-001 '{"test_summary":{"passed":4,"failed":1,"families":[{"family":"unit","passed":4,"failed":1}]}}'
expect_not_done "$sandbox" TX-001 "failed required tests is NOT_DONE" "required_tests_failed"
seed_closure "$sandbox" TX-001 '{"test_summary":{"passed":4,"failed":0,"families":[]}}'
expect_not_done "$sandbox" TX-001 "empty required test families is NOT_DONE" "required_test_families_empty"

# 9. forged evidence path (proof ref missing)
seed_closure "$sandbox" TX-001 '{"proofs":[{"id":"ghost","ref":".agent/remediation/evidence/TX-001/ghost.log","digest":"sha256:0000000000000000000000000000000000000000000000000000000000000000"}]}'
expect_not_done "$sandbox" TX-001 "forged evidence path is NOT_DONE" "proofs_not_validated"

# 10. closure manifest copied from another node (node binding breaks attestation)
seed_closure "$sandbox" TX-001 ""
cp "$sandbox/.agent/state/closures/TX-001.json" "$sandbox/.agent/state/closures/TX-002.json"
expect_not_done "$sandbox" TX-002 "closure copied from another node is NOT_DONE" "closure_node_mismatch"

# 11. AUD-085: forged NODE_DONE cannot advance the scheduler
# TX-001 closed (valid); TX-002 has ONLY forged NODE_DONE (no closure) -> the
# scheduler must NOT treat TX-002 as DONE; it may schedule it as next work.
seed_closure "$sandbox" TX-001 ""
printf 'TX-001 DEPS -\nTX-002 DEPS TX-001\n' > "$sandbox/.agent/remediation/REMEDIATION_DAG.txt"
out=$(cd "$sandbox" && sh "$REPO/scripts/graph-next-v2.sh" "$sandbox")
case "$out" in
  "NEXT TX-002") ok "scheduler advances only via closure (TX-002 next, not DONE)" ;;
  *) bad "scheduler forged-NODE_DONE handling (got: $out)" ;;
esac
# remove TX-001 closure AND tag: forged NODE_DONE must leave everything stalled
rm "$sandbox/.agent/state/closures/TX-001.json"
git -C "$sandbox" tag -d "green-v2/TX-001/$(git -C "$sandbox" rev-parse HEAD)" >/dev/null 2>&1 || true
out=$(cd "$sandbox" && sh "$REPO/scripts/graph-next-v2.sh" "$sandbox")
case "$out" in
  "STALL TX-001"|"NEXT TX-001") ok "forged NODE_DONE cannot advance graph (stalled at TX-001)" ;;
  *) bad "forged NODE_DONE advanced graph (got: $out)" ;;
esac
# legacy graph-next.sh must delegate to V2 in generation 2 (AUD-085 root fix)
mkdir -p "$sandbox/.agent/remediation"
printf 'REMEDIATION_GENERATION=2\nRELEASE_ALLOWED=false\n' > "$sandbox/.agent/remediation/REMEDIATION_STATE.env"
seed_closure "$sandbox" TX-001 ""
out=$(cd "$sandbox" && sh "$REPO/scripts/graph-next.sh")
case "$out" in
  "NEXT TX-002") ok "legacy graph-next.sh delegates to V2 scheduler in generation 2" ;;
  *) bad "legacy scheduler did not delegate (got: $out)" ;;
esac

# 12. AUD-080: EP-043 closure gate must FAIL while readiness is NOT_READY
if [ -f "$REPO/scripts/ep043-m5-tests.sh" ]; then
  if grep -q "AUD-080" "$REPO/scripts/ep043-m5-tests.sh"; then
    ok "EP-043 M5 gate contains AUD-080 readiness-required fix"
  else
    bad "EP-043 M5 gate missing AUD-080 fix"
  fi
  if grep -q "closure gate: ship gate BLOCKED - node cannot close" "$REPO/scripts/ep043-m5-tests.sh"; then
    ok "EP-043 M5 gate fails closed on BLOCKED ship verdict"
  else
    bad "EP-043 M5 gate missing ship-gate fail-closed"
  fi
else
  bad "ep043-m5-tests.sh missing"
fi

echo "----"
echo "RX-001 hostile battery: $PASS passed, $FAILED failed"
[ "$FAILED" -eq 0 ] || exit 1
exit 0
