#!/usr/bin/env sh
# Regression test for the strengthened scope audit (ADR-005).
#
# Proves: a path introduced in an EARLIER milestone commit is detected even
# when the LAST commit is clean. The old audit only checked `HEAD~1`, so an
# out-of-fence path added in M1 and never touched again would slip through.
#
# Builds a disposable git repository with a fake EP-001 history:
#   base   (parent of first milestone)
#   M1     introduces allowed.txt AND rogue.txt (rogue.txt is out of fence)
#   M2     touches only allowed.txt  (clean last commit)
# The strengthened audit must fail on M2's state because rogue.txt entered in M1.
set -eu
export CI=true GIT_TERMINAL_PROMPT=0 GIT_PAGER=cat PAGER=cat DEBIAN_FRONTEND=noninteractive CARGO_TERM_COLOR=never
root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"

git init -q repo
cd repo
git config user.email test@example.com
git config user.name "Scope Audit Test"

mkdir -p .agent/expected-files .agent/state
cat > .agent/expected-files/EP-001.txt <<'EOF'
.agent/state/LEDGER.md
allowed.txt
EOF

echo "baseline" > .agent/state/LEDGER.md
git add -A
git commit -qm "base"

echo "m1" > allowed.txt
echo "rogue" > rogue.txt
git add -A
git commit -qm "[EP-001][M1] first milestone introduces rogue.txt"

echo "m2" > allowed.txt
git add -A
git commit -qm "[EP-001][M2] clean last commit"

# The last commit is clean, but the audit from baseline..HEAD must see rogue.txt.
if sh "$root/scripts/scope-audit.sh" EP-001 2>audit.err; then
  echo "scope audit regression: FAIL - rogue.txt was not detected" >&2
  exit 1
fi
if ! grep -q "rogue.txt" audit.err; then
  echo "scope audit regression: FAIL - rogue.txt missing from failure output" >&2
  cat audit.err >&2
  exit 1
fi
echo "scope audit regression: ok"
