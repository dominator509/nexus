#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
pat=.agent/reality-patterns
allow=.agent/reality-allow
[ -f "$pat" ] && [ -f "$allow" ] || { echo "reality gate: FAIL - missing pattern files" >&2; exit 1; }
hits=0
for dir in apps crates packages services python mobile infra; do
  [ -d "$dir" ] || continue
  out=$(grep -RInE -f "$pat" "$dir" 2>/dev/null | grep -vE -f "$allow" || true)
  if [ -n "$out" ]; then printf '%s
' "$out"; hits=1; fi
done
[ "$hits" -eq 0 ] || { echo "reality gate: FAIL" >&2; exit 1; }
echo "reality gate: ok"
