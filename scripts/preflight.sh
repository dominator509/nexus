#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
fail() { echo "preflight: FAIL - $1" >&2; exit 1; }
[ -f AGENTS.md ] && [ -d .agent ] || fail "run from repository root"
for f in AGENTS.md COMMANDS.md PREFLIGHT.md .env.example .agent/GRAPH.md .agent/LOOPS.md .agent/state/LEDGER.md .agent/reality-patterns .agent/reality-allow; do
  [ -f "$f" ] || fail "missing required file: $f"
done
python3 scripts/blueprint_validate.py >/dev/null || fail "blueprint structural validation failed"
sh scripts/check-shell.sh >/dev/null || fail "shell syntax validation failed"
sh scripts/toolchain-check.sh >/dev/null || fail "toolchain validation failed"
[ -f .env ] || fail "missing .env; copy .env.example and fill required values"
set -a
. ./.env
set +a
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
awk '/^PREFLIGHT-TABLE-BEGIN$/{t=1;next} /^PREFLIGHT-TABLE-END$/{t=0} t && NF' PREFLIGHT.md > "$tmp"
[ -s "$tmp" ] || fail "PREFLIGHT-TABLE missing or empty"
if command -v timeout >/dev/null 2>&1; then timeout_cmd="timeout 30"; else timeout_cmd=""; fi
while IFS='|' read -r var req probe; do
  var=$(printf '%s' "$var" | tr -d ' ')
  req=$(printf '%s' "$req" | tr -d ' ')
  probe=$(printf '%s' "$probe" | tr -d ' ')
  [ -n "$var" ] || continue
  eval "val=\${${var}:-}"
  if [ -z "$val" ]; then
    if [ "$req" = REQUIRED ]; then fail "env var not set: $var"; fi
    continue
  fi
  if [ "$probe" != "-" ]; then
    [ -f "$probe" ] || fail "missing probe script: $probe"
    if ! $timeout_cmd sh "$probe" >/dev/null 2>&1; then fail "credential probe failed: $var through $probe"; fi
  fi
done < "$tmp"
sh scripts/profile-preflight.sh >/dev/null || fail "selected profile requirements are not satisfied"
echo "preflight: ok"
