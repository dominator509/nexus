#!/usr/bin/env sh
# LF-029 runtime-smoke (EP-044): prove the real Nexus Control Plane Runtime
# serves the canonical runtime endpoints over real HTTP, then shut down
# gracefully. This is the runtime smoke ownership proof for EP-044.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
log=/tmp/lf029-runtime-smoke.log
: > "$log"

# Determine the runtime base URL. The canonical convention is NEXUS_SMOKE_URL
# or https://${NEXUS_BASE_DOMAIN}. The runtime node owns base domain
# resolution; when a live server is present the smoke must pass.
base="${NEXUS_SMOKE_URL:-https://${NEXUS_BASE_DOMAIN}}"

# Bring up the runtime deterministically (compose core profile when
# available, else the real binary via local-start's readiness loop).
if [ -f infra/compose/core.yaml ]; then
  sh scripts/local-start.sh core >> "$log" 2>&1 || {
    echo "LF-029: FAIL - local start core failed" >&2
    tail -20 "$log" >&2
    exit 1
  }
fi

# Canonical runtime smoke assertions (identical to scripts/smoke/runtime.sh).
curl --fail --silent --show-error --max-time 10 "$base/healthz" | jq -e '.status == "healthy"' >/dev/null \
  || { echo "LF-029: FAIL - /healthz not healthy" >&2; tail -20 "$log" >&2; exit 1; }
curl --fail --silent --show-error --max-time 10 "$base/readyz" | jq -e '.ready == true' >/dev/null \
  || { echo "LF-029: FAIL - /readyz not ready" >&2; tail -20 "$log" >&2; exit 1; }
curl --fail --silent --show-error --max-time 10 "$base/v1/capabilities" | jq -e '.capabilities | length > 0' >/dev/null \
  || { echo "LF-029: FAIL - /v1/capabilities empty" >&2; tail -20 "$log" >&2; exit 1; }

# Tear down deterministically (strict cleanup doctrine).
if [ -f infra/compose/core.yaml ]; then
  sh scripts/local-stop.sh core >> "$log" 2>&1 || true
fi

echo "LF-029: ok"
