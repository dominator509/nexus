#!/usr/bin/env sh
# EP-030 AdGuard Home operations diagnostic (M4; SPEC-007).
#
# Bounded actions only: probe the AdGuard Home control API health and
# report reachability truthfully. This diagnostic NEVER mutates DNS
# filtering, never blocks or unblocks domains, never changes config. It
# prints no credentials.
#
# Fail-closed semantics: the exit status is non-zero whenever the probe
# fails or authentication fails, so an unreachable AdGuard Home is
# reported as reachable=no with a non-zero code - never "healthy" from
# config existence alone.
set -eu

usage() {
  echo "usage: adguard-diag.sh <base-url> [username-env-name] [password-env-name]" >&2
  exit 2
}

base="${1:-}"
[ -n "$base" ] || usage

# Credentials are read from environment variable NAMES, never from argv
# and never printed. Defaults: ADGUARD_USERNAME / ADGUARD_PASSWORD.
user_env="${2:-ADGUARD_USERNAME}"
pass_env="${3:-ADGUARD_PASSWORD}"
user="$(eval "printf '%s' \"\${$user_env:-}\"" 2>/dev/null || true)"
pass="$(eval "printf '%s' \"\${$pass_env:-}\"" 2>/dev/null || true)"

echo "adguard-diag: probing $base"
curl_rc=0
if [ -n "$user" ] && [ -n "$pass" ]; then
  # The credentials go ONLY into the Basic auth header of the probe
  # process; they never appear in output.
  curl -sS -o /dev/null -w "%{http_code}" \
    -u "$user:$pass" \
    --max-time 10 \
    "$base/control/status" > /tmp/adguard-diag-code.$$ 2>/dev/null || curl_rc=$?
else
  curl -sS -o /dev/null -w "%{http_code}" \
    --max-time 10 \
    "$base/control/status" > /tmp/adguard-diag-code.$$ 2>/dev/null || curl_rc=$?
fi
code="$(cat /tmp/adguard-diag-code.$$ 2>/dev/null || true)"
rm -f /tmp/adguard-diag-code.$$

if [ "$curl_rc" -ne 0 ]; then
  echo "adguard-diag: reachable=no (curl rc=$curl_rc)"
  # Fail closed: any probe failure is a non-zero diagnostic.
  exit 3
fi

case "$code" in
  2*|3*)
    echo "adguard-diag: reachable=yes http=$code"
    exit 0
    ;;
  401|403)
    echo "adguard-diag: reachable=yes http=$code (authentication failed; healthy NEVER claimed)"
    exit 4
    ;;
  *)
    echo "adguard-diag: reachable=yes http=$code (unexpected status)"
    exit 5
    ;;
esac
