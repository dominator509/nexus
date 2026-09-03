#!/usr/bin/env sh
# RX-004 regression battery: build/test/reality-gate truth.
# AUD-004: BUILD GREEN != PRODUCTION ARTIFACT EXISTS - missing required
#          artifact must fail the build; no silent fallback.
# AUD-005: security-check proves the SECURITY.md claims it makes.
# AUD-062: EP-040 M5 gate must wire three consecutive full verifies into
#          the canonical ConsecutiveVerify policy (not just unit tests).
# AUD-063: performance certification must not be vacuous (RX-021 shared).
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

# --- AUD-004: build.sh must fail closed on missing required artifacts ---
# The fallback patterns must not exist as executable lines (comments are
# allowed to explain the doctrine; strip them before matching).
if grep -E "^[^#]*compileall" scripts/build.sh; then
  bad "build.sh still has compileall fallback (AUD-004)"
else
  note "build.sh has no compileall fallback"
fi
if grep -E "^[^#]*(uv build|compileall|cargo build|pnpm build)[^#]*2>/dev/null[^#]*\|\|" scripts/build.sh; then
  bad "build.sh still has silent-fallback pattern (AUD-004)"
else
  note "build.sh has no silent-fallback pattern"
fi
for needle in "required artifact missing" "uv build" "flutter build bundle"; do
  grep -q "$needle" scripts/build.sh && note "build.sh covers $needle" || bad "build.sh missing $needle"
done
# Hostile check: when the python producer cannot run, the missing wheel
# must fail the build (no fallback, no vacuous pass). Shadow `uv` with a
# failing stub on PATH.
if [ -f pyproject.toml ]; then
  stubdir=$(mktemp -d)
  printf '#!/bin/sh\nexit 1\n' > "$stubdir/uv"
  chmod +x "$stubdir/uv"
  if PATH="$stubdir:$PATH" sh scripts/build.sh >/tmp/rx004-build-hostile.log 2>&1; then
    bad "build passed with uv broken and wheel missing (AUD-004 hostile)"
  else
    note "build fails closed when the python producer cannot run"
  fi
  rm -rf "$stubdir"
fi

# --- AUD-005: security-check surface matches SECURITY.md claims ---
# SECURITY.md claims secret scan, dep audit, static analysis, policy
# tests, license + reality gates. The script must exercise them.
for needle in "cargo audit" "pnpm audit" "license-gate.sh" "reality-gate.sh" "cargo clippy" "nexus-security-core"; do
  grep -q "$needle" scripts/security-check.sh && note "security-check covers $needle" || bad "security-check missing $needle"
done
# SECURITY.md must not promise surfaces the script cannot prove; the
# claimed surface must match what the script actually runs.
if grep -qi "IaC/container image scans" SECURITY.md && grep -qi "NOT part of this script" SECURITY.md; then
  note "SECURITY.md honestly scopes the asserted surface"
else
  bad "SECURITY.md does not honestly scope security-check surface"
fi
# The script must pass on the current tree (real gate).
if sh scripts/security-check.sh >/tmp/rx004-sec.log 2>&1; then
  note "security-check passes on current tree"
else
  bad "security-check fails on current tree: $(tail -3 /tmp/rx004-sec.log)"
fi

# --- AUD-062: M5 gate and node verify must wire consecutive-verify ---
if grep -q "ep040-consecutive-verify.sh" scripts/ep040-m5-tests.sh; then
  note "M5 gate invokes the consecutive-verify gate"
else
  bad "M5 gate does not invoke consecutive-verify gate (AUD-062)"
fi
if grep -q "ep040-consecutive-verify.sh" scripts/nodes/EP-040.sh; then
  note "node EP-040 verify invokes the consecutive-verify gate"
else
  bad "node EP-040 verify does not invoke consecutive-verify gate (AUD-062)"
fi
if grep -q "ConsecutiveVerify::new(3)" tests/integration/src/bin/consecutive_verify_gate.rs; then
  note "consecutive-verify harness uses the canonical policy (new(3))"
else
  bad "consecutive-verify harness does not use ConsecutiveVerify::new(3)"
fi
# Hostile: the harness must reject 3 REDs and accept 3 GREENs.
if cargo build -p nexus-test-execution --bin consecutive_verify_gate --locked >/tmp/rx004-harness-build.log 2>&1; then
  bin=$(find target/debug -maxdepth 1 -name 'consecutive_verify_gate' -type f | head -n 1)
  if [ -n "$bin" ]; then
    if printf 'RED\nRED\nRED\n' | "$bin" >/dev/null 2>&1; then
      bad "harness accepted 3 REDs (hostile)"
    else
      note "harness rejects 3 REDs"
    fi
    if printf 'GREEN\nGREEN\nGREEN\n' | "$bin" >/dev/null 2>&1; then
      note "harness accepts 3 GREENs"
    else
      bad "harness rejected 3 GREENs"
    fi
    if printf 'GREEN\nRED\nGREEN\n' | "$bin" >/dev/null 2>&1; then
      bad "harness accepted interrupted sequence (hostile)"
    else
      note "harness rejects interrupted sequence"
    fi
  else
    bad "consecutive_verify_gate binary not found"
  fi
else
  bad "cannot build consecutive_verify_gate harness"
fi

echo "---"
# LF-029 state-preserving teardown (composition defect caught by AUD-062):
# the runtime smoke must not destroy shared infrastructure it did not
# create. When the control plane is already running, LF-029 must leave it
# running afterwards so consecutive full verify passes can succeed.
if grep -q "was_up" scripts/live-fire/LF-029.sh && grep -q '\[ "\$was_up" != true \]' scripts/live-fire/LF-029.sh; then
  note "LF-029 is state-preserving (tears down only what it created)"
else
  bad "LF-029 tears down shared runtime unconditionally (composition defect)"
fi
if [ -f infra/compose/core.yaml ] && docker compose -f infra/compose/core.yaml ps -q control-plane 2>/dev/null | grep -q .; then
  before=$(docker compose -f infra/compose/core.yaml ps -q control-plane)
  if sh scripts/live-fire/LF-029.sh >/tmp/rx004-lf029.log 2>&1; then
    after=$(docker compose -f infra/compose/core.yaml ps -q control-plane 2>/dev/null)
    if [ -n "$after" ] && [ "$before" = "$after" ]; then
      note "LF-029 preserves a pre-existing running control plane"
    else
      bad "LF-029 removed a pre-existing running control plane"
    fi
  else
    bad "LF-029 failed against a pre-existing running control plane (see /tmp/rx004-lf029.log)"
  fi
else
  note "LF-029 state-preservation skipped (no running control plane)"
fi

echo "---"
echo "RX-004 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
