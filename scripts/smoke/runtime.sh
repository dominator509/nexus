#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
# Resolve the runtime base URL by precedence so the smoke gate works in any
# hosting situation (the repo is self-hosted-first and may be deployed on a
# dynamic IP / foreign host):
#   1. NEXUS_SMOKE_URL - operator override, always wins.
#   2. NEXUS_BASE_DOMAIN - used ONLY when it is a real deployable domain
#      (not a local/test placeholder). A hosted deployment behind a reverse
#      proxy on :443 resolves here (https://<domain>).
#   3. Canonical local mapping http://127.0.0.1:8443 - the compose core
#      profile binds the control-plane on 0.0.0.0:8443 and maps it to host
#      127.0.0.1:8443. nginx owns host :443 with an unrelated API in the
#      local dev profile, so the local fallback is HTTP on 8443, never
#      https://NEXUS_BASE_DOMAIN when the domain is only a placeholder.
if [ -n "${NEXUS_SMOKE_URL:-}" ]; then
  base="$NEXUS_SMOKE_URL"
elif [ -n "${NEXUS_BASE_DOMAIN:-}" ]; then
  case "$NEXUS_BASE_DOMAIN" in
    *.test|*.local|*.example.test|localhost) base="http://127.0.0.1:8443" ;;
    *) base="https://$NEXUS_BASE_DOMAIN" ;;
  esac
else
  base="http://127.0.0.1:8443"
fi
curl --fail --silent --show-error --max-time 10 "$base/healthz" | jq -e '.status == "healthy"' >/dev/null
curl --fail --silent --show-error --max-time 10 "$base/readyz" | jq -e '.ready == true' >/dev/null
curl --fail --silent --show-error --max-time 10 "$base/v1/capabilities" | jq -e '.capabilities | length > 0' >/dev/null
echo "runtime smoke: ok"
