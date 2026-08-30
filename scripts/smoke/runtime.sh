#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
# Deterministic base URL for the local profile: the control-plane binds
# 0.0.0.0:8443 in the compose stack and is mapped to host 127.0.0.1:8443.
# NEXUS_SMOKE_URL wins when the operator supplies it; otherwise the
# canonical local mapping is used (nginx owns host :443 with an unrelated
# API, so https://nexus.test is NOT the local control-plane).
base="${NEXUS_SMOKE_URL:-http://127.0.0.1:8443}"
curl --fail --silent --show-error --max-time 10 "$base/healthz" | jq -e '.status == "healthy"' >/dev/null
curl --fail --silent --show-error --max-time 10 "$base/readyz" | jq -e '.ready == true' >/dev/null
curl --fail --silent --show-error --max-time 10 "$base/v1/capabilities" | jq -e '.capabilities | length > 0' >/dev/null
echo "runtime smoke: ok"
