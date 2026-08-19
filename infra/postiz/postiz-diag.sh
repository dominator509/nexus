#!/usr/bin/env sh
# EP-029 Postiz operations diagnostic (M4; SPEC-007).
#
# Bounded actions only: probe the Postiz public API health and report
# reachability truthfully. This diagnostic NEVER publishes, schedules,
# replies, spends, or mutates anything. It prints no credentials.
#
# Fail-closed semantics: the exit status is the CURL exit status (or 3
# when the probe could not complete), so an unreachable Postiz is
# reported as reachable=no with a non-zero code - never "healthy" from
# config existence alone.
set -eu

usage() {
  echo "usage: postiz-diag.sh <base-url> [api-key-env-name]" >&2
  exit 2
}

base="${1:-}"
[ -n "$base" ] || usage

# The API key is read from an environment variable NAME, never from
# argv and never printed. Default: POSTIZ_API_KEY.
key_env="${2:-POSTIZ_API_KEY}"
key="$(eval "printf '%s' \"\${$key_env:-}\"" 2>/dev/null || true)"

echo "postiz-diag: probing $base"
curl_rc=0
if [ -n "$key" ]; then
  # The credential goes ONLY into the Authorization header of the
  # probe process; it never appears in output.
  curl -sS -o /dev/null -w "%{http_code}" \
    -H "Authorization: $key" \
    --max-time 10 \
    "$base/integrations" > /tmp/postiz-diag-code.$$ 2>/dev/null || curl_rc=$?
else
  curl -sS -o /dev/null -w "%{http_code}" \
    --max-time 10 \
    "$base/integrations" > /tmp/postiz-diag-code.$$ 2>/dev/null || curl_rc=$?
fi
code="$(cat /tmp/postiz-diag-code.$$ 2>/dev/null || true)"
rm -f /tmp/postiz-diag-code.$$

if [ "$curl_rc" -ne 0 ]; then
  echo "postiz-diag: reachable=no (curl rc=$curl_rc)"
  # Fail closed: any probe failure is a non-zero diagnostic.
  exit 3
fi

case "$code" in
  2*|3*)
    echo "postiz-diag: reachable=yes http=$code"
    exit 0
    ;;
  401|403)
    echo "postiz-diag: reachable=yes http=$code (authentication failed; healthy NEVER claimed)"
    exit 4
    ;;
  *)
    echo "postiz-diag: reachable=yes http=$code (unexpected status)"
    exit 5
    ;;
esac
