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
