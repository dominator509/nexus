#!/usr/bin/env sh
set -eu
# EP-008 orphan audit (COMMANDS.md): prove the EP-008 M3 integration
# suite left ZERO resources behind. Runs after the suite; fails the M3
# gate if any nexus-ep008-* container, network, or volume survives.
. scripts/env.sh
export NO_COLOR=1
DOCKER_BIN=$(command -v docker)
fail=0

leftovers=$("$DOCKER_BIN" ps -a --filter name=nexus-ep008 | awk 'NR>1 {print $NF}' | sed '/^$/d')
if [ -n "$leftovers" ]; then
  echo "EP-008 orphan audit: FAIL - leftover containers:" >&2
  echo "$leftovers" >&2
  fail=1
fi

leftover_nets=$("$DOCKER_BIN" network ls --filter name=nexus-ep008 | awk 'NR>1 {print $2}' | sed '/^$/d')
if [ -n "$leftover_nets" ]; then
  echo "EP-008 orphan audit: FAIL - leftover networks:" >&2
  echo "$leftover_nets" >&2
  fail=1
fi

leftover_vols=$("$DOCKER_BIN" volume ls --filter name=nexus-ep008 | awk 'NR>1 {print $2}' | sed '/^$/d')
if [ -n "$leftover_vols" ]; then
  echo "EP-008 orphan audit: FAIL - leftover volumes:" >&2
  echo "$leftover_vols" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "EP-008 orphan audit: FAIL" >&2
  exit 1
fi
echo "EP-008 orphan audit: ok"
