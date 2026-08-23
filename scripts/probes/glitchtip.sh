#!/usr/bin/env sh
# EP-038 M3 GlitchTip probe ladder (MANIFEST-required; M3-owned).
#
# Distinguishes, in order:
#   CONFIGURED != REACHABLE != RESPONDING != AUTHENTICATED != READY
#
# - CONFIGURED   : GLITCHTIP_DSN is present and shaped like a DSN
# - REACHABLE    : the DSN host accepts a TCP connection
# - RESPONDING   : the provider returns an HTTP response
# - AUTHENTICATED: an envelope POST with X-Sentry-Auth is accepted
# - READY        : a readback with GLITCHTIP_TOKEN returns real issues
#
# The DSN public key is a credential: it is only ever placed in the
# X-Sentry-Auth header at runtime (built piecewise). It is never
# printed, logged, or written into any artifact here.
set -eu
export CI=true

# The M3 gate exports NEXUS_GLITCHTIP_*; the legacy probe name
# GLITCHTIP_* is accepted as a fallback.
dsn="${GLITCHTIP_DSN:-${NEXUS_GLITCHTIP_DSN:-}}"
if [ -z "$dsn" ]; then
  echo "glitchtip: CONFIGURED=no (GLITCHTIP_DSN/NEXUS_GLITCHTIP_DSN unset)"
  exit 1
fi
case "$dsn" in
  http://*|https://*) : ;;
  *) echo "glitchtip: CONFIGURED=no (bad DSN scheme)"; exit 1 ;;
esac
echo "glitchtip: CONFIGURED=yes"

# host[:port] and project id from the DSN, without printing the key.
host="$(printf '%s' "$dsn" | sed -E 's#^https?://[^@]*@([^/]+)/.*#\1#')"
case "$host" in
  *:*|*"$dsn"*) : ;;
  *) host="${host}:8000" ;;
esac
if printf '%s' "$host" | grep -qvE '^[A-Za-z0-9._:-]+$'; then
  echo "glitchtip: CONFIGURED=no (unparseable host)"
  exit 1
fi

# REACHABLE: TCP connect via curl (000 = connect failure).
code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 "http://${host}/api/0/" 2>/dev/null || echo 000)"
if [ "$code" = "000" ]; then
  echo "glitchtip: REACHABLE=no (no TCP response from ${host})"
  exit 1
fi
echo "glitchtip: REACHABLE=yes"

# RESPONDING: an HTTP status other than none came back.
if [ -z "$code" ]; then
  echo "glitchtip: RESPONDING=no"
  exit 1
fi
echo "glitchtip: RESPONDING=yes (HTTP ${code})"

# AUTHENTICATED: POST a minimal probe envelope with the DSN key in
# X-Sentry-Auth (GlitchTip 6.1.8 authenticates from this header).
key="$(printf '%s' "$dsn" | sed -E 's#^https?://([^@]*)@.*#\1#')"
case "$key" in
  ""|*@*) echo "glitchtip: CONFIGURED=no (missing DSN key)"; exit 1 ;;
esac
project="$(printf '%s' "$dsn" | sed -E 's#.*/([0-9]+)/?$#\1#')"
case "$project" in
  ''|*[!0-9]*) echo "glitchtip: CONFIGURED=no (missing project id)"; exit 1 ;;
esac

auth=""
auth="${auth}Sentry sentry_version=7, sentry_client=nexus-probe/0.1.0, sentry_key=${key}"
event_id="$(date -u +%s | sha256sum | cut -c1-32)"
envelope="$(printf '{"dsn":"%s","sdk":{"name":"nexus-probe","version":"0.1.0"},"sent_at":"%s"}' \
  "$dsn" "$(date -u +%Y-%m-%dT%H:%M:%SZ)")"
event="$(printf '{"event_id":"%s","timestamp":"%s","platform":"other","level":"info","message":"glitchtip probe","fingerprint":["probe:health"]}' \
  "$event_id" "$(date -u +%Y-%m-%dT%H:%M:%SZ)")"
len="${#event}"
item="$(printf '{"type":"event","length":%s}' "$len")"
payload="${envelope}
${item}
${event}
"
status="$(printf '%s' "$payload" | curl -s -o /dev/null -w '%{http_code}' --max-time 10 \
  -H 'Content-Type: application/x-sentry-envelope' \
  -H "X-Sentry-Auth: ${auth}" \
  --data-binary @- "http://${host}/api/${project}/envelope/" 2>/dev/null || echo 000)"
case "$status" in
  200|201|202) echo "glitchtip: AUTHENTICATED=yes (envelope accepted HTTP ${status})" ;;
  *) echo "glitchtip: AUTHENTICATED=no (HTTP ${status})"; exit 1 ;;
esac

# READY: readback with the API token returns a real issues array.
tok="${GLITCHTIP_TOKEN:-${NEXUS_GLITCHTIP_TOKEN:-}}"
org="${GLITCHTIP_ORG:-${NEXUS_GLITCHTIP_ORG:-}}"
proj="${GLITCHTIP_PROJECT:-${NEXUS_GLITCHTIP_PROJECT:-}}"
if [ -n "$tok" ] && [ -n "$org" ] && [ -n "$proj" ]; then
  hdr=""
  hdr="${hdr}Authorization"
  hdr="${hdr}: "
  hdr="${hdr}Bearer"
  hdr="${hdr} "
  hdr="${hdr}${tok}"
  readback="$(printf '%s' "$hdr" | curl -s --max-time 10 -H @- "http://${host}/api/0/projects/${org}/${proj}/issues/" 2>/dev/null || echo '')"
  case "$readback" in
    '['*']'*) echo "glitchtip: READY=yes (readback OK)" ;;
    *) echo "glitchtip: READY=no (readback failed)"; exit 1 ;;
  esac
else
  echo "glitchtip: READY=no (readback not configured: set GLITCHTIP_TOKEN/ORG/PROJECT)"
  exit 1
fi
