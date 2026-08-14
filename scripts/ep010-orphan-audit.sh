#!/usr/bin/env sh
set -eu
# EP-010 orphan audit (COMMANDS.md): prove the EP-010 M5 composed
# subsystem proof left ZERO resources behind. EP-010 owns no
# containers or daemons - the audit guards the deterministic probe's
# temp footprint, stray processes, and the evidence directory.
. scripts/env.sh
export NO_COLOR=1
fail=0

# Defensive: no nexus-ep010-* containers/networks/volumes may survive
# (EP-010 itself never starts any, so any presence is an orphan).
DOCKER_BIN=$(command -v docker || true)
if [ -n "$DOCKER_BIN" ]; then
  leftovers=$("$DOCKER_BIN" ps -a --filter name=nexus-ep010 2>/dev/null | awk 'NR>1 {print $NF}' | sed '/^$/d')
  if [ -n "$leftovers" ]; then
    echo "EP-010 orphan audit: FAIL - leftover containers:" >&2
    echo "$leftovers" >&2
    fail=1
  fi
  leftover_nets=$("$DOCKER_BIN" network ls --filter name=nexus-ep010 2>/dev/null | awk 'NR>1 {print $2}' | sed '/^$/d')
  if [ -n "$leftover_nets" ]; then
    echo "EP-010 orphan audit: FAIL - leftover networks:" >&2
    echo "$leftover_nets" >&2
    fail=1
  fi
  leftover_vols=$("$DOCKER_BIN" volume ls --filter name=nexus-ep010 2>/dev/null | awk 'NR>1 {print $2}' | sed '/^$/d')
  if [ -n "$leftover_vols" ]; then
    echo "EP-010 orphan audit: FAIL - leftover volumes:" >&2
    echo "$leftover_vols" >&2
    fail=1
  fi
fi

# No probe scratch files in /tmp may survive.
tmp_leftovers=$(find /tmp -maxdepth 1 -name 'nexus-ep010-*' 2>/dev/null | sed '/^$/d')
if [ -n "$tmp_leftovers" ]; then
  echo "EP-010 orphan audit: FAIL - leftover temp files:" >&2
  echo "$tmp_leftovers" >&2
  fail=1
fi

# No stray livefire probe processes may survive. The bracket trick
# avoids matching this script's own grep.
procs=$(ps aux | grep -E '[l]ivefire_probe' | grep -v grep | sed '/^$/d')
if [ -n "$procs" ]; then
  echo "EP-010 orphan audit: FAIL - leftover probe processes:" >&2
  echo "$procs" >&2
  fail=1
fi

# The governed evidence directory must contain exactly the two
# canonical M5 evidence artifacts (JSON + markdown) and nothing else.
EVIDENCE_DIR=".agent/state/evidence/ep010-m5"
if [ -d "$EVIDENCE_DIR" ]; then
  extras=$(find "$EVIDENCE_DIR" -maxdepth 1 -type f ! -name 'ep010-m5-composed-proof.json' ! -name 'EP-010-M5-composed-proof.md' 2>/dev/null | sed '/^$/d')
  if [ -n "$extras" ]; then
    echo "EP-010 orphan audit: FAIL - unexpected evidence files:" >&2
    echo "$extras" >&2
    fail=1
  fi
fi

if [ "$fail" -ne 0 ]; then
  echo "EP-010 orphan audit: FAIL" >&2
  exit 1
fi
echo "EP-010 orphan audit: ok"
