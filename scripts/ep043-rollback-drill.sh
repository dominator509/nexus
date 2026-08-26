#!/usr/bin/env sh
# EP-043 M5 rollback drill (SPEC-008 drill evidence).
#
# Real bounded local drill against the release evidence:
#   known state A  = committed PRODUCTION_READINESS.md (exact committed
#                    bytes) plus the canonical manifest component
#                    digests (generated artifact, dist/ is gitignored)
#   capture A      = sha256 of the committed report + component digests
#                    of a fresh canonical manifest generation
#   apply state B  = isolated bad release (forged READY report bytes +
#                    corrupted manifest bytes)
#   verify B       = digests differ from A (the bad state is real)
#   execute rollback = git restore the committed report and regenerate
#                    the manifest from canonical state (bounded recovery)
#   restore A      = exact committed report bytes + canonical manifest
#   verify A       = sha256 of restored report == captured A AND
#                    regenerated component digests == captured A AND
#                    release-evidence CLI verify-manifest passes
#   evidence       = dated drill evidence written ONLY after verification
#
# The drill runs in a throwaway clone so the development tree is never
# mutated. It fails closed on any missing/corrupt/mismatched input and
# never writes a receipt before verification.
#
# Usage:
#   sh scripts/ep043-rollback-drill.sh [--evidence-dir .agent/state/evidence]
set -eu
export CI=true
export NO_COLOR=1
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat

repo_root=$(pwd)
evidence_dir="${1:-.agent/state/evidence}"
run_id="ep043-rollback-drill-$(date +%s)"
candidate_commit=$(/usr/bin/git rev-parse HEAD 2>/dev/null || echo "unknown")
CLI="node --experimental-transform-types --import file://$repo_root/release-evidence/scripts/ts-resolve-loader.mjs $repo_root/release-evidence/src/cli.ts"

fail() {
  echo "ep043-rollback-drill: FAIL - $1" >&2
  exit 1
}

report_path="PRODUCTION_READINESS.md"
manifest_path="dist/release/RELEASE_MANIFEST.json"

# --- isolated drill clone at the candidate commit ----------------------------
drill_dir=$(mktemp -d /tmp/ep043-m5-drill.XXXXXX)
trap 'rm -rf "$drill_dir"' EXIT INT TERM
git clone --quiet --depth 1 "file://$repo_root" "$drill_dir/clone" \
  || fail "cannot clone committed tree for drill"
cd "$drill_dir/clone"
clone_commit=$(/usr/bin/git rev-parse HEAD)
[ "$clone_commit" = "$candidate_commit" ] \
  || fail "drill clone is not the candidate commit ($clone_commit != $candidate_commit)"

# --- state A: committed report + canonical manifest digests (from the clone) --
[ -f "$report_path" ] || fail "missing committed release report: $report_path"
a_report=$(sha256sum "$report_path" | awk '{print $1}')

probe_dir="$drill_dir/probe"
$CLI manifest --output-dir "$probe_dir" >/dev/null 2>&1 \
  || fail "cannot generate canonical manifest for state A capture"
python3 - "$probe_dir/RELEASE_MANIFEST.json" "$probe_dir/digests.txt" <<'PYEOF'
import json, sys
with open(sys.argv[1]) as f:
    manifest = json.load(f)
with open(sys.argv[2], "w") as f:
    for c in manifest["components"]:
        f.write(f"{c['component_id']}={c['digest']}\n")
PYEOF
a_digests=$(sort "$probe_dir/digests.txt")
echo "state A captured: report=$a_report"
echo "state A component digests:"; echo "$a_digests"

# --- apply state B: bad release (forged report + corrupted manifest) ---------
printf '# PRODUCTION READINESS\n\nDecision: READY (FORGED)\n' > "$report_path"
mkdir -p dist/release
printf '{"release_id":"BAD","components":[{"digest":"sha256:%064d"}]}\n' 0 > "$manifest_path"

b_report=$(sha256sum "$report_path" | awk '{print $1}')
b_manifest=$(sha256sum "$manifest_path" | awk '{print $1}')
[ "$b_report" != "$a_report" ] \
  || fail "state B did not actually change the release report"
echo "state B applied: report=$b_report manifest=$b_manifest"

# --- execute actual rollback ---------------------------------------------------
/usr/bin/git restore --source=HEAD -- "$report_path" \
  || fail "rollback restore of committed report failed"
$CLI manifest --output-dir dist/release >/dev/null 2>&1 \
  || fail "rollback regeneration of manifest failed"

# --- verify exact A restored -----------------------------------------------------
r_report=$(sha256sum "$report_path" | awk '{print $1}')
[ "$r_report" = "$a_report" ] || fail "report not restored to state A"
[ -f "$manifest_path" ] || fail "manifest missing after rollback"

r_digests=$(python3 - "$manifest_path" <<'PYEOF'
import json, sys
with open(sys.argv[1]) as f:
    manifest = json.load(f)
for c in manifest["components"]:
    print(f"{c['component_id']}={c['digest']}")
PYEOF
)
[ "$(printf '%s\n' "$r_digests" | sort)" = "$a_digests" ] \
  || fail "regenerated manifest digests do not match state A"

$CLI verify-manifest --manifest "$manifest_path" >/dev/null 2>&1 \
  || fail "verify-manifest failed after rollback"

echo "rollback verified: report=$r_report"
echo "rollback verified component digests:"; echo "$r_digests"
cd "$repo_root"

# --- evidence only after verification ------------------------------------------
mkdir -p "$evidence_dir"
evidence="$evidence_dir/ep043-drill-rollback.md"
cat > "$evidence" <<EOF
# ROLLBACK DRILL EVIDENCE

Run: $run_id
Git commit: $candidate_commit
Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)

State A captured: report $a_report (committed bytes)
State A manifest component digests:
$(printf '%s\n' "$a_digests" | sed 's/^/  /')
State B applied: forged READY report + corrupted manifest (isolated)
Rollback executed: git restore committed report; manifest regenerated
Rollback verified: report $r_report (exact A restored); component
  digests match state A; verify-manifest ok

Redaction: no secret-shaped content
EOF
echo "evidence written: $evidence"
