#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
base="${NEXUS_SMOKE_URL:-https://${NEXUS_BASE_DOMAIN}}"
curl --fail --silent --show-error --max-time 10 "$base/healthz" | jq -e '.status == "healthy"' >/dev/null
curl --fail --silent --show-error --max-time 10 "$base/readyz" | jq -e '.ready == true' >/dev/null
curl --fail --silent --show-error --max-time 10 "$base/v1/capabilities" | jq -e '.capabilities | length > 0' >/dev/null
echo "runtime smoke: ok"
