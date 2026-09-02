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
# Read the registry from a dedicated fd (3), never stdin. Live-fire
# proofs are full subprocess batteries (docker exec, ffmpeg, tcpdump,
# CLI tools) that may read stdin; had the loop consumed the registry
# from fd 0, a child draining stdin would advance the shared file
# offset and the NEXT registry line would be read shifted (proof/owner
# columns misaligned), silently skipping or failing subsequent proofs
# (observed: LF-013 owner EP-027 read as shifted fields after the
# EP-025/LF-012 battery). fd 3 keeps the iteration immune to children.
while IFS='|' read -r proof owner script slug description <&3; do
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
done 3< "$registry"
if [ "${NEXUS_REQUIRE_ALL_PROOFS:-0}" = 1 ] && [ "$ran" -eq 0 ]; then
  echo "live-fire: FAIL - no proofs executed" >&2
  exit 1
fi
echo "live-fire: ok"
