#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
node="${1:?node id}"
list=".agent/expected-files/$node.txt"
[ -f "$list" ] || { echo "expected files $node: FAIL - missing $list" >&2; exit 1; }
while IFS= read -r path; do
  [ -n "$path" ] || continue
  case "$path" in \#*) continue;; esac
  case "$path" in
    */) [ -d "${path%/}" ] || { echo "expected files $node: FAIL - missing directory $path" >&2; exit 1; } ;;
    *) [ -e "$path" ] || { echo "expected files $node: FAIL - missing $path" >&2; exit 1; } ;;
  esac
done < "$list"
echo "expected files $node: ok"
