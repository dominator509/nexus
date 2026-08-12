#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
registry=live-fire/REGISTRY.tsv
[ -f "$registry" ] || { echo "live-fire: FAIL - missing registry" >&2; exit 1; }
ran=0
while IFS='|' read -r proof owner script slug description; do
  [ -n "$proof" ] || continue
  status=$(sh scripts/ledger.sh status "$owner")
  if [ "$status" = DONE ]; then
    [ -x "$script" ] || [ -f "$script" ] || { echo "live-fire: FAIL - missing $script" >&2; exit 1; }
    sh "$script"
    ran=$((ran + 1))
  elif [ "${NEXUS_REQUIRE_ALL_PROOFS:-0}" = 1 ]; then
    echo "live-fire: FAIL - $proof owner $owner is not DONE" >&2
    exit 1
  fi
done < "$registry"
if [ "${NEXUS_REQUIRE_ALL_PROOFS:-0}" = 1 ] && [ "$ran" -eq 0 ]; then
  echo "live-fire: FAIL - no proofs executed" >&2
  exit 1
fi
echo "live-fire: ok"
