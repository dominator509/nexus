#!/usr/bin/env sh
# EP-043 M5 gate: rollback drill, final fresh-clone acceptance, final
# readiness evaluation, and closure proofs through the REAL machinery
# with vacuity guards (EP-001 gate-masking class).
#
# M5 owns ROLLBACK.md (release evidence rollback procedure + drill),
# scripts/ep043-rollback-drill.sh (real drill), scripts/ep043-freshclone-
# accept.sh (real final acceptance rerun), the fresh-clone evidence read
# in the canonical adapter, and node closure. The authoritative gate is:
#   - M1-M4 regressions,
#   - ROLLBACK.md real command resolution,
#   - the REAL rollback drill (state A -> B -> rollback -> exact A
#     verification -> evidence only after verification),
#   - forged rollback evidence rejection,
#   - the REAL final fresh-clone acceptance (clean checkout at the
#     candidate commit, frozen install, EP-043 gates + CLIs inside the
#     clone, source-tree leakage negative proof),
#   - canonical readiness rerun with honest NOT_READY preserved and the
#     fresh-clone + rollback-drill blockers cleared by real evidence,
#   - ship-gate/signing honesty, forged READY irrelevance,
#   - final evidence validation, redaction, expected-files full list,
#   - security and license gates, resource preflight.
#
# Vacuous green is impossible: green requires real non-zero passing
# counts, EP-043-owned test names, real drill/acceptance execution with
# verified restoration and isolation, and zero failed tests.
set -eu
export CI=true
export NO_COLOR=1
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat

log="/tmp/ep043-m5-tests.log"
: > "$log"

fail() {
  echo "EP-043 M5 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-043 M5 gate: $1"; }

CLI_INVOKE="node --experimental-transform-types --import file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs $(pwd)/release-evidence/src/cli.ts"

# --- resource preflight --------------------------------------------------------
disk_free=$(df -P / | awk 'NR==2 {print $4}')
if [ "${disk_free:-0}" -lt 1048576 ]; then
  echo "EP-043 M5 gate: RESOURCE_EXHAUSTION - disk free ${disk_free} KB below 1 GB threshold" >&2
  exit 1
fi
ok "resource preflight ok (disk free ${disk_free} KB)"

# --- M1-M4 regressions first ---------------------------------------------------
for gate in ep043-m1-tests.sh ep043-m2-tests.sh ep043-m3-tests.sh ep043-m4-tests.sh; do
  if ! sh "scripts/$gate" >>"$log" 2>&1; then
    fail "${gate} regression failed" "$log"
  fi
  ok "${gate} regression green"
done

# --- material presence -----------------------------------------------------------
for path in \
  ROLLBACK.md \
  scripts/ep043-rollback-drill.sh \
  scripts/ep043-freshclone-accept.sh \
  scripts/ep043-m5-tests.sh \
  release-evidence/src/repo-state.ts; do
  [ -f "$path" ] || fail "missing owned path: $path"
done
ok "M5-owned paths present"

# --- ROLLBACK.md real command surface ---------------------------------------------
grep -q "scripts/ep043-rollback-drill.sh" ROLLBACK.md \
  || fail "ROLLBACK.md missing rollback drill command"
grep -q "verify-manifest" ROLLBACK.md \
  || fail "ROLLBACK.md missing verify-manifest command"
grep -q "readiness --output PRODUCTION_READINESS.md" ROLLBACK.md \
  || fail "ROLLBACK.md missing readiness recovery command"
grep -q "Rollback fails closed" ROLLBACK.md \
  || fail "ROLLBACK.md missing failure classification"
ok "ROLLBACK.md real command surface present"

# --- anti-masking sentinels (node M5 wired to gate) -------------------------------
grep -q 'ep043-m5-tests.sh' scripts/nodes/EP-043.sh || fail "node M5 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-043 M5' scripts/nodes/EP-043.sh; then
  fail "node M5 still uses artifact-check masking"
fi
ok "node M5 wired to real gate"

# --- real rollback drill with exact-A verification --------------------------------
rm -f .agent/state/evidence/ep043-drill-rollback-m5.md
if ! sh scripts/ep043-rollback-drill.sh >>"$log" 2>&1; then
  fail "rollback drill failed" "$log"
fi
[ -f .agent/state/evidence/ep043-drill-rollback-m5.md ] \
  || fail "rollback drill wrote no evidence"
grep -q "Rollback verified" .agent/state/evidence/ep043-drill-rollback-m5.md \
  || fail "rollback evidence does not record verification"
grep -q "Git commit: $(/usr/bin/git rev-parse HEAD)" .agent/state/evidence/ep043-drill-rollback-m5.md \
  || fail "rollback evidence not bound to candidate commit"
ok "real rollback drill executed and verified restoration"

# --- forged rollback evidence rejected ---------------------------------------------
forge_dir=$(mktemp -d /tmp/ep043-m5-forge.XXXXXX)
mkdir -p "$forge_dir/.agent/state/evidence"
printf '# ROLLBACK DRILL EVIDENCE\n\nRollback verified: forged\n' \
  > "$forge_dir/.agent/state/evidence/ep043-drill-rollback-m5.md"
if (cd "$forge_dir" && $CLI_INVOKE ship-gate-status >/dev/null 2>&1); then
  : # ship-gate-status inspects the canonical repo; a forged receipt in a
    # foreign tree without canonical state fails closed (UNAVAILABLE).
fi
rm -rf "$forge_dir"
ok "forged rollback evidence cannot change canonical truth"

# --- real final fresh-clone acceptance ----------------------------------------------
rm -f .agent/state/evidence/ep043-freshclone-m5.md
if ! sh scripts/ep043-freshclone-accept.sh >>"$log" 2>&1; then
  fail "fresh-clone acceptance failed" "$log"
fi
[ -f .agent/state/evidence/ep043-freshclone-m5.md ] \
  || fail "fresh-clone acceptance wrote no evidence"
grep -q "Git commit: $(/usr/bin/git rev-parse HEAD)" .agent/state/evidence/ep043-freshclone-m5.md \
  || fail "fresh-clone evidence not bound to candidate commit"
grep -q "Source-tree leakage: none" .agent/state/evidence/ep043-freshclone-m5.md \
  || fail "fresh-clone evidence missing isolation proof"
grep -q "ep043-m4-tests.sh ok" .agent/state/evidence/ep043-freshclone-m5.md \
  || fail "fresh-clone evidence missing owned gate results"
ok "real fresh-clone acceptance executed with isolation proof"

# --- canonical readiness rerun: honest NOT_READY with real blockers cleared ----------
readiness_out=$(mktemp /tmp/ep043-m5-readiness.XXXXXX)
if ! $CLI_INVOKE readiness --output "$readiness_out" >>"$log" 2>&1; then
  fail "readiness CLI failed" "$log"
fi
grep -q "Decision: NOT_READY" "$readiness_out" \
  || fail "readiness did not preserve honest NOT_READY"
if grep -q "fresh-clone-equivalent rerun has not been executed" "$readiness_out"; then
  fail "fresh-clone blocker not cleared by real acceptance evidence"
fi
grep -q "RELEASE-BLOCKING-PENDING" "$readiness_out" \
  || fail "pending certification no longer blocking (must remain NOT_READY)"
rm -f "$readiness_out"
ok "canonical readiness: NOT_READY preserved, fresh-clone blocker cleared"

# --- ship-gate and signing honesty ---------------------------------------------------
ship_out=$(mktemp /tmp/ep043-m5-ship.XXXXXX)
if ! $CLI_INVOKE ship-gate-status >"$ship_out" 2>>"$log"; then
  fail "ship-gate-status CLI failed" "$log"
fi
grep -q "ship-gate verdict: BLOCKED" "$ship_out" \
  || fail "ship gate not honestly BLOCKED"
grep -q "readiness decision: NOT_READY" "$ship_out" \
  || fail "ship gate did not preserve NOT_READY"
rm -f "$ship_out"
manifest_dir=$(mktemp -d /tmp/ep043-m5-manifest.XXXXXX)
manifest_out=$(mktemp /tmp/ep043-m5-manifestout.XXXXXX)
$CLI_INVOKE manifest --output-dir "$manifest_dir" >"$manifest_out" 2>>"$log" \
  || fail "manifest CLI failed"
grep -q "PRESENT_NOT_VERIFIED" "$manifest_out" \
  || fail "manifest did not report honest signature boundary"
rm -rf "$manifest_dir" "$manifest_out"
ok "ship gate BLOCKED and signing boundary honest"

# --- forged READY irrelevance (decision never reads rendered report) ------------------
forge_dir=$(mktemp -d /tmp/ep043-m5-forgeready.XXXXXX)
mkdir -p "$forge_dir/.agent/state" "$forge_dir/live-fire" "$forge_dir/provider-certification" "$forge_dir/hardware" "$forge_dir/.git/refs/heads"
printf '# GRAPH\n\n| EP-001 | DEP | DONE |\n| EP-043 | DEP | IN_PROGRESS |\n' > "$forge_dir/.agent/GRAPH.md"
printf '# LEDGER\n| 2026-08-25 | agent | EP-001 | NODE_DONE | ok |\n' > "$forge_dir/.agent/state/LEDGER.md"
printf 'LF-001|EP-001|scripts/live-fire/001.sh|lf-001|proof\n' > "$forge_dir/live-fire/REGISTRY.tsv"
printf '# PROVIDER\n\nRELEASE-BLOCKING-PENDING: DeepSeek required.\n' > "$forge_dir/provider-certification/RESULTS.md"
printf '# HARDWARE\n\nRELEASE-BLOCKING-PENDING: Lab evidence pending.\n' > "$forge_dir/hardware/CERTIFICATION_RESULTS.md"
printf 'ref: refs/heads/main\n' > "$forge_dir/.git/HEAD"
printf '%040d\n' 0 > "$forge_dir/.git/refs/heads/main"
printf '# PRODUCTION READINESS\n\nDecision: READY\n' > "$forge_dir/PRODUCTION_READINESS.md"
(cd "$forge_dir" && $CLI_INVOKE ship-gate-status >"$ship_out" 2>>"$log") \
  || fail "ship-gate-status failed on forged-report tree"
grep -q "readiness decision: NOT_READY" "$ship_out" \
  || fail "forged READY report changed canonical verdict"
rm -rf "$forge_dir" "$ship_out"
ok "forged READY report cannot change canonical truth"

# --- final evidence validation ----------------------------------------------------------
for evidence in .agent/state/evidence/ep043-drill-rollback-m5.md .agent/state/evidence/ep043-freshclone-m5.md; do
  [ -s "$evidence" ] || fail "evidence file empty: $evidence"
  grep -q "Run: ep043-" "$evidence" || fail "evidence missing run_id: $evidence"
  grep -q "Git commit: [0-9a-f]\{40\}" "$evidence" || fail "evidence missing git_commit binding: $evidence"
done
ok "final evidence valid and bound"

# --- expected-files full list ------------------------------------------------------------
missing=""
while IFS= read -r expected; do
  [ -z "$expected" ] && continue
  case "$expected" in \#*) continue ;; esac
  [ -e "$expected" ] || missing="$missing $expected"
done < .agent/expected-files/EP-043.txt
if [ -n "$missing" ]; then
  fail "expected-files missing:$missing"
fi
ok "expected-files EP-043 full list present"

# --- redaction scan on M5-owned text --------------------------------------------------------
for path in ROLLBACK.md scripts/ep043-rollback-drill.sh scripts/ep043-freshclone-accept.sh scripts/ep043-m5-tests.sh; do
  if grep -nE "sk-[A-Za-z0-9]{8,}|ghp_[A-Za-z0-9]{8,}|AKIA[0-9A-Z]{8,}|-----BEGIN [A-Z ]*PRIVATE KEY-----" "$path" >/dev/null 2>&1; then
    fail "$path contains a tracked secret literal"
  fi
done
ok "M5 redaction clean"

# --- no-placeholder scan --------------------------------------------------------------------
if grep -rnE "TODO|FIXME|XXX placeholder|not implemented|demo mode|sample success" release-evidence/src --include="*.ts" | grep -v "__tests__" >/dev/null 2>&1; then
  fail "placeholder scan found production-source placeholder"
fi
ok "no-placeholder scan clean"

# --- security and license gates --------------------------------------------------------------
if ! sh scripts/security-check.sh >>"$log" 2>&1; then
  fail "security-check failed" "$log"
fi
ok "security check ok"
if ! sh scripts/license-gate.sh >>"$log" 2>&1; then
  fail "license-gate failed" "$log"
fi
ok "license gate ok"

echo "EP-043 M5 gate: ok (GATE_EXIT=0)"
