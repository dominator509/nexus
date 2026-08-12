#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
node="${1:?node id}"
allowed=".agent/expected-files/$node.txt"
[ -f "$allowed" ] || { echo "scope audit: FAIL - missing $allowed" >&2; exit 1; }

# Find the earliest milestone commit for this node; its parent is the node baseline.
first_milestone=$(git log --reverse --format='%H' --grep="^\[$node\]\[M" | head -n 1)
if [ -n "$first_milestone" ]; then
  baseline=$(git rev-parse "$first_milestone^")
else
  baseline=$(git rev-parse HEAD~1 2>/dev/null || true)
fi

# Collect all changed paths: committed (baseline..HEAD), staged, unstaged, untracked.
changed=$( {
  if [ -n "$baseline" ]; then git diff --name-only "$baseline"..HEAD 2>/dev/null || true; fi
  git diff --cached --name-only
  git diff --name-only
  git ls-files --others --exclude-standard
} | sort -u )

fail=0
for path in $changed; do
  # L6 always-writable state (ledger + evidence) is governed, not fenced.
  case "$path" in
    .agent/state/LEDGER.md|.agent/state/evidence/*) continue ;;
  esac
  ok=0
  while IFS= read -r rule; do
    [ -n "$rule" ] || continue
    case "$rule" in \#*) continue;; esac
    case "$rule" in
      */) case "$path" in "$rule"*) ok=1;; esac ;;
      *) [ "$path" = "$rule" ] && ok=1 ;;
    esac
    [ "$ok" -eq 1 ] && break
  done < "$allowed"
  if [ "$ok" -ne 1 ]; then
    echo "scope audit: FAIL - $path is outside $node" >&2
    fail=1
  fi
done
[ "$fail" -eq 0 ] || exit 1
echo "scope audit $node: ok"
