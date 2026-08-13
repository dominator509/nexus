#!/usr/bin/env sh
set -eu
# EP-006 orphan audit (COMMANDS.md): prove the EP-006 integration suite
# left ZERO resources behind. Runs after the suite; fails the M3 gate if
# any nexus-ep006-* container, nexus-ep006-* network, registered stack
# volume, or temporal-server start process survives.
#
# Registered volumes come from the stack registry file written by
# tests/workflows/src/helpers/stack.ts (anonymous postgres volumes have
# no nexus-* name to filter by, so the registry is the authoritative
# list). Container/network/process checks are name- and process-based.
. scripts/env.sh
export NO_COLOR=1
DOCKER_BIN=$(command -v docker)
STATE=/tmp/nexus-ep006-stack-state.json
fail=0

leftovers=$("$DOCKER_BIN" ps -a --filter name=nexus-ep006 --format '{{.Names}}' | sed '/^$/d')
if [ -n "$leftovers" ]; then
  echo "EP-006 orphan audit: FAIL - leftover containers:" >&2
  echo "$leftovers" >&2
  fail=1
fi

leftover_nets=$("$DOCKER_BIN" network ls --filter name=nexus-ep006 --format '{{.Name}}' | sed '/^$/d')
if [ -n "$leftover_nets" ]; then
  echo "EP-006 orphan audit: FAIL - leftover networks:" >&2
  echo "$leftover_nets" >&2
  fail=1
fi

if [ -f "$STATE" ]; then
  volumes=$("$DOCKER_BIN" volume ls -q)
  registered=$(python3 -c "import json,sys; d=json.load(open('$STATE')); print('\n'.join(v for e in d.get('entries',[]) for v in e.get('volumes',[])))" 2>/dev/null || true)
  for volume in $registered; do
    case "$volumes" in
      *"$volume"*)
        echo "EP-006 orphan audit: FAIL - leftover volume: $volume" >&2
        fail=1
        ;;
    esac
  done
fi

if ps -eo args | grep 'temporal-server start' | grep -v grep | sed '/^$/d' | grep -q .; then
  echo "EP-006 orphan audit: FAIL - leftover temporal-server start process:" >&2
  ps -eo pid,args | grep 'temporal-server start' | grep -v grep >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "EP-006 orphan audit: FAIL" >&2
  exit 1
fi
echo "EP-006 orphan audit: ok"
