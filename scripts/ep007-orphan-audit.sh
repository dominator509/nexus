#!/usr/bin/env sh
set -eu
# EP-007 orphan audit: prove the live-fire run left ZERO EP-007 resources
# behind - no probe container, no nexus-ep007-* network, no temporary
# browser/Chrome profile processes, no callback helper, and no /tmp
# token/state files that held live credentials.
#
# The EP-006 precedent (scripts/ep006-orphan-audit.sh) checks containers,
# networks, and processes; EP-007 additionally checks the /tmp credential
# artifacts because the ceremony state file held a real access token
# (0600, deleted at teardown).
. scripts/env.sh
export NO_COLOR=1
DOCKER_BIN=$(command -v docker)
fail=0

leftovers=$("$DOCKER_BIN" ps -a --filter name=nexus-ep007 | awk 'NR>1 {print $NF}' | sed '/^$/d')
if [ -n "$leftovers" ]; then
  echo "EP-007 orphan audit: FAIL - leftover containers:" >&2
  echo "$leftovers" >&2
  fail=1
fi

leftover_nets=$("$DOCKER_BIN" network ls --filter name=nexus-ep007 | awk 'NR>1 {print $2}' | sed '/^$/d')
if [ -n "$leftover_nets" ]; then
  echo "EP-007 orphan audit: FAIL - leftover networks:" >&2
  echo "$leftover_nets" >&2
  fail=1
fi

# Browser/helper processes for the passkey ceremony.
if ps -eo args | grep -E 'ep007_(combined_ceremony|passkey|fresh_auth|debug_auth)' | grep -v grep | sed '/^$/d' | grep -q .; then
  echo "EP-007 orphan audit: FAIL - leftover ceremony/browser processes:" >&2
  ps -eo pid,args | grep -E 'ep007_(combined_ceremony|passkey|fresh_auth|debug_auth)' | grep -v grep >&2
  fail=1
fi

# /tmp credential/state artifacts (the only files that ever held live
# tokens; browser profiles were self-removed by the driver).
for f in /tmp/ep007_state.json /tmp/ep007_m5_tokens.json /tmp/ep007_fresh_tokens.json \
         /tmp/ep007_access.txt /tmp/ep007_admin_token.txt /tmp/ep007_owner_pw.txt \
         /tmp/ep007_jwks.json /tmp/ep007_m5_jwks.json; do
  if [ -e "$f" ]; then
    echo "EP-007 orphan audit: FAIL - leftover credential/state file: $f" >&2
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "EP-007 orphan audit: FAIL" >&2
  exit 1
fi
echo "EP-007 orphan audit: ok"
