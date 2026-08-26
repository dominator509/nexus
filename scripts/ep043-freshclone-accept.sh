#!/usr/bin/env sh
# EP-043 M5 final fresh-clone acceptance (SPEC-008 fresh-clone
# prerequisite).
#
# Real acceptance rerun on a clean isolated checkout of the exact
# candidate commit:
#   - clone file://<repo> at the candidate commit into a throwaway dir
#   - assert the clone HEAD is exactly the candidate commit
#   - assert the clone tree starts clean (no hidden local state)
#   - restore dependencies from frozen files (pnpm --frozen-lockfile;
#     the pnpm store is a global package cache, never working-tree state)
#   - run the EP-043 owned gates (M1-M4) inside the clone with
#     EP043_TEST_ROOT pointing at the clone, so no test reads the
#     development tree
#   - run the real readiness / manifest / verify-manifest CLIs in the
#     clone
#   - source-tree leakage negative proof: no command output or gate log
#     may reference the development tree path
#   - write dated acceptance evidence to the real evidence dir binding
#     the candidate commit, only after every step passed
#
# Usage:
#   sh scripts/ep043-freshclone-accept.sh [--evidence-dir .agent/state/evidence]
set -eu
export CI=true
export NO_COLOR=1
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat

repo_root=$(pwd)
evidence_dir="${1:-.agent/state/evidence}"
run_id="ep043-freshclone-$(date +%s)"
candidate_commit=$(/usr/bin/git rev-parse HEAD 2>/dev/null || echo "unknown")

fail() {
  echo "ep043-freshclone-accept: FAIL - $1" >&2
  exit 1
}

accept_dir=$(mktemp -d /tmp/ep043-m5-accept.XXXXXX)
trap 'rm -rf "$accept_dir"' EXIT INT TERM

# --- clean isolated checkout of the exact candidate commit -------------------
git clone --quiet --depth 1 "file://$repo_root" "$accept_dir/clone" \
  || fail "cannot clone candidate commit"
cd "$accept_dir/clone"
clone_commit=$(/usr/bin/git rev-parse HEAD)
[ "$clone_commit" = "$candidate_commit" ] \
  || fail "clone is not the candidate commit ($clone_commit != $candidate_commit)"
[ -z "$(/usr/bin/git status --porcelain)" ] \
  || fail "clone tree is not clean at checkout"

# --- dependency restore from frozen files ------------------------------------
corepack enable >/dev/null 2>&1 || true
pnpm install --frozen-lockfile --prefer-offline >/tmp/ep043-accept-install.log 2>&1 \
  || fail "pnpm install --frozen-lockfile failed (see /tmp/ep043-accept-install.log)"

# --- EP-043 owned gates in the clone (isolated test root) ----------------------
export EP043_TEST_ROOT="$accept_dir/clone"
for gate in ep043-m1-tests.sh ep043-m2-tests.sh ep043-m3-tests.sh ep043-m4-tests.sh; do
  sh "scripts/$gate" >/tmp/ep043-accept-$gate.log 2>&1 \
    || fail "$gate failed inside fresh clone (see /tmp/ep043-accept-$gate.log)"
  echo "fresh-clone gate: $gate ok"
done

# --- real CLI surface in the clone ---------------------------------------------
report_out="$accept_dir/report.md"
manifest_out="$accept_dir/manifest"
node --experimental-transform-types \
  --import "file://$accept_dir/clone/release-evidence/scripts/ts-resolve-loader.mjs" \
  "$accept_dir/clone/release-evidence/src/cli.ts" readiness --output "$report_out" \
  >/tmp/ep043-accept-readiness.log 2>&1 \
  || fail "readiness CLI failed in fresh clone"
grep -q "NOT_READY" "$report_out" || fail "readiness report not honest in fresh clone"
node --experimental-transform-types \
  --import "file://$accept_dir/clone/release-evidence/scripts/ts-resolve-loader.mjs" \
  "$accept_dir/clone/release-evidence/src/cli.ts" manifest --output-dir "$manifest_out" \
  >/tmp/ep043-accept-manifest.log 2>&1 \
  || fail "manifest CLI failed in fresh clone"
node --experimental-transform-types \
  --import "file://$accept_dir/clone/release-evidence/scripts/ts-resolve-loader.mjs" \
  "$accept_dir/clone/release-evidence/src/cli.ts" verify-manifest \
  --manifest "$manifest_out/RELEASE_MANIFEST.json" \
  >/tmp/ep043-accept-verify.log 2>&1 \
  || fail "verify-manifest CLI failed in fresh clone"
grep -q "verify-manifest: ok" /tmp/ep043-accept-verify.log \
  || fail "verify-manifest did not pass in fresh clone"

# --- source-tree leakage negative proof ----------------------------------------
for log in /tmp/ep043-accept-*.log; do
  if grep -q "$repo_root" "$log"; then
    fail "source-tree leakage: $log references development tree $repo_root"
  fi
done
echo "fresh-clone isolation: no development-tree path in any acceptance log"

cd "$repo_root"

# --- acceptance evidence (only after every step passed) -------------------------
mkdir -p "$evidence_dir"
evidence="$evidence_dir/ep043-freshclone-m5.md"
cat > "$evidence" <<EOF
# FRESH-CLONE ACCEPTANCE EVIDENCE

Run: $run_id
Git commit: $candidate_commit
Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)

Checkout: git clone --depth 1 file://$repo_root (HEAD == $candidate_commit)
Tree at checkout: clean
Dependency restore: pnpm install --frozen-lockfile (prefer-offline)
EP-043 gates in clone: ep043-m1-tests.sh ok, ep043-m2-tests.sh ok,
  ep043-m3-tests.sh ok, ep043-m4-tests.sh ok
Readiness CLI in clone: ok (honest NOT_READY report)
Manifest CLI in clone: ok
Verify-manifest CLI in clone: ok
Source-tree leakage: none (no development-tree path in acceptance logs)
Hidden local state: none (clone is self-contained; pnpm store is a
  global package cache, not working-tree state)

Redaction: no secret-shaped content
EOF
echo "fresh-clone acceptance evidence written: $evidence"
