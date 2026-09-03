#!/usr/bin/env sh
# CI battery fixture provisioner: starts the real MinIO + SeaweedFS
# fixtures the blanket workspace battery requires (ep037_m3_backup_minio,
# ep037_m5_s3, lf020_storage_backend_portability fail closed when
# NEXUS_MINIO_* / NEXUS_S3_* env is absent). Mirrors the local closure
# ladder's /tmp/ep038-m5-battery.env exactly (see
# .agent/execplans + nexus-rx-closure-runtime-smoke-battery-rebind ref).
# Writes /tmp/battery.env with runtime-generated credentials; caller must
# source it with `set -a; . /tmp/battery.env; set +a`.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

MINIO_IMAGE="minio/minio:RELEASE.2024-04-06T05-26-02Z"
MINIO_CONTAINER="nexus-ci-minio"
MINIO_PORT="19090"
MINIO_ACCESS="nexus-ci-access"
MINIO_PW="ci-$(date +%s | tail -c 9)-x7"

SWF_IMAGE="chrislusf/seaweedfs:4.43@sha256:4d5118c198a6b9c0470c04b1822a0671459301625d995d1764c352bb553b3160"
SWF_CONTAINER="nexus-ci-seaweedfs"
SWF_S3_PORT="19333"
SWF_FILER_PORT="19888"
SWF_VOLUME_PORT="19080"
SWF_CFG_DIR="/tmp/nexus-ci-s3cfg"
SWF_ACCESS="nexus-ci-swf-access"
SWF_PW="ci-$(date +%s | tail -c 9)-swf-x7"

fail() { echo "ci battery: FAIL - $1" >&2; exit 1; }
ok() { echo "ci battery: $1"; }

docker rm -f "$MINIO_CONTAINER" >/dev/null 2>&1 || true
if ! docker run -d --name "$MINIO_CONTAINER" \
  -p "127.0.0.1:${MINIO_PORT}:9000" \
  -e "MINIO_ROOT_USER=${MINIO_ACCESS}" \
  -e "MINIO_ROOT_PASSWORD=${MINIO_PW}" \
  "$MINIO_IMAGE" server /data >/dev/null 2>&1; then
  fail "cannot start MinIO container"
fi
ok "MinIO container started"

ready=0; i=0
while [ "$i" -lt 30 ]; do
  i=$((i + 1))
  if curl -s -o /dev/null "http://127.0.0.1:${MINIO_PORT}/minio/health/live"; then
    ready=1; break
  fi
  sleep 1
done
[ "$ready" -eq 1 ] || fail "MinIO readiness timeout"
ok "MinIO readiness confirmed"

rm -rf "$SWF_CFG_DIR"
mkdir -p "$SWF_CFG_DIR"
printf '{"identities":[{"name":"nexus-ci","credentials":[{"accessKey":"%s","secretKey":"%s"}],"actions":["Read","Write","List","Tagging","Admin"]}]}' \
  "$SWF_ACCESS" "$SWF_PW" > "$SWF_CFG_DIR/s3.json"

docker rm -f "$SWF_CONTAINER" >/dev/null 2>&1 || true
if ! docker run -d --name "$SWF_CONTAINER" \
  -p "127.0.0.1:${SWF_S3_PORT}:8333" \
  -p "127.0.0.1:${SWF_FILER_PORT}:8888" \
  -p "127.0.0.1:${SWF_VOLUME_PORT}:8080" \
  -v "$SWF_CFG_DIR:/etc/seaweedfs:ro" \
  "$SWF_IMAGE" \
  server -master.port=9333 -volume.port=8080 -filer.port=8888 -s3.port=8333 \
  -filer -s3 \
  -s3.config=/etc/seaweedfs/s3.json -volume.max=256 -dir=/data >/dev/null 2>&1; then
  fail "cannot start SeaweedFS container"
fi
ok "SeaweedFS container started"

# Write the battery env file (mirrors /tmp/ep038-m5-battery.env keys).
cat > /tmp/battery.env <<EOF
export NEXUS_MINIO_ENDPOINT='127.0.0.1:${MINIO_PORT}'
export NEXUS_MINIO_ACCESS_KEY='${MINIO_ACCESS}'
export NEXUS_MINIO_PW_KEY='${MINIO_PW}'
export NEXUS_MINIO_BUCKET='nexus-backup-tests'
export NEXUS_S3_MINIO_ENDPOINT='127.0.0.1:${MINIO_PORT}'
export NEXUS_S3_MINIO_ACCESS_KEY='${MINIO_ACCESS}'
export NEXUS_S3_MINIO_PW_KEY='${MINIO_PW}'
export NEXUS_S3_MINIO_BUCKET_PREFIX='m5'
export NEXUS_S3_SEAWEEDFS_ENDPOINT='127.0.0.1:${SWF_S3_PORT}'
export NEXUS_S3_SEAWEEDFS_ACCESS_KEY='${SWF_ACCESS}'
export NEXUS_S3_SEAWEEDFS_PW_KEY='${SWF_PW}'
export NEXUS_S3_SEAWEEDFS_BUCKET_PREFIX='m5swf'
EOF
ok "battery env written to /tmp/battery.env"

# --- GlitchTip stack (ep038 integration needs a real provider) ---
GT_OWN="nexus-ci-gt"
GT_NET="${GT_OWN}-net"
GT_PG_IMG="postgres:18.4"
GT_REDIS_IMG="redis:7-alpine"
GT_IMG="glitchtip/glitchtip:6.1.8"
GT_PORT="18000"
GT_PG_PW="ci-gt-$(date +%s | tail -c 9)-x7"
GT_SECRET="ci-secret-$(date +%s | tail -c 9)"
GT_PROV_OUT="/tmp/${GT_OWN}-provision.out"
GT_PROV_ERR="/tmp/${GT_OWN}-provision.err"

docker network create "$GT_NET" >/dev/null 2>&1 || true
docker rm -f "${GT_OWN}-postgres" "${GT_OWN}-redis" "${GT_OWN}-glitchtip" >/dev/null 2>&1 || true

docker run -d --name "${GT_OWN}-postgres" --network "$GT_NET" \
  -e POSTGRES_USER=glitchtip -e POSTGRES_PASSWORD="***" -e POSTGRES_DB=glitchtip \
  "$GT_PG_IMG" >/dev/null 2>&1 || fail "cannot start glitchtip postgres"

pg_ready=0
for i in $(seq 1 30); do
  if docker exec "${GT_OWN}-postgres" pg_isready -U glitchtip >/dev/null 2>&1; then
    pg_ready=1; break
  fi
  sleep 1
done
[ "$pg_ready" -eq 1 ] || fail "glitchtip postgres not ready"
ok "glitchtip postgres ready"

docker run -d --name "${GT_OWN}-redis" --network "$GT_NET" --network-alias redis "$GT_REDIS_IMG" >/dev/null 2>&1 || fail "cannot start glitchtip redis"

docker run -d --name "${GT_OWN}-glitchtip" --network "$GT_NET" \
  -p "127.0.0.1:${GT_PORT}:8000" \
  -e SERVER_ROLE=all_in_one -e GLITCHTIP_EMBED_WORKER=true \
  -e "DATABASE_URL=postgres://glitchtip:***@${GT_OWN}-postgres:5432/glitchtip" \
  -e "SECRET_KEY=ci-secret-$(date +%s | tail -c 9)" -e EMAIL_URL=consolemail:// \
  -e "GLITCHTIP_DOMAIN=http://127.0.0.1:${GT_PORT}" -e PORT=8000 \
  "$GT_IMG" >/dev/null 2>&1 || fail "cannot start glitchtip"

gt_ready=0
for i in $(seq 1 90); do
  code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 2 "http://127.0.0.1:${GT_PORT}/api/0/" 2>/dev/null || true)"
  if [ -n "$code" ] && [ "$code" != "000" ]; then gt_ready=1; break; fi
  sleep 2
done
[ "$gt_ready" -eq 1 ] || fail "glitchtip not reachable"
ok "glitchtip reachable"

timeout 120 docker exec "${GT_OWN}-glitchtip" python manage.py migrate --noinput >/dev/null 2>&1 || fail "glitchtip migrations failed"

docker exec -i "${GT_OWN}-glitchtip" python - > "$GT_PROV_OUT" 2> "$GT_PROV_ERR" <<'PYEOF'
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
key = ProjectKey.objects.create(project=proj, name="ci-key")
print("KEYHEX", key.public_key.hex)
print("PROJECT_ID", proj.id)
tok = APIToken.objects.create(user=user, label="ci-readback", scopes=1153)
print("TOKEN", tok.token)
PYEOF

keyhex="$(grep '^KEYHEX ' "$GT_PROV_OUT" | tail -1 | awk '{print $2}')"
proj_id="$(grep '^PROJECT_ID ' "$GT_PROV_OUT" | tail -1 | awk '{print $2}')"
gt_tok="$(grep '^TOKEN ' "$GT_PROV_OUT" | tail -1 | awk '{print $2}')"
if [ -z "$keyhex" ] || [ -z "$proj_id" ] || [ -z "$gt_tok" ]; then
  tail -10 "$GT_PROV_ERR" >&2 2>/dev/null || true
  fail "glitchtip provisioning incomplete"
fi

GT_DSN="http://${keyhex}@127.0.0.1:${GT_PORT}/${proj_id}"
cat >> /tmp/battery.env <<EOF
export NEXUS_GLITCHTIP_DSN='${GT_DSN}'
export NEXUS_GLITCHTIP_ORG='nexus-test-org'
export NEXUS_GLITCHTIP_PROJECT='nexus-core'
export NEXUS_GLITCHTIP_TOKEN='${gt_tok}'
export NEXUS_GLITCHTIP_REVOKED=''
export NEXUS_GLITCHTIP_STOPPED_DSN='http://${keyhex}@127.0.0.1:1/${proj_id}'
EOF
ok "glitchtip provisioned and appended to battery env"
