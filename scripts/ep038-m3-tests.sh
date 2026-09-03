#!/usr/bin/env sh
# EP-038 M3 gate: GlitchTip incident integration (SPEC-007 behavior 3;
# node contract) through a REAL ephemeral GlitchTip 6.1.8 fixture.
#
# M3 owns infra/glitchtip/ (nexus-glitchtip provider crate) and the
# integration proofs in tests/glitchtip/. The gate proves:
#   - real postgres + redis + glitchtip fixture with runtime creds
#   - provisioning of org/project/project-key/API token via Django shell
#   - production adapter unit suite (52 proofs)
#   - real-provider integration (4 proofs): envelope lands, readback
#     verified, escalation + dedupe grouping, redaction canary absent
#   - stopped-provider phase: fixture stopped, refused -> Unavailable
#   - no silent skips (exact pass counts asserted)
#   - orphan guard: zero owned containers/networks/files after teardown
set -eu
export CI=true
export CARGO_TERM_COLOR=never
umask 077

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

OWN="nexus-ep038m3"
NET="${OWN}-net"
PG_IMG="postgres:18.4"
REDIS_IMG="redis:7-alpine"
GT_IMG="glitchtip/glitchtip:6.1.8"
GT_PORT=18000
LOG="/tmp/${OWN}-gate.log"
PROV_OUT="/tmp/${OWN}-provision.out"
PROV_ERR="/tmp/${OWN}-provision.err"
ENV_FILE="/tmp/ep038-m3-env.sh"
TOKEN_FILE="/tmp/gt-token.txt"
: > "$LOG"

fail() {
  echo "EP-038 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-$LOG}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-038 M3 gate: $1"; }

# ------------------------------------------------------------- teardown
cleanup() {
  # Remove ONLY M3-owned resources: owned-prefix containers, anything
  # attached to the owned network, the owned network, owned temp files.
  for c in "${OWN}-postgres" "${OWN}-redis" "${OWN}-glitchtip"; do
    docker rm -f "$c" >/dev/null 2>&1 || true
  done
  for c in $(docker ps -aq --filter "network=$NET" 2>/dev/null || true); do
    docker rm -f "$c" >/dev/null 2>&1 || true
  done
  for c in $(docker ps -aq --filter "name=${OWN}-" 2>/dev/null || true); do
    docker rm -f "$c" >/dev/null 2>&1 || true
  done
  docker network rm "$NET" >/dev/null 2>&1 || true
  docker volume rm "${OWN}-pg-data" >/dev/null 2>&1 || true
  rm -f "$ENV_FILE" "$TOKEN_FILE" "$PROV_OUT" "$PROV_ERR" "$LOG" /tmp/ep038-gt-hdr-* 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# ------------------------------------------------------------- preflight
command -v docker >/dev/null 2>&1 || fail "docker missing"
command -v openssl >/dev/null 2>&1 || fail "openssl missing"
command -v curl >/dev/null 2>&1 || fail "curl missing"

# Remove M3 residue from prior manual proofs. Precise ownership: only
# containers attached to the owned network or carrying the owned prefix.
for c in $(docker ps -aq --filter "network=$NET" 2>/dev/null || true); do
  docker rm -f "$c" >/dev/null 2>&1 || true
done
for c in $(docker ps -aq --filter "name=${OWN}-" 2>/dev/null || true); do
  docker rm -f "$c" >/dev/null 2>&1 || true
done
docker network rm "$NET" >/dev/null 2>&1 || true
docker volume rm "${OWN}-pg-data" >/dev/null 2>&1 || true
rm -f "$ENV_FILE" "$TOKEN_FILE" /tmp/ep038-gt-hdr-* 2>/dev/null || true

# ------------------------------------------------------------- fixture up
PG_PW="$(openssl rand -hex 16)"
SECRET="$(openssl rand -hex 32)"

docker network create "$NET" >/dev/null

# postgres:18+ stores data under a versioned subdirectory; mount the
# parent /var/lib/postgresql (mounting .../data directly breaks the
# image with "Error: in 18+, these Docker images...").
docker run -d --name "${OWN}-postgres" --network "$NET" \
  -v "${OWN}-pg-data:/var/lib/postgresql" \
  -e POSTGRES_USER=glitchtip -e POSTGRES_PASSWORD="$PG_PW" -e POSTGRES_DB=glitchtip \
  "$PG_IMG" >/dev/null

pg_ready=0
for i in $(seq 1 30); do
  if docker exec "${OWN}-postgres" pg_isready -U glitchtip >/dev/null 2>&1; then
    pg_ready=1
    break
  fi
  sleep 1
done
[ "$pg_ready" -eq 1 ] || fail "postgres did not become ready"

# Redis must answer the baked-in settings hostname `redis` (django cache
# config); the owned container name stays prefixed, the alias carries
# the canonical service name.
docker run -d --name "${OWN}-redis" --network "$NET" --network-alias redis "$REDIS_IMG" >/dev/null

docker run -d --name "${OWN}-glitchtip" --network "$NET" \
  -p "127.0.0.1:${GT_PORT}:8000" \
  -e SERVER_ROLE=all_in_one -e GLITCHTIP_EMBED_WORKER=true \
  -e "DATABASE_URL=postgres://glitchtip:${PG_PW}@${OWN}-postgres:5432/glitchtip" \
  -e "SECRET_KEY=${SECRET}" -e EMAIL_URL=consolemail:// \
  -e "GLITCHTIP_DOMAIN=http://127.0.0.1:${GT_PORT}" -e PORT=8000 \
  "$GT_IMG" >/dev/null

# Bounded readiness: real HTTP response from the provider. Deadline
# derived from observed first-boot convergence (~2 min cold) with
# margin: 90 attempts x 2s = 180s.
ready=0
for i in $(seq 1 90); do
  code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 2 \
    "http://127.0.0.1:${GT_PORT}/api/0/" 2>/dev/null || true)"
  if [ -n "$code" ] && [ "$code" != "000" ]; then
    ready=1
    break
  fi
  sleep 2
done
[ "$ready" -eq 1 ] || fail "glitchtip did not become reachable"

# Idempotent migrations (bounded; image also migrates on boot).
timeout 120 docker exec "${OWN}-glitchtip" python manage.py migrate --noinput \
  >/dev/null 2>&1 || fail "glitchtip migrations failed"

# ------------------------------------------------------------- provision
# stdout carries KEYHEX/PROJECT_ID/TOKEN lines (mode 600 by umask);
# stderr is captured separately so a provisioning failure is diagnosed
# without ever printing the token.
docker exec -i "${OWN}-glitchtip" python - > "$PROV_OUT" 2> "$PROV_ERR" <<'PYEOF'
import django, os
os.environ.setdefault("DJANGO_SETTINGS_MODULE", "glitchtip.settings")
django.setup()
from apps.users.models import User
from apps.organizations_ext.models import Organization, OrganizationUser
from apps.projects.models import Project, ProjectKey
from apps.api_tokens.models import APIToken
user, created = User.objects.get_or_create(email="admin@nexus.local")
if created:
    user.set_password(os.urandom(12).hex())
    user.save()
org, _ = Organization.objects.get_or_create(slug="nexus-test-org", defaults={"name": "Nexus Test Org"})
OrganizationUser.objects.get_or_create(user=user, organization=org, defaults={"role": 3})
proj, _ = Project.objects.get_or_create(organization=org, slug="nexus-core", defaults={"name": "Nexus Core"})
key = ProjectKey.objects.create(project=proj, name="m3-key")
print("KEYHEX", key.public_key.hex)
print("PROJECT_ID", proj.id)
tok = APIToken.objects.create(user=user, label="m3-readback", scopes=1153)
print("TOKEN", tok.token)
PYEOF

keyhex="$(grep '^KEYHEX ' "$PROV_OUT" | tail -1 | awk '{print $2}')"
proj_id="$(grep '^PROJECT_ID ' "$PROV_OUT" | tail -1 | awk '{print $2}')"
tok="$(grep '^TOKEN ' "$PROV_OUT" | tail -1 | awk '{print $2}')"
if [ -z "$keyhex" ] || [ -z "$proj_id" ] || [ -z "$tok" ]; then
  tail -20 "$PROV_ERR" >&2 2>/dev/null || true
  fail "provisioning incomplete"
fi

DSN="http://${keyhex}@127.0.0.1:${GT_PORT}/${proj_id}"
ORG="nexus-test-org"
PROJECT="nexus-core"

umask 077
: > "$ENV_FILE"
cat >> "$ENV_FILE" <<EOF
export PG_PW='$PG_PW'
export SECRET='$SECRET'
export NEXUS_GLITCHTIP_DSN='$DSN'
export NEXUS_GLITCHTIP_ORG='$ORG'
export NEXUS_GLITCHTIP_PROJECT='$PROJECT'
export NEXUS_GLITCHTIP_TOKEN='$tok'
export NEXUS_GLITCHTIP_STOPPED_DSN=''
EOF
chmod 600 "$ENV_FILE"
printf '%s' "$tok" > "$TOKEN_FILE"
chmod 600 "$TOKEN_FILE"

export NEXUS_GLITCHTIP_DSN="$DSN"
export NEXUS_GLITCHTIP_ORG="$ORG"
export NEXUS_GLITCHTIP_PROJECT="$PROJECT"
export NEXUS_GLITCHTIP_TOKEN="$tok"

ok "fixture provisioned (org=$ORG project=$PROJECT)"

# ------------------------------------------------------------- unit
if ! sh -c 'cargo test -p nexus-glitchtip --lib >> "$1" 2>&1' _ "$LOG"; then
  fail "unit tests failed" "$LOG"
fi
grep -q "test result: ok. 52 passed; 0 failed" "$LOG" \
  || fail "unit vacuity guard (expect 52 passed)" "$LOG"
ok "unit suite 52/52"

# ------------------------------------------------------------- integration
if ! sh -c 'cargo test -p nexus-glitchtip-tests --test ep038_m3_integration -- --test-threads=1 >> "$1" 2>&1' _ "$LOG"; then
  fail "integration tests failed" "$LOG"
fi
grep -q "test result: ok. 4 passed; 0 failed" "$LOG" \
  || fail "integration vacuity guard (expect 4 passed)" "$LOG"
ok "integration suite 4/4"

# ------------------------------------------------------------- stopped
docker stop "${OWN}-glitchtip" >/dev/null
refused=0
for i in $(seq 1 15); do
  if ! curl -s -o /dev/null --max-time 1 "http://127.0.0.1:${GT_PORT}/api/0/" 2>/dev/null; then
    refused=1
    break
  fi
  sleep 1
done
[ "$refused" -eq 1 ] || fail "stopped provider still reachable"

export NEXUS_GLITCHTIP_STOPPED_DSN="$DSN"
if ! sh -c 'cargo test -p nexus-glitchtip-tests --test ep038_m3_stopped -- --exact ep038_integration_stopped_provider_unavailable >> "$1" 2>&1' _ "$LOG"; then
  fail "stopped-provider phase failed" "$LOG"
fi
grep -q "test result: ok. 1 passed; 0 failed" "$LOG" \
  || fail "stopped-phase vacuity guard (expect 1 passed)" "$LOG"
ok "stopped-provider phase 1/1 (refused -> Unavailable)"

# ------------------------------------------------------------- orphan guard
cleanup
if [ -n "$(docker ps -aq --filter "name=${OWN}-" 2>/dev/null || true)" ]; then
  fail "owned containers remain after teardown"
fi
if docker network inspect "$NET" >/dev/null 2>&1; then
  fail "owned network remains after teardown"
fi
if [ -f "$ENV_FILE" ] || [ -f "$TOKEN_FILE" ]; then
  fail "owned temp files remain after teardown"
fi
ok "orphan guard clean (zero owned containers/network/files)"

echo "EP-038 M3 gate: ok"
