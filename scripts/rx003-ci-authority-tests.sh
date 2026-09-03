#!/usr/bin/env sh
# RX-003 regression battery: GitHub/default-branch/CI authority.
# AUD-088: push CI must target the actual release branch (master), not main.
# AUD-089: the release workflow must run a real release-integrity surface.
# Trusted actions pinned to immutable commit SHAs (never mutable tags).
# Default branch protected with required checks; no direct unchecked pushes.
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

# --- AUD-088: push CI targets master (the actual default/release branch) ---
if grep -qE "branches: \[main\]" .github/workflows/ci.yml; then
  bad "ci.yml still pushes to main (AUD-088 not fixed)"
else
  note "ci.yml no longer targets main"
fi
if grep -qE "branches: \[master\]" .github/workflows/ci.yml; then
  note "ci.yml push CI targets master"
else
  bad "ci.yml push CI does not target master"
fi
# nightly must not target a phantom branch either
if grep -qE "branches: \[main\]" .github/workflows/nightly.yml; then
  bad "nightly.yml still references main"
else
  note "nightly.yml has no phantom main reference"
fi

# --- Trusted actions pinned to immutable SHAs (never mutable tags) ---
# A pinned use looks like: uses: owner/repo@40hexchars
# A mutable use looks like: uses: owner/repo@v1 or @main
mutable=$(grep -RhoE "uses: [A-Za-z0-9._-]+/[A-Za-z0-9._-]+@[^ #]+" .github/workflows/ | grep -vE "@[0-9a-f]{40}$" || true)
if [ -n "$mutable" ]; then
  bad "unpinned action refs found: $mutable"
else
  note "all trusted actions pinned to immutable commit SHAs"
fi

# --- AUD-089: release workflow is a real release-integrity surface ---
rel=.github/workflows/release.yml
for step in ep042-m1-tests.sh test-unit.sh test-integration.sh security-check.sh dependency-audit.sh license-gate.sh reality-gate.sh build.sh verify-remediation-register.sh; do
  if grep -q "$step" "$rel"; then
    note "release workflow runs $step"
  else
    bad "release workflow missing $step"
  fi
done

# --- Default branch protection (requires gh + repo-admin; verified live) ---
if command -v gh >/dev/null 2>&1; then
  prot=$(gh api repos/dominator509/nexus/branches/master/protection 2>/dev/null || true)
  if [ -z "$prot" ]; then
    bad "master branch protection not active (AUD-088 enforcement missing)"
  else
    for ctx in graphlock-v2 gate integration; do
      if echo "$prot" | grep -q "\"$ctx\""; then
        note "branch protection requires check $ctx"
      else
        bad "branch protection missing required check $ctx"
      fi
    done
    if echo "$prot" | grep -q '"enabled": *true'; then
      note "enforce_admins enabled (no admin bypass)"
    else
      bad "enforce_admins not enabled"
    fi
    if echo "$prot" | grep -q '"allow_force_pushes"'; then
      fp=$(echo "$prot" | grep -o '"allow_force_pushes": *{[^}]*}' | grep -o '"enabled": *[a-z]*' | grep -o '[a-z]*$')
      if [ "$fp" = "false" ]; then
        note "force pushes disabled"
      else
        bad "force pushes not disabled"
      fi
    fi
  fi
else
  bad "gh unavailable; cannot verify branch protection"
fi

echo "---"
echo "RX-003 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
