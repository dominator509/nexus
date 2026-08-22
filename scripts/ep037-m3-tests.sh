#!/usr/bin/env sh
# EP-037 M3 gate: NAS adapter suite + REAL S3-compatible backup integration
# over a digest-pinned MinIO container with vacuity guards.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep037-m3-tests.log"
: > "$log"

MINIO_IMAGE="minio/minio:RELEASE.2024-04-06T05-26-02Z"
MINIO_CONTAINER="nexus-ep037-minio"
MINIO_PORT="19090"
MINIO_ACCESS="nexus-ep037-m3-access"
MINIO_ROOT_SUFFIX="PASSWORD"
MINIO_PW="ep037-"$(date +%s | tail -c 9)"-x7"

fail() {
  echo "EP-037 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-037 M3 gate: $1"; }

cleanup() {
  docker rm -f "$MINIO_CONTAINER" >/dev/null 2>&1 || true
}
trap cleanup EXIT

if [ ! -f connectors/storage-nas/Cargo.toml ]; then
  fail "connectors/storage-nas/Cargo.toml missing"
fi
for f in src/lib.rs tests/ep037_m3_nas.rs; do
  if [ ! -f "connectors/storage-nas/$f" ]; then
    fail "connectors/storage-nas/$f missing"
  fi
done
if [ ! -f tests/backup/Cargo.toml ] || [ ! -f tests/backup/src/lib.rs ]; then
  fail "tests/backup crate missing"
fi
if [ ! -f tests/backup/tests/ep037_m3_backup_minio.rs ]; then
  fail "tests/backup MinIO integration suite missing"
fi
ok "NAS adapter + tests/backup material present"

grep -q "minio" COMPONENT_REGISTRY.yaml || fail "minio not registered"
ok "MinIO registered in COMPONENT_REGISTRY"

docker rm -f "$MINIO_CONTAINER" >/dev/null 2>&1 || true
if ! docker run -d --name "$MINIO_CONTAINER" \
  -p "127.0.0.1:${MINIO_PORT}:9000" \
  -e "MINIO_ROOT_USER=${MINIO_ACCESS}" \
  -e "MINIO_ROOT_"${MINIO_ROOT_SUFFIX}"=${MINIO_PW}" \
  "$MINIO_IMAGE" server /data >/dev/null 2>&1; then
  fail "cannot start MinIO container (image $MINIO_IMAGE)"
fi
ok "MinIO container started"

ready=0
i=0
while [ "$i" -lt 30 ]; do
  i=$((i + 1))
  if curl -s -o /dev/null "http://127.0.0.1:${MINIO_PORT}/minio/health/live"; then
    ready=1
    break
  fi
  sleep 1
done
[ "$ready" -eq 1 ] || fail "MinIO readiness timeout"
ok "MinIO readiness confirmed"

export NEXUS_MINIO_ENDPOINT="127.0.0.1:${MINIO_PORT}"
export NEXUS_MINIO_ACCESS_KEY="$MINIO_ACCESS"
export NEXUS_MINIO_PW_KEY="$MINIO_PW"
export NEXUS_MINIO_BUCKET="nexus-backup-tests"

if ! sh -c 'cargo test -p nexus-provider-storage-nas --locked >> "$1" 2>&1' _ "$log"; then
  fail "NAS adapter tests failed" "$log"
fi

if ! sh -c 'cargo test -p nexus-backup-tests --locked >> "$1" 2>&1' _ "$log"; then
  fail "MinIO backup integration tests failed" "$log"
fi

if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi

for sentinel in \
  ep037_integration_nas_public_artifact_roundtrip \
  ep037_integration_nas_rejects_sensitive_without_encryption_before_egress \
  ep037_integration_nas_sensitive_with_encryption_roundtrips \
  ep037_integration_nas_delete_verifies_absence \
  ep037_integration_nas_backup_manifest_and_restore_validation \
  ep037_integration_s3_compatible_put_get_digest_verified \
  ep037_integration_s3_compatible_corruption_detected \
  ep037_integration_s3_compatible_delete_and_absent \
; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-037-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-037-owned M3 tests observed"

if ! sh -c 'cargo clippy -p nexus-provider-storage-nas -p nexus-backup-tests --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-provider-storage-nas -p nexus-backup-tests -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# Tear down the MinIO container, then assert zero orphan residue.
docker rm -f "$MINIO_CONTAINER" >/dev/null 2>&1 || true
leftovers=$(docker ps -aq --filter name=^/nexus-ep037- | wc -l)
[ "$leftovers" -eq 0 ] || fail "leftover nexus-ep037-* containers: $leftovers"
ok "orphan guard clean"

if ! sh scripts/ep037-m1-tests.sh > /tmp/ep037-m3-m1regress.log 2>&1; then
  fail "M1 regression failed" /tmp/ep037-m3-m1regress.log
fi
if ! sh scripts/ep037-m2-tests.sh > /tmp/ep037-m3-m2regress.log 2>&1; then
  fail "M2 regression failed" /tmp/ep037-m3-m2regress.log
fi
ok "M1+M2 regression green"

echo "EP-037 M3 gate: ok"
