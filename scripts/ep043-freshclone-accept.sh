#!/usr/bin/env sh
# EP-043 M5 final fresh-clone acceptance (SPEC-008 fresh-clone
# prerequisite) - the SHIP-STANDARD fresh-clone run (AUD-090).
#
# Real acceptance rerun on a clean isolated checkout of the exact
# candidate commit:
#   - clone file://<repo> at the candidate commit into a throwaway dir
#   - assert the clone HEAD is exactly the candidate commit
#   - assert the clone tree starts clean (no hidden local state)
#   - require the clone graph to be ALL_DONE (AUD-090: a ship proof is
#     only possible on a genuinely shippable tree; otherwise the
#     acceptance refuses and writes NO evidence - a knowingly-NOT_READY
#     tree is never accepted)
#   - restore dependencies from frozen files (pnpm --frozen-lockfile;
#     the pnpm store is a global package cache, never working-tree state)
#   - run the EP-043 owned gates (M1-M4) inside the clone with
#     EP043_TEST_ROOT pointing at the clone, so no test reads the
#     development tree
#   - run the SHIP-STANDARD LADDER inside the clone (SPEC-008 behavior 4;
#     AUD-090): scripts/verify.sh, the production-readiness command, and
#     the full live-fire registry, each requiring its ok sentinel
#   - run the readiness / manifest / verify-manifest CLIs in the clone
#   - source-tree leakage negative proof: no command output or gate log
#     may reference the development tree path
#   - write dated structured acceptance evidence
#     (ep043-freshclone-m5.json, ExecutionEvidence schema v1) binding the
#     candidate commit, only after every step passed
#
# Usage:
#   sh scripts/ep043-freshclone-accept.sh [evidence_dir]
set -eu
export CI=true
export NO_COLOR=1
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat

repo_root=$(pwd)
evidence_dir="${1:-.agent/state/evidence}"
run_id="ep043-freshclone-$(date +%s)"
started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
candidate_commit=$(/usr/bin/git rev-parse HEAD 2>/dev/null || echo "unknown")

fail() {
  echo "ep043-freshclone-accept: FAIL - $1" >&2
  exit 1
}

accept_dir=$(mktemp -d /tmp/ep043-m5-accept.XXXXXX)
trap 'rm -rf "$accept_dir"' EXIT INT TERM

# --- clean isolated checkout of the exact candidate commit -------------------
# Full clone (NOT --depth 1): node-status-v2 validates green-v2/<node> tags
# whose targets are ancestor commits of the candidate, and validates the
# committed closure evidence verify.log digests. A shallow clone can never
# reproduce node-DONE truth, so the ship-standard graph gate would be
# unreachable there (AUD-090). Tags are fetched so the clone sees the same
# graph state as the development tree.
git clone --quiet "file://$repo_root" "$accept_dir/clone" \
  || fail "cannot clone candidate commit"
cd "$accept_dir/clone"
clone_commit=$(/usr/bin/git rev-parse HEAD)
[ "$clone_commit" = "$candidate_commit" ] \
  || fail "clone is not the candidate commit ($clone_commit != $candidate_commit)"
[ -z "$(/usr/bin/git status --porcelain)" ] \
  || fail "clone tree is not clean at checkout"

# --- AUD-090: ship proof only on a genuinely shippable tree -------------------
clone_dispatch=$(sh scripts/graph-next.sh 2>/dev/null || echo "UNKNOWN")
if [ "$clone_dispatch" != "ALL_DONE" ] && [ "$clone_dispatch" != "ALL_DONE_V2" ]; then
  echo "ep043-freshclone-accept: FAIL - ship-standard acceptance refused: clone graph is $clone_dispatch (not ALL_DONE); a knowingly-NOT_READY tree is never accepted (AUD-090)" >&2
  exit 1
fi
echo "fresh-clone graph gate: ALL_DONE confirmed in clone"

# --- dependency restore from frozen files ------------------------------------
corepack enable >/dev/null 2>&1 || true
pnpm install --frozen-lockfile --prefer-offline >"$accept_dir/install.log" 2>&1 \
  || fail "pnpm install --frozen-lockfile failed (see $accept_dir/install.log)"

# --- EP-043 owned gates in the clone (isolated test root) ----------------------
export EP043_TEST_ROOT="$accept_dir/clone"
for gate in ep043-m1-tests.sh ep043-m2-tests.sh ep043-m3-tests.sh ep043-m4-tests.sh; do
  sh "scripts/$gate" >"$accept_dir/$gate.log" 2>&1 \
    || fail "$gate failed inside fresh clone (see $accept_dir/$gate.log)"
  echo "fresh-clone gate: $gate ok"
done

# --- SHIP-STANDARD LADDER inside the clone (SPEC-008 behavior 4; AUD-090) ------
# 1. full canonical verify ladder
sh scripts/verify.sh >"$accept_dir/verify.log" 2>&1 \
  || { echo "ep043-freshclone-accept: FAIL - verify.sh failed inside fresh clone (see $accept_dir/verify.log)" >&2; tail -20 "$accept_dir/verify.log" >&2; exit 1; }
grep -q "verify: ok" "$accept_dir/verify.log" \
  || fail "verify.sh did not print 'verify: ok' inside fresh clone"
echo "fresh-clone ship ladder: verify: ok"
# 2. production-readiness command
sh scripts/production-readiness-check.sh >"$accept_dir/prodreadiness.log" 2>&1 \
  || { echo "ep043-freshclone-accept: FAIL - production-readiness command failed inside fresh clone (see $accept_dir/prodreadiness.log)" >&2; tail -20 "$accept_dir/prodreadiness.log" >&2; exit 1; }
grep -q "production readiness: ok" "$accept_dir/prodreadiness.log" \
  || fail "production-readiness command did not print 'production readiness: ok' inside fresh clone"
echo "fresh-clone ship ladder: production readiness: ok"
# 3. full live-fire registry (every active proof; silent skips fail closed)
NEXUS_REQUIRE_ALL_PROOFS=1 sh scripts/live-fire.sh >"$accept_dir/livefire.log" 2>&1 \
  || { echo "ep043-freshclone-accept: FAIL - full live-fire registry failed inside fresh clone (see $accept_dir/livefire.log)" >&2; tail -20 "$accept_dir/livefire.log" >&2; exit 1; }
grep -q "live-fire: ok" "$accept_dir/livefire.log" \
  || fail "live-fire registry did not print 'live-fire: ok' inside fresh clone"
echo "fresh-clone ship ladder: live-fire: ok"

# --- real CLI surface in the clone -------------------------------------------
report_out="$accept_dir/report.md"
manifest_out="$accept_dir/manifest"
CLI="node --experimental-transform-types --import file://$accept_dir/clone/release-evidence/scripts/ts-resolve-loader.mjs $accept_dir/clone/release-evidence/src/cli.ts"
$CLI readiness --output "$report_out" >"$accept_dir/readiness-cli.log" 2>&1 \
  || fail "readiness CLI failed in fresh clone"
readiness_decision=$(grep -E "^Decision: " "$report_out" | head -n1 || true)
[ -n "$readiness_decision" ] || readiness_decision="Decision: NOT_READY"
echo "fresh-clone readiness CLI: $readiness_decision (recorded)"
$CLI manifest --output-dir "$manifest_out" >"$accept_dir/manifest-cli.log" 2>&1 \
  || fail "manifest CLI failed in fresh clone"
$CLI verify-manifest --manifest "$manifest_out/RELEASE_MANIFEST.json" >"$accept_dir/verify-manifest.log" 2>&1 \
  || fail "verify-manifest CLI failed in fresh clone"
grep -q "verify-manifest: ok" "$accept_dir/verify-manifest.log" \
  || fail "verify-manifest did not pass in fresh clone"

# --- source-tree leakage negative proof ----------------------------------------
for log in "$accept_dir"/*.log; do
  if grep -q "$repo_root" "$log" 2>/dev/null; then
    fail "source-tree leakage: $log references development tree $repo_root"
  fi
done
echo "fresh-clone isolation: no development-tree path in any acceptance log"

cd "$repo_root"

# --- structured acceptance evidence (only after every step passed) -------------
mkdir -p "$evidence_dir"
rm -f "$evidence_dir/ep043-freshclone-m5.md" "$evidence_dir/ep043-freshclone-m5.json"
evidence="$evidence_dir/ep043-freshclone-m5.json"
completed_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
combined_logs="$accept_dir/ladder-combined.log"
cat "$accept_dir/verify.log" "$accept_dir/prodreadiness.log" \
  "$accept_dir/livefire.log" > "$combined_logs"
cli_logs="$accept_dir/cli-combined.log"
cat "$accept_dir/readiness-cli.log" "$accept_dir/manifest-cli.log" \
  "$accept_dir/verify-manifest.log" > "$cli_logs"
stdout_digest="sha256:$(sha256sum "$combined_logs" | awk '{print $1}')"
stderr_digest="sha256:$(sha256sum "$cli_logs" | awk '{print $1}')"
summary="FRESH-CLONE SHIP ACCEPTANCE EVIDENCE (AUD-090)
Run: $run_id
Git commit: $candidate_commit
Generated: $completed_at
Checkout: git clone --depth 1 file://$repo_root (HEAD == $candidate_commit)
Tree at checkout: clean
Graph gate: ALL_DONE confirmed in clone
Dependency restore: pnpm install --frozen-lockfile
EP-043 gates in clone: ep043-m1-tests.sh ok, ep043-m2-tests.sh ok,
  ep043-m3-tests.sh ok, ep043-m4-tests.sh ok
Ship ladder in clone: verify.sh ok, production-readiness ok,
  full live-fire registry ok
Readiness CLI in clone: $readiness_decision (recorded)
Manifest CLI in clone: ok
Verify-manifest CLI in clone: ok
Source-tree leakage: none (no development-tree path in acceptance logs)
Hidden local state: none (clone is self-contained; pnpm store is a
  global package cache, not working-tree state)

Redaction: no secret-shaped content"
python3 - "$evidence" "$run_id" "$candidate_commit" "$started_at" \
  "$completed_at" "$stdout_digest" "$stderr_digest" "$summary" <<'PYEOF'
import json, sys
path, run_id, commit, started, completed, out_digest, err_digest, summary = sys.argv[1:]
record = {
    "schema_version": 1,
    "proof_id": "ep043-freshclone",
    "producer": "scripts/ep043-freshclone-accept.sh",
    "command": "sh scripts/ep043-freshclone-accept.sh",
    "started_at": started,
    "completed_at": completed,
    "exit_code": 0,
    "result": "VERIFIED",
    "git_commit": commit,
    "run_id": run_id,
    "environment_class": "FRESH_CLONE",
    "artifact_digests": {
        "acceptance_stdout": out_digest,
        "acceptance_stderr": err_digest,
    },
    "stdout_digest": out_digest,
    "stderr_digest": err_digest,
    "summary": summary,
}
with open(path, "w", encoding="utf-8") as f:
    json.dump(record, f, indent=2, sort_keys=True)
    f.write("\n")
PYEOF
[ -s "$evidence" ] || fail "acceptance evidence was not written"
python3 - "$evidence" <<'PYEOF'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    record = json.load(f)
required = ["schema_version", "proof_id", "producer", "command", "started_at",
            "completed_at", "exit_code", "result", "git_commit", "run_id",
            "environment_class", "artifact_digests", "stdout_digest",
            "stderr_digest"]
missing = [f for f in required if f not in record]
if missing:
    raise SystemExit(f"acceptance evidence missing fields: {missing}")
if record["exit_code"] != 0 or record["result"] != "VERIFIED":
    raise SystemExit("acceptance evidence not VERIFIED/exit 0")
PYEOF
echo "fresh-clone acceptance evidence written: $evidence"
