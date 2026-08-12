#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
profile="${1:-core}"
compose="infra/compose/$profile.yaml"
[ -f "$compose" ] || { echo "local start: FAIL - missing $compose" >&2; exit 1; }
docker compose -f "$compose" up -d --remove-orphans
count=0
while [ "$count" -lt 60 ]; do
  if sh scripts/smoke/runtime.sh >/dev/null 2>&1; then echo "local start $profile: ok"; exit 0; fi
  count=$((count + 1)); sleep 2
done
docker compose -f "$compose" logs --tail=100 >&2
echo "local start: FAIL - readiness timeout" >&2
exit 1
