#!/usr/bin/env sh
# AUD-090 regression battery: the final fresh-clone ship proof must
# perform the ship-standard fresh-clone run (SPEC-008 behavior 4).
#
# The defect (pre-fix): scripts/ep043-freshclone-accept.sh installed
# pnpm deps, ran only the EP-043 M1-M4 gates, required a knowingly
# NOT_READY readiness report, wrote only a markdown evidence file, and
# NEVER ran scripts/verify.sh, the production-readiness command, or the
# full live-fire registry. Because collectFreshCloneEvidence demands a
# structured ep043-freshclone-*.json record (AUD-075 engine), the
# markdown-only producer meant freshCloneRerun was structurally ALWAYS
# false: the claimed final fresh-clone ship proof could never clear the
# blocker.
#
# The fix (this battery proves it):
#   1. the acceptance now runs the SHIP-STANDARD LADDER inside the clone
#      (verify.sh -> verify: ok, production-readiness-check.sh ->
#      production readiness: ok, live-fire.sh full registry ->
#      live-fire: ok) before any evidence is written;
#   2. the acceptance refuses to certify (writes NO evidence) when the
#      clone graph is not ALL_DONE - a knowingly-NOT_READY tree is never
#      accepted (fail closed);
#   3. the acceptance writes a STRUCTURED ep043-freshclone-m5.json
#      ExecutionEvidence record (schema_version 1, exit 0, VERIFIED,
#      git_commit bound) that the canonical collectFreshCloneEvidence
#      adapter validates - no more markdown-only evidence;
#   4. the structured JSON producer is exercised for real in a
#      ship-state sandbox (evidence dir override), and the written
#      record round-trips through the repo's own parseExecutionEvidence /
#      validateExecutionEvidence machinery.
#
# Hostile + positive aud090 proofs, no in-test skips, no fake ALL_DONE:
# the sandbox never touches the real graph/closure/evidence state.
set -eu
export CI=true
export NO_COLOR=1
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export NODE_NO_WARNINGS=1

repo_root=$(pwd)
log="/tmp/aud090-freshclone-ship-tests.log"
: > "$log"

fail() {
  echo "AUD-090 battery: FAIL - $1" >&2
  tail -30 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "AUD-090 battery: $1"; }

CLI_INVOKE="node --experimental-transform-types --import file://$repo_root/release-evidence/scripts/ts-resolve-loader.mjs"
EVIDENCE_TS="$repo_root/release-evidence/src/evidence.ts"
SHIP_ACCEPT="$repo_root/scripts/ep043-freshclone-accept.sh"

[ -f "$SHIP_ACCEPT" ] || fail "missing $SHIP_ACCEPT"
[ -f "$EVIDENCE_TS" ] || fail "missing $EVIDENCE_TS"

# --- 1. ship-standard ladder is wired into the acceptance ---------------------
grep -q "scripts/verify.sh" "$SHIP_ACCEPT" \
  || fail "acceptance does not run scripts/verify.sh"
grep -q "verify: ok" "$SHIP_ACCEPT" \
  || fail "acceptance does not require verify: ok sentinel"
grep -q "production-readiness-check.sh" "$SHIP_ACCEPT" \
  || fail "acceptance does not run the production-readiness command"
grep -q "production readiness: ok" "$SHIP_ACCEPT" \
  || fail "acceptance does not require production readiness sentinel"
grep -q "scripts/live-fire.sh" "$SHIP_ACCEPT" \
  || fail "acceptance does not run the full live-fire registry"
grep -q "live-fire: ok" "$SHIP_ACCEPT" \
  || fail "acceptance does not require live-fire ok sentinel"
grep -q "NEXUS_REQUIRE_ALL_PROOFS=1" "$SHIP_ACCEPT" \
  || fail "acceptance live-fire run does not require all active proofs"
ok "ship-standard ladder wired (verify.sh + production-readiness + full live-fire)"

# --- 2. NOT_READY trees are never accepted (fail closed) -----------------------
grep -q 'grep -q "NOT_READY"' "$SHIP_ACCEPT" \
  && fail "acceptance still treats NOT_READY as acceptable"
grep -q "not ALL_DONE" "$SHIP_ACCEPT" \
  || fail "acceptance missing non-ALL_DONE refusal"
grep -q "is never accepted" "$SHIP_ACCEPT" \
  || fail "acceptance missing knowingly-NOT_READY fail-closed text"
ok "acceptance fails closed on non-ALL_DONE (no NOT_READY acceptance)"

# --- 3. structured JSON evidence is produced, not markdown-only -----------------
grep -q "ep043-freshclone-m5.json" "$SHIP_ACCEPT" \
  || fail "acceptance does not write structured JSON evidence"
grep -q '"schema_version"' "$SHIP_ACCEPT" \
  || fail "acceptance JSON missing schema_version"
grep -q '"proof_id"' "$SHIP_ACCEPT" \
  || fail "acceptance JSON missing proof_id"
grep -q '"result"' "$SHIP_ACCEPT" \
  || fail "acceptance JSON missing result"
grep -q '"git_commit"' "$SHIP_ACCEPT" \
  || fail "acceptance JSON missing git_commit"
grep -q '"exit_code"' "$SHIP_ACCEPT" \
  || fail "acceptance JSON missing exit_code"
ok "acceptance emits structured ExecutionEvidence JSON"

# --- 4. real hostile: acceptance refuses on the CURRENT (non-ALL_DONE) tree -----
#    The current tree is under quarantine (RX-020+ pending), so running the
#    acceptance here MUST refuse and MUST NOT write evidence anywhere.
sandbox=$(mktemp -d /tmp/aud090-hostile.XXXXXX)
trap 'rm -rf "$sandbox"' EXIT INT TERM
set +e
( cd "$repo_root" && sh scripts/ep043-freshclone-accept.sh "$sandbox/evidence" \
  >"$sandbox/run.log" 2>&1 )
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
  fail "acceptance exited 0 on a non-ALL_DONE tree (AUD-090 defect reproduced)" "$sandbox/run.log"
fi
grep -q "not ALL_DONE\|refused" "$sandbox/run.log" \
  || fail "acceptance refusal did not state the non-ALL_DONE reason" "$sandbox/run.log"
if [ -e "$sandbox/evidence/ep043-freshclone-m5.json" ]; then
  fail "acceptance wrote evidence on a non-ALL_DONE tree"
fi
if [ -e "$repo_root/.agent/state/evidence/ep043-freshclone-m5.json" ]; then
  fail "acceptance wrote evidence into the real evidence dir on a non-ALL_DONE tree"
fi
if [ -e "$repo_root/.agent/state/evidence/ep043-freshclone-m5.md" ]; then
  fail "stale markdown fresh-clone evidence still present in the real evidence dir"
fi
ok "hostile: acceptance refused on non-ALL_DONE tree and wrote no evidence"

# --- 5. real positive: the JSON evidence producer round-trips -------------------
#    Extract the evidence-writing python (the tail of the acceptance) and run it
#    against a sandbox evidence dir with real digests, then parse + validate the
#    written record through the repo's own structured evidence machinery. The
#    sandbox never touches the real graph/closure/evidence state.
evidence_writer="$sandbox/write_evidence.py"
python3 - "$SHIP_ACCEPT" "$evidence_writer" <<'PYEOF'
import re, sys
src = open(sys.argv[1], encoding="utf-8").read()
m = re.search(r"python3 - .*?<<'PYEOF'\n(.*?)\nPYEOF", src, re.S)
if not m:
    raise SystemExit("cannot locate evidence writer heredoc")
open(sys.argv[2], "w", encoding="utf-8").write(m.group(1))
PYEOF
[ -s "$evidence_writer" ] || fail "could not extract evidence writer"

writer_args="$sandbox/args.env"
cat > "$writer_args" <<EOF
evidence='$sandbox/evidence/ep043-freshclone-m5.json'
run_id='ep043-freshclone-aud090-positive'
commit='1111111111111111111111111111111111111111'
started='2026-09-02T00:00:00.000Z'
completed='2026-09-02T00:00:01.000Z'
stdout_digest='sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
stderr_digest='sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
summary='FRESH-CLONE SHIP ACCEPTANCE EVIDENCE (AUD-090)
Run: ep043-freshclone-aud090-positive
Git commit: 1111111111111111111111111111111111111111
Tree at checkout: clean
Graph gate: ALL_DONE confirmed in clone
EP-043 gates in clone: ep043-m4-tests.sh ok
Ship ladder in clone: verify.sh ok, production-readiness ok, full live-fire registry ok
Source-tree leakage: none'
EOF
# shellcheck disable=SC1090
. "$writer_args"
mkdir -p "$(dirname "$evidence")"
python3 "$evidence_writer" "$evidence" "$run_id" "$commit" "$started" \
  "$completed" "$stdout_digest" "$stderr_digest" "$summary" \
  >"$sandbox/writer2.log" 2>&1 \
  || { cat "$sandbox/writer2.log" >&2; exit 1; }
[ -s "$evidence" ] || fail "evidence producer wrote nothing"

# round-trip through the canonical adapter machinery
node --experimental-transform-types \
  --import "file://$repo_root/release-evidence/scripts/ts-resolve-loader.mjs" \
  -e "
import { readFileSync } from 'node:fs';
import { parseExecutionEvidence, validateExecutionEvidence } from 'file://$repo_root/release-evidence/src/evidence.ts';
const raw = readFileSync('$evidence', 'utf8');
const record = parseExecutionEvidence(raw);
const valid = validateExecutionEvidence(record, {
  expectedCommit: '$commit',
  requiredResult: ['VERIFIED', 'PASS'],
});
if (record.proof_id !== 'ep043-freshclone') throw new Error('proof_id mismatch');
if (record.result !== 'VERIFIED') throw new Error('result not VERIFIED');
if (record.exit_code !== 0) throw new Error('exit_code not 0');
if (!valid) throw new Error('validateExecutionEvidence rejected the produced record');
console.log('roundtrip: ok');
" >"$sandbox/roundtrip.log" 2>&1 \
  || { cat "$sandbox/roundtrip.log" >&2; exit 1; }
grep -q "roundtrip: ok" "$sandbox/roundtrip.log" \
  || fail "produced evidence did not round-trip through canonical adapter" "$sandbox/roundtrip.log"
ok "positive: structured JSON evidence producer round-trips through canonical adapter"

echo "AUD-090 battery: ok (GATE_EXIT=0)"
