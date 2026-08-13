#!/usr/bin/env sh
set -eu
# EP-007 one-shot Keycloak diagnostic (M4 ops requirement).
# Verifies the pinned 26.7.0 image, imports the Nexus realm into an
# ephemeral container, waits for the REAL OIDC discovery document, then
# removes the container. Bounded: start, prove, dispose.
. scripts/env.sh

IMAGE="quay.io/keycloak/keycloak:26.7.0"
DIGEST="sha256:0f198be292568439d700cdbfb893e69a6009bb43a94a06a945b1d3d506c76b13"
REALM="tests/auth/nexus-realm.json"
NAME="nexus-ep007-diag-$$"

cleanup() {
  /usr/bin/docker rm -f "$NAME" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

/usr/bin/docker image inspect "$IMAGE@$DIGEST" >/dev/null 2>&1 || {
  echo "ep007 diag: FAIL - pinned keycloak image not present locally"
  exit 1
}

# Dedicated ephemeral bootstrap admin; value never printed or persisted.
ADMIN_PW=$(head -c 24 /dev/urandom | base64 | tr -dc 'A-Za-z0-9' | head -c 18)
/usr/bin/docker run -d --name "$NAME" -p 127.0.0.1::8080 \
  -e KC_BOOTSTRAP_ADMIN_USERNAME=admin \
  -e KC_BOOTSTRAP_ADMIN_PASSWORD="$ADMIN_PW" \
  -v "$(pwd)/$REALM:/opt/keycloak/data/import/nexus-realm.json:ro" \
  "$IMAGE@$DIGEST" start-dev --import-realm >/dev/null

PORT=$(/usr/bin/docker port "$NAME" 8080 | cut -d: -f2 | tr -d ' \n')
i=0
while [ "$i" -lt 90 ]; do
  if curl -fsS --max-time 3 "http://127.0.0.1:$PORT/realms/nexus/.well-known/openid-configuration" 2>/dev/null | grep -q '"issuer"'; then
    echo "ep007 diag: ok - realm nexus discovery served on 127.0.0.1:$PORT"
    exit 0
  fi
  i=$((i + 1))
  sleep 1
done
echo "ep007 diag: FAIL - discovery not ready within 90s"
exit 1
