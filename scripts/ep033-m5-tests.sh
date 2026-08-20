#!/usr/bin/env sh
# EP-033 M5 gate: web/desktop accessibility + LF-005 cross-device
# continuity proof through the REAL vitest machinery with vacuity
# guards, production-import guards, anti-masking sentinels, real
# browser evidence freshness, and M1-M4 regressions.
#
# M5 owns:
#   - WCAG 2.2 A/AA machine-observed scan of the REAL server-rendered
#     production @nexus/ui surfaces in REAL headless Chrome (axe-core)
#   - LF-005 cross-device continuity: voice start -> web dashboard
#     continue -> mobile FOUR_EYES approval -> final artifact in the
#     same task graph, composed of real production components
#   - telemetry/redaction canary proofs at the a11y package layer
#
# The gate FAILS on a zero-match filter, on stale evidence, on missing
# owned proof names, on skipped tests, or on a mock-only substitute.
set -eu
export CI=true

log="/tmp/ep033-m5-tests.log"
: > "$log"

fail() {
  echo "EP-033 M5 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-033 M5 gate: $1"; }

PNPM="${PNPM_BIN:-/root/.local/share/mise/installs/pnpm/11.17.0/pnpm}"
PKG="tests/accessibility/web"

# Vacuity guard 0: the a11y package must exist.
if [ ! -f "$PKG/package.json" ]; then
  fail "$PKG/package.json missing"
fi

# Vacuity guard 0b: all owned a11y source files and test files must exist.
for f in \
  src/harness.tsx \
  src/scan.ts \
  src/lf005.ts \
  src/__tests__/ep033_a11y_unit.test.ts \
  src/__tests__/ep033_a11y_browser_scan.test.ts \
  src/__tests__/ep033_lf005_evidence.test.ts; do
  if [ ! -f "$PKG/$f" ]; then
    fail "$PKG/$f missing"
  fi
done
ok "a11y package and owned source/test files present"

# Production-import guard: the suite composes the REAL production
# components, never a mock-only substitute.
if ! grep -q 'from "@nexus/ui"' "$PKG/src/harness.tsx"; then
  fail "harness does not render production @nexus/ui components"
fi
if ! grep -q 'from "@nexus/web"' "$PKG/src/lf005.ts"; then
  fail "LF-005 proof does not import production @nexus/web contracts"
fi
if ! grep -q 'from "@nexus/desktop"' "$PKG/src/lf005.ts"; then
  fail "LF-005 proof does not import production @nexus/desktop components"
fi
if grep -rqE 'vi\.mock\(|mock\([^)]*\)' "$PKG/src"; then
  fail "mock-only substitute detected in a11y package"
fi
ok "production components imported; no mock-only substitute"

# Real typecheck: tsc --noEmit must pass.
if ! (cd "$PKG" && "$PNPM" exec tsc --noEmit >>"$log" 2>&1); then
  fail "tsc --noEmit failed" "$log"
fi
ok "tsc --noEmit clean"

# Real test run: the full a11y suite through vitest with the verbose
# reporter so the anti-masking greps can observe the exact proofs.
if ! (cd "$PKG" && "$PNPM" exec vitest run src/__tests__ --reporter=verbose >>"$log" 2>&1); then
  fail "vitest run failed" "$log"
fi

# vitest emits ANSI color codes even under CI=true; strip them so the
# vacuity greps observe plain text.
sed -i 's/\x1b\[[0-9;]*m//g' "$log"

# Vacuity guard 1: a non-zero number of tests actually ran.
if ! grep -qE 'Tests[[:space:]]+[1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: zero failures observed.
if grep -qE '[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 3: zero skipped/ignored tests.
if grep -qE 'Tests[[:space:]]+[0-9]+ passed \([0-9]+ skipped' "$log"; then
  fail "required tests were skipped (vacuity guard)" "$log"
fi

# Vacuity guard 4 (anti-masking): the exact EP-033-owned a11y test
# files must be observed - not the web unit suite, the desktop unit
# suite, the ui suite, or a zero-match filter.
for sentinel in \
  "ep033_a11y_unit.test.ts" \
  "ep033_a11y_browser_scan.test.ts" \
  "ep033_lf005_evidence.test.ts"; do
  if ! grep -q "$sentinel" "$log"; then
    fail "a11y test file did not run: $sentinel (anti-masking guard)" "$log"
  fi
done

# Vacuity guard 5 (anti-masking): exact owned proof names observed,
# proving the concrete behaviors executed.
for sentinel in \
  "renders the owned production surfaces into a complete document" \
  "renders the FOUR_EYES requirement verbatim" \
  "renders a disabled capability button for visible-but-unauthorized" \
  "renders stale status non-color" \
  "includes a skip link and main landmark" \
  "binds the voice-started objective in the web dashboard via correlation" \
  "satisfies mobile FOUR_EYES approval with two distinct principals" \
  "delivers the final artifact in the same task graph" \
  "preserves the UI authority distinctions" \
  "binds a current-run identity to every journey" \
  "imports only @nexus packages, react, and the scan toolchain" \
  "scans the rendered owned surfaces with axe-core WCAG 2.2 A/AA in real Chrome and writes current-run evidence" \
  "writes current-run LF-005 evidence that the gate can observe"; do
  if ! grep -qF "$sentinel" "$log"; then
    fail "EP-033-owned proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
ok "all three a11y test files and 13 owned proof names observed"

total=$(grep -oE 'Tests[[:space:]]+[1-9][0-9]* passed' "$log" | awk '{s+=$2} END {print s}')
ok "real a11y suite passed (${total} tests total)"

# Evidence freshness guard: the real browser scan and the LF-005
# journey must have written current-run machine-readable evidence with
# matching run_id binding. Stale evidence never satisfies the gate.
if [ ! -f .agent/state/evidence/LF-005-ep033-m5.json ]; then
  fail "browser-scan evidence missing (LF-005-ep033-m5.json)"
fi
if [ ! -f .agent/state/evidence/LF-005-ep033-m5-lf005.json ]; then
  fail "LF-005 journey evidence missing (LF-005-ep033-m5-lf005.json)"
fi
if ! grep -q '"violations": \[\]' .agent/state/evidence/LF-005-ep033-m5.json; then
  fail "browser-scan evidence records violations (WCAG 2.2 A/AA not clean)"
fi
if ! grep -q '"node": "EP-033"' .agent/state/evidence/LF-005-ep033-m5.json; then
  fail "browser-scan evidence not bound to EP-033"
fi
if ! grep -q '"milestone": "M5"' .agent/state/evidence/LF-005-ep033-m5.json; then
  fail "browser-scan evidence not bound to M5"
fi
# The scan evidence must be fresh (written within the last 10 minutes).
if ! find .agent/state/evidence/LF-005-ep033-m5.json -mmin -10 | grep -q .; then
  fail "browser-scan evidence is stale (older than 10 minutes)"
fi
if ! find .agent/state/evidence/LF-005-ep033-m5-lf005.json -mmin -10 | grep -q .; then
  fail "LF-005 journey evidence is stale (older than 10 minutes)"
fi
# The browser scan must have exercised a real browser (axe-core version
# recorded) and observed real passes.
if ! grep -q '"engine": "axe-core"' .agent/state/evidence/LF-005-ep033-m5.json; then
  fail "browser-scan evidence does not record the axe-core engine"
fi
if ! grep -qE '"passes": [1-9][0-9]*' .agent/state/evidence/LF-005-ep033-m5.json; then
  fail "browser-scan evidence records zero axe passes (vacuity)"
fi
ok "current-run evidence fresh and bound (browser scan + LF-005 journey)"

# LF-005 runner integrity: the live-fire script must call THIS real
# gate, never a dangling proof-runner / nexus-cli.
if ! grep -q 'sh scripts/ep033-m5-tests.sh' scripts/live-fire/LF-005.sh; then
  fail "scripts/live-fire/LF-005.sh does not call the real M5 gate"
fi
if grep -q 'proof-runner.sh' scripts/live-fire/LF-005.sh; then
  fail "scripts/live-fire/LF-005.sh still delegates to the dangling proof-runner"
fi
ok "LF-005 live-fire wired to the real gate"

# M1-M4 regressions: M5 must not weaken the prior milestones.
if ! sh scripts/ep033-m1-tests.sh >>"$log" 2>&1; then
  fail "M1 regression (web contract suite) failed" "$log"
fi
if ! sh scripts/ep033-m2-tests.sh >>"$log" 2>&1; then
  fail "M2 regression (desktop suite) failed" "$log"
fi
if ! sh scripts/ep033-m3-tests.sh >>"$log" 2>&1; then
  fail "M3 regression (ui suite) failed" "$log"
fi
if ! sh scripts/ep033-m4-tests.sh >>"$log" 2>&1; then
  fail "M4 regression (e2e failure suite) failed" "$log"
fi
ok "M1/M2/M3/M4 regressions green"

# Orphan guard: the a11y suite launches a real browser that must be
# closed; any stray vitest worker or chrome child would be an orphan.
stray=$(ps aux | grep -E '[v]itest.*accessibility/web' | sed '/^$/d')
if [ -n "$stray" ]; then
  echo "EP-033 M5 orphan guard: FAIL - stray vitest processes:" >&2
  echo "$stray" >&2
  exit 1
fi
if [ -f /tmp/ep033-m5-scratch ]; then
  echo "EP-033 M5 orphan guard: FAIL - scratch marker present" >&2
  exit 1
fi
ok "orphan guard clean"

# Milestone artifact/fence checks.
for f in .agent/milestone-files/EP-033-M5.txt .agent/node-contracts/EP-033.md \
         .agent/execplans/EP-033-web-dashboard-and-desktop.md "$PKG/package.json"; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-033 M5: ok"
