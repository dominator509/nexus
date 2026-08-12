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
changed=$(git diff --name-only HEAD~1 2>/dev/null || git diff --name-only)
for path in $changed; do
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
  [ "$ok" -eq 1 ] || { echo "scope audit: FAIL - $path is outside $node" >&2; exit 1; }
done
echo "scope audit $node: ok"
