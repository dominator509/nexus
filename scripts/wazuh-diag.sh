#!/usr/bin/env sh
# EP-031 Wazuh operations diagnostic and bounded recovery (M4).
#
# Fails closed: an unreachable provider reports reachable=no with a
# non-zero exit; configuration existence is NEVER claimed as health
# (configured != healthy). A single bounded recovery (one re-probe
# after a short delay) is attempted; there is no unbounded retry loop.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

base="${WAZUH_BASE_URL:-}"
user="${WAZUH_USER:-}"
pass="${WAZUH_PASS:-}"

if [ -z "$base" ]; then
  echo "wazuh-diag: FAIL - WAZUH_BASE_URL not set (configured != healthy)" >&2
  exit 3
fi

probe() {
  # Use the real authenticate endpoint; any HTTP response proves
  # reachability. Credentials are never echoed.
  if command -v curl >/dev/null 2>&1; then
    code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 \
      -u "$user:$pass" -X POST "$base/security/user/authenticate" 2>/dev/null || true)
  else
    code=$(wget -q -O /dev/null --timeout=5 --user "$user" --password "$pass" \
      --post-data '' "$base/security/user/authenticate" 2>/dev/null || echo 000)
  fi
  case "$code" in
    000|"" ) echo "no" ;;
    * ) echo "yes" ;;
  esac
}

reachable=$(probe)
if [ "$reachable" = "no" ]; then
  # Bounded recovery: one re-probe after a short delay, never a loop.
  sleep 2
  reachable=$(probe)
fi

if [ "$reachable" = "no" ]; then
  echo "wazuh-diag: FAIL - unreachable ($base)" >&2
  exit 3
fi

echo "wazuh-diag: reachable=yes ($base); health NOT asserted (configured != healthy)"
exit 0
