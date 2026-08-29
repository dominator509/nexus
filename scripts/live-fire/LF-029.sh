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

# Deterministic base URL for the local profile: the control-plane binds
# 0.0.0.0:8443 in the compose stack and is mapped to host 127.0.0.1:8443.
# NEXUS_SMOKE_URL wins when the operator supplies it; otherwise the
# canonical local mapping is used (nginx owns host :443 with an unrelated
# API, so https://nexus.test is NOT the local control-plane).
export NEXUS_SMOKE_URL="${NEXUS_SMOKE_URL:-http://127.0.0.1:8443}"
base="$NEXUS_SMOKE_URL"

# Bring up the runtime deterministically (compose core profile when
# available, else the real binary via local-start's readiness loop).
# Capture prior runtime state: the smoke MUST NOT destroy shared
# infrastructure it did not create. When the control plane is already
# running (e.g. a canonical verify ladder that runs this proof multiple
# times back-to-back), leave it running afterwards; only tear down when
# this proof itself brought it up.
was_up=false
if [ -f infra/compose/core.yaml ]; then
  if docker compose -f infra/compose/core.yaml ps -q control-plane 2>/dev/null | grep -q .; then
    was_up=true
  fi
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

# Tear down deterministically ONLY when this proof created the runtime
# (strict cleanup doctrine, state-preserving). Shared infrastructure
# that was already running before this proof must remain running for
# subsequent ladder passes.
if [ -f infra/compose/core.yaml ] && [ "$was_up" != true ]; then
  sh scripts/local-stop.sh core >> "$log" 2>&1 || true
fi

echo "LF-029: ok"
