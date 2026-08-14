#!/usr/bin/env sh
set -eu
# EP-009 orphan audit (COMMANDS.md): prove the EP-009 M2 integration
# suite left ZERO resources behind. Runs after the suite; fails the M2
# gate if any nexus-ep009-* container, network, or volume survives, or
# if any SOPS plaintext/age-identity temp files remain.
. scripts/env.sh
export NO_COLOR=1
DOCKER_BIN=$(command -v docker)
fail=0

leftovers=$("$DOCKER_BIN" ps -a --filter name=nexus-ep009 | awk 'NR>1 {print $NF}' | sed '/^$/d')
if [ -n "$leftovers" ]; then
  echo "EP-009 orphan audit: FAIL - leftover containers:" >&2
  echo "$leftovers" >&2
  fail=1
fi

leftover_nets=$("$DOCKER_BIN" network ls --filter name=nexus-ep009 | awk 'NR>1 {print $2}' | sed '/^$/d')
if [ -n "$leftover_nets" ]; then
  echo "EP-009 orphan audit: FAIL - leftover networks:" >&2
  echo "$leftover_nets" >&2
  fail=1
fi

leftover_vols=$("$DOCKER_BIN" volume ls --filter name=nexus-ep009 | awk 'NR>1 {print $2}' | sed '/^$/d')
if [ -n "$leftover_vols" ]; then
  echo "EP-009 orphan audit: FAIL - leftover volumes:" >&2
  echo "$leftover_vols" >&2
  fail=1
fi

# SOPS temp identity/plaintext files (directive Q): zero may survive.
sops_leftovers=$(find /tmp -maxdepth 1 -name 'nexus-sops-age-*' 2>/dev/null | sed '/^$/d')
if [ -n "$sops_leftovers" ]; then
  echo "EP-009 orphan audit: FAIL - leftover SOPS temp files:" >&2
  echo "$sops_leftovers" >&2
  fail=1
fi

# age private-key temp files (directive Q): zero may survive.
age_leftovers=$(find /tmp -maxdepth 1 -name 'nexus-ep009-*.key' 2>/dev/null | sed '/^$/d')
if [ -n "$age_leftovers" ]; then
  echo "EP-009 orphan audit: FAIL - leftover age identity files:" >&2
  echo "$age_leftovers" >&2
  fail=1
fi

# headscale temp dirs (TLS certs, sqlite data, configs): zero may survive.
hs_leftovers=$(find /tmp -maxdepth 1 -name 'nexus-ep009-hs-*' 2>/dev/null | sed '/^$/d')
if [ -n "$hs_leftovers" ]; then
  echo "EP-009 orphan audit: FAIL - leftover headscale temp files:" >&2
  echo "$hs_leftovers" >&2
  fail=1
fi

# PKI temp files (EP-009 M4): CA certs, leaf keys, CSRs, token files,
# and the pki live-fire scratch dir must ALL be gone.
pki_leftovers=$(find /tmp -maxdepth 1 \( -name 'nexus-pki-*' -o -name 'm4*.csr' -o -name 'm4*.key' -o -name 'm4ca*.pem' \) 2>/dev/null | sed '/^$/d')
if [ -n "$pki_leftovers" ]; then
  echo "EP-009 orphan audit: FAIL - leftover PKI temp files:" >&2
  echo "$pki_leftovers" >&2
  fail=1
fi

# helper processes (directive Q): zero may survive. The bracket trick
# avoids matching this script's own grep; the pattern targets actual
# daemon/helper binaries, not shell wrappers whose command line merely
# mentions the node name.
procs=$(ps aux | grep -E '[o]penbao server -dev|[b]ao server -dev|[s]ops --decrypt|[a]ge-keygen|[h]eadscale serve|[p]ki_live_proof|[p]ki_failure_probe' | grep -v grep | sed '/^$/d')
if [ -n "$procs" ]; then
  echo "EP-009 orphan audit: FAIL - leftover helper processes:" >&2
  echo "$procs" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "EP-009 orphan audit: FAIL" >&2
  exit 1
fi
echo "EP-009 orphan audit: ok"
