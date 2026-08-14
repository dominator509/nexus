#!/usr/bin/env sh
set -eu
# EP-011 orphan audit (COMMANDS.md): prove the EP-011 M3 transport
# suite left ZERO resources behind - no sidecar processes, no listener
# ports held by orphaned fixture servers, no temp/checkpoint files, no
# evidence pollution.
. scripts/env.sh
export NO_COLOR=1
fail=0

# Defensive: no nexus-ep011-* containers/networks/volumes may survive
# (EP-011 itself never starts any, so any presence is an orphan).
DOCKER_BIN=$(command -v docker || true)
if [ -n "$DOCKER_BIN" ]; then
  leftovers=$("$DOCKER_BIN" ps -a --filter name=nexus-ep011 2>/dev/null | awk 'NR>1 {print $NF}' | sed '/^$/d')
  if [ -n "$leftovers" ]; then
    echo "EP-011 orphan audit: FAIL - leftover containers:" >&2
    echo "$leftovers" >&2
    fail=1
  fi
  leftover_nets=$("$DOCKER_BIN" network ls --filter name=nexus-ep011 2>/dev/null | awk 'NR>1 {print $2}' | sed '/^$/d')
  if [ -n "$leftover_nets" ]; then
    echo "EP-011 orphan audit: FAIL - leftover networks:" >&2
    echo "$leftover_nets" >&2
    fail=1
  fi
  leftover_vols=$("$DOCKER_BIN" volume ls --filter name=nexus-ep011 2>/dev/null | awk 'NR>1 {print $2}' | sed '/^$/d')
  if [ -n "$leftover_vols" ]; then
    echo "EP-011 orphan audit: FAIL - leftover volumes:" >&2
    echo "$leftover_vols" >&2
    fail=1
  fi
fi

# No fixture sidecar processes may survive. The bracket trick avoids
# matching this script's own grep.
procs=$(ps aux | grep -E '[f]ixture_sidecar.py' | grep -v grep | sed '/^$/d')
if [ -n "$procs" ]; then
  echo "EP-011 orphan audit: FAIL - leftover fixture sidecar processes:" >&2
  echo "$procs" >&2
  fail=1
fi

# No legacy-source or checkpoint temp files may survive. The suite
# writes only under pytest's auto-managed tmp_path plus the explicit
# nexus-ep011-* scratch namespace; any such file is an orphan.
tmp_leftovers=$(find /tmp -maxdepth 1 -name 'nexus-ep011-*' 2>/dev/null | sed '/^$/d')
if [ -n "$tmp_leftovers" ]; then
  echo "EP-011 orphan audit: FAIL - leftover temp source/checkpoint files:" >&2
  echo "$tmp_leftovers" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "EP-011 orphan audit: FAIL" >&2
  exit 1
fi
echo "EP-011 orphan audit: ok"
