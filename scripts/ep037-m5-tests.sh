#!/usr/bin/env sh
# EP-037 M5 gate: S3-compatible adapter integration + LF-002/LF-020
# live-fire journeys + closure proofs (SPEC-024).
#
# RUNS the REAL storage-s3 adapter suite against REAL digest-pinned
# MinIO + SeaweedFS S3-gateway containers, the REAL LF-020
# storage-backend-portability journey (local -> MinIO with
# approval-before-source-delete), the REAL LF-002 restore-existing-nexus
# journey (encrypted state -> fresh deployment -> five domains
# reattach), with vacuity guards, anti-masking sentinels, current-run
# evidence freshness, redaction scans, M1-M4 regressions, expected-files
# EP-037, clippy/fmt, and orphan/resource hygiene.
#
# The LF scripts (LF-002.sh / LF-020.sh) invoke this gate; they are NOT
# substitutes for the journeys - this gate executes the journeys.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep037-m5-tests.log"
: > "$log"

MINIO_IMAGE="minio/minio:RELEASE.2024-04-06T05-26-02Z"
MINIO_CONTAINER="nexus-ep037-m5-minio"
MINIO_PORT="19090"
MINIO_ACCESS="nexus-ep037-m5-access"
MINIO_ROOT_SUFFIX="PASSWORD"
MINIO_PW="ep037m5-"$(date +%s | tail -c 9)"-x7"
MINIO_BUCKET_PREFIX="m5"

SWF_IMAGE="chrislusf/seaweedfs:4.43@sha256:4d5118c198a6b9c0470c04b1822a0671459301625d995d1764c352bb553b3160"
SWF_CONTAINER="nexus-ep037-m5-seaweedfs"
SWF_S3_PORT="19333"
SWF_FILER_PORT="19888"
SWF_VOLUME_PORT="19080"
SWF_CFG_DIR="/tmp/nexus-ep037-m5-s3cfg"
SWF_ACCESS="nexus-ep037-m5-swf-access"
SWF_KEY_NAME="secretKey"
SWF_PW="ep037m5-"$(date +%s | tail -c 9)"-swf-x7"
SWF_BUCKET_PREFIX="m5swf"

fail() {
  echo "EP-037 M5 gate: FAIL - $1" >&2
  tail -60 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-037 M5 gate: $1"; }

owned_containers() {
  docker ps -aq --filter name=^/nexus-ep037-m5- 2>/dev/null | wc -l
}
owned_volumes() {
  docker volume ls -q --filter name=^nexus-ep037-m5- 2>/dev/null | wc -l
}
owned_networks() {
  docker network ls -q --filter name=^nexus-ep037-m5- 2>/dev/null | wc -l
}
pressure() {
  echo "pressure: disk_free=$(df -P / | tail -1 | awk '{print $4}') owned_containers=$(owned_containers) owned_volumes=$(owned_volumes) owned_networks=$(owned_networks)"
}

cleanup() {
  # Exact EP-037 M5 ownership only. Never a broad prune.
  docker rm -f "$MINIO_CONTAINER" >/dev/null 2>&1 || true
  docker rm -f "$SWF_CONTAINER" >/dev/null 2>&1 || true
  for c in $(docker ps -aq --filter name=^/nexus-ep037-m5- 2>/dev/null); do
    docker rm -f "$c" >/dev/null 2>&1 || true
  done
  rm -rf "$SWF_CFG_DIR"
  rm -rf /tmp/nexus-ep037-m5-*-cfg
}
trap cleanup EXIT

# --- material guards (non-vacuous) ---
if [ ! -f connectors/storage-s3/Cargo.toml ]; then
  fail "connectors/storage-s3/Cargo.toml missing"
fi
for f in src/lib.rs src/transport.rs src/bin/s3-diag.rs tests/ep037_m5_s3.rs; do
  if [ ! -f "connectors/storage-s3/$f" ]; then
    fail "connectors/storage-s3/$f missing"
  fi
done
ok "storage-s3 adapter material present"

if [ ! -f tests/livefire/storage/Cargo.toml ] || [ ! -f tests/livefire/storage/src/lib.rs ]; then
  fail "tests/livefire/storage crate missing"
fi
for f in tests/lf002_restore_existing_nexus.rs tests/lf020_storage_backend_portability.rs; do
  if [ ! -f "tests/livefire/storage/$f" ]; then
    fail "tests/livefire/storage/$f missing"
  fi
done
ok "live-fire journeys material present"

# Anti-phantom: LF scripts must never call proof-runner / nexus-cli.
for lf in scripts/live-fire/LF-002.sh scripts/live-fire/LF-020.sh; do
  if grep -q "proof-runner\|nexus-cli" "$lf"; then
    fail "$lf still references the phantom proof-runner/nexus-cli"
  fi
done
ok "no proof-runner/nexus-cli references in LF scripts"

grep -q "minio" COMPONENT_REGISTRY.yaml || fail "minio not registered"
grep -q "seaweedfs" COMPONENT_REGISTRY.yaml || fail "seaweedfs not registered"
ok "providers registered in COMPONENT_REGISTRY"

# --- MinIO runtime with runtime-generated credentials ---
docker rm -f "$MINIO_CONTAINER" >/dev/null 2>&1 || true
if ! docker run -d --name "$MINIO_CONTAINER" \
  -p "127.0.0.1:${MINIO_PORT}:9000" \
  -e "MINIO_ROOT_USER=${MINIO_ACCESS}" \
  -e "MINIO_ROOT_PASSWORD=${MINIO_PW}" \
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

# --- SeaweedFS runtime with runtime-generated credentials ---
rm -rf "$SWF_CFG_DIR"
mkdir -p "$SWF_CFG_DIR"
printf '{"identities":[{"name":"nexus-m5","credentials":[{"accessKey":"%s","%s":"%s"}],"actions":["Read","Write","List","Tagging","Admin"]}]}' \
  "$SWF_ACCESS" "$SWF_KEY_NAME" "$SWF_PW" > "$SWF_CFG_DIR/s3.json"

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
  fail "cannot start SeaweedFS container (image $SWF_IMAGE)"
fi
ok "SeaweedFS container started"

pressure >> "$log"
ok "docker pressure recorded (pre-run)"

# --- probe-based readiness for BOTH real providers via the production
# adapter (requirement H/I/J: healthz alone is never readiness) ---
if ! cargo build -p nexus-provider-storage-s3 --locked --bin s3-diag >> "$log" 2>&1; then
  fail "cannot build s3-diag for probe readiness" "$log"
fi

ready=0
i=0
last_diag=""
while [ "$i" -lt 90 ]; do
  i=$((i + 1))
  if env NEXUS_S3_ENDPOINT="127.0.0.1:${MINIO_PORT}" \
    NEXUS_S3_ACCESS_KEY="$MINIO_ACCESS" \
    NEXUS_S3_PW_KEY="$MINIO_PW" \
    NEXUS_S3_BUCKET_PREFIX="$MINIO_BUCKET_PREFIX" \
    NEXUS_S3_PROFILE="MINIO" \
    ./target/debug/s3-diag status > /tmp/ep037-m5-diag-minio.log 2>&1; then
    if grep -q "probe_verified: true" /tmp/ep037-m5-diag-minio.log; then
      ready=1
      break
    fi
  fi
  last_diag=$(tail -1 /tmp/ep037-m5-diag-minio.log 2>/dev/null)
  sleep 2
done
[ "$ready" -eq 1 ] || fail "MinIO production probe readiness timeout (last: $last_diag)" /tmp/ep037-m5-diag-minio.log
ok "MinIO production probe verified (initial readiness)"

ready=0
i=0
last_diag=""
while [ "$i" -lt 90 ]; do
  i=$((i + 1))
  if env NEXUS_S3_ENDPOINT="127.0.0.1:${SWF_S3_PORT}" \
    NEXUS_S3_ACCESS_KEY="$SWF_ACCESS" \
    NEXUS_S3_PW_KEY="$SWF_PW" \
    NEXUS_S3_BUCKET_PREFIX="$SWF_BUCKET_PREFIX" \
    NEXUS_S3_PROFILE="SEAWEEDFS" \
    ./target/debug/s3-diag status > /tmp/ep037-m5-diag-swf.log 2>&1; then
    if grep -q "probe_verified: true" /tmp/ep037-m5-diag-swf.log; then
      ready=1
      break
    fi
  fi
  last_diag=$(tail -1 /tmp/ep037-m5-diag-swf.log 2>/dev/null)
  sleep 2
done
[ "$ready" -eq 1 ] || fail "SeaweedFS production probe readiness timeout (last: $last_diag)" /tmp/ep037-m5-diag-swf.log
ok "SeaweedFS production probe verified (initial readiness)"

export NEXUS_S3_MINIO_ENDPOINT="127.0.0.1:${MINIO_PORT}"
export NEXUS_S3_MINIO_ACCESS_KEY="$MINIO_ACCESS"
export NEXUS_S3_MINIO_PW_KEY="$MINIO_PW"
export NEXUS_S3_MINIO_BUCKET_PREFIX="$MINIO_BUCKET_PREFIX"
export NEXUS_S3_SEAWEEDFS_ENDPOINT="127.0.0.1:${SWF_S3_PORT}"
export NEXUS_S3_SEAWEEDFS_ACCESS_KEY="$SWF_ACCESS"
export NEXUS_S3_SEAWEEDFS_PW_KEY="$SWF_PW"
export NEXUS_S3_SEAWEEDFS_BUCKET_PREFIX="$SWF_BUCKET_PREFIX"

# --- storage-s3 adapter suite (both real providers) ---
if ! sh -c 'cargo test -p nexus-provider-storage-s3 --locked --test ep037_m5_s3 -- --test-threads=1 --nocapture >> "$1" 2>&1' _ "$log"; then
  fail "storage-s3 M5 suite failed" "$log"
fi
if ! sh -c 'cargo test -p nexus-provider-storage-s3 --locked --lib >> "$1" 2>&1' _ "$log"; then
  fail "storage-s3 transport unit tests failed" "$log"
fi
ok "storage-s3 adapter suite green"

# --- LF-020 storage-backend-portability journey ---
if ! sh -c 'cargo test -p nexus-storage-livefire --locked --test lf020_storage_backend_portability -- --nocapture >> "$1" 2>&1' _ "$log"; then
  fail "LF-020 journey failed" "$log"
fi
ok "LF-020 storage-backend-portability journey green"

# --- LF-002 restore-existing-nexus journey ---
if ! sh -c 'cargo test -p nexus-storage-livefire --locked --test lf002_restore_existing_nexus -- --nocapture >> "$1" 2>&1' _ "$log"; then
  fail "LF-002 journey failed" "$log"
fi
ok "LF-002 restore-existing-nexus journey green"

# --- vacuity + anti-masking ---
if ! grep -qE 'test result: ok\. [0-9]+ passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi
for sentinel in \
  ep037_m5_s3_positive_roundtrip_hash_verified \
  ep037_m5_s3_rejects_sensitive_without_encryption_before_egress \
  ep037_m5_s3_delete_absent_verified_ladder \
  ep037_m5_s3_shared_content_delete_preserves_object \
  ep037_m5_s3_backup_restore_hash_gates \
  ep037_m5_s3_migration_verifies_destination_and_failure_preserves_source \
  ep037_m5_s3_list_pagination \
  ep037_m5_s3_set_retention_updates_metadata \
  ep037_m5_s3_transport_timeout_silent_peer \
  ep037_m5_s3_transport_malformed_response_external \
  ep037_m5_s3_transport_refused_unavailable_not_found_distinct \
  ep037_m5_s3_redaction_canary_zero_leakage \
  lf020_storage_backend_portability_journey \
  lf002_restore_existing_nexus_journey \
; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-037-owned proof $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-037-owned M5 proofs observed"

# --- current-run evidence freshness (stale evidence never satisfies) ---
evid_dir=".agent/state/evidence"
for evid in LF-002-ep037-m5.json LF-020-ep037-m5.json; do
  f="$evid_dir/$evid"
  if [ ! -f "$f" ]; then
    fail "evidence $f missing"
  fi
  age=$(( $(date +%s) - $(stat -c %Y "$f") ))
  if [ "$age" -gt 600 ]; then
    fail "evidence $f is stale (age ${age}s > 600s)"
  fi
  if ! grep -q '"node": "EP-037"' "$f"; then
    fail "evidence $f not node-bound"
  fi
  if ! grep -q '"milestone": "M5"' "$f"; then
    fail "evidence $f not milestone-bound"
  fi
  if ! grep -q '"run_id"' "$f"; then
    fail "evidence $f missing run_id"
  fi
  if grep -qi "secret_key\|access_key\|password\|aws4-hmac\|signature=" "$f"; then
    fail "evidence $f leaks credential-shaped content"
  fi
done
ok "current-run evidence fresh + redacted"

# --- diag negative: unreachable exits nonzero truthfully ---
if ! docker stop "$MINIO_CONTAINER" >/dev/null 2>&1; then
  fail "cannot stop MinIO for diag negative check"
fi
if env NEXUS_S3_ENDPOINT="127.0.0.1:${MINIO_PORT}" \
  NEXUS_S3_ACCESS_KEY="$MINIO_ACCESS" \
  NEXUS_S3_PW_KEY="$MINIO_PW" \
  NEXUS_S3_BUCKET_PREFIX="$MINIO_BUCKET_PREFIX" \
  NEXUS_S3_PROFILE="MINIO" \
  ./target/debug/s3-diag status > /tmp/ep037-m5-diag-neg.log 2>&1; then
  fail "s3-diag must exit nonzero when provider unreachable" /tmp/ep037-m5-diag-neg.log
fi
grep -q "state: DEGRADED" /tmp/ep037-m5-diag-neg.log || fail "s3-diag must report DEGRADED truthfully" /tmp/ep037-m5-diag-neg.log
ok "s3-diag unreachable exits nonzero with truthful status"

# --- clippy + fmt ---
if ! sh -c 'cargo clippy -p nexus-provider-storage-s3 -p nexus-storage-livefire --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"
if ! sh -c 'cargo fmt -p nexus-provider-storage-s3 -p nexus-storage-livefire -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# --- teardown then orphan guard ---
docker rm -f "$MINIO_CONTAINER" >/dev/null 2>&1 || true
docker rm -f "$SWF_CONTAINER" >/dev/null 2>&1 || true
for c in $(docker ps -aq --filter name=^/nexus-ep037-m5- 2>/dev/null); do
  docker rm -f "$c" >/dev/null 2>&1 || true
done
rm -rf "$SWF_CFG_DIR"
rm -rf /tmp/nexus-ep037-m5-*-cfg
leftovers=$(docker ps -aq --filter name=^/nexus-ep037- | wc -l)
[ "$leftovers" -eq 0 ] || fail "leftover nexus-ep037-* containers: $leftovers"
ok "orphan guard clean"
pressure >> "$log"
ok "docker pressure recorded (post-run)"

# --- expected-files EP-037 (M5 owns connectors/storage-s3/) ---
if ! sh scripts/expected-files.sh EP-037 > /tmp/ep037-m5-expected.log 2>&1; then
  fail "expected-files EP-037 failed" /tmp/ep037-m5-expected.log
fi
ok "expected-files EP-037 green"

# --- M1-M4 regressions ---
if ! sh scripts/ep037-m1-tests.sh > /tmp/ep037-m5-m1regress.log 2>&1; then
  fail "M1 regression failed" /tmp/ep037-m5-m1regress.log
fi
if ! sh scripts/ep037-m2-tests.sh > /tmp/ep037-m5-m2regress.log 2>&1; then
  fail "M2 regression failed" /tmp/ep037-m5-m2regress.log
fi
if ! sh scripts/ep037-m3-tests.sh > /tmp/ep037-m5-m3regress.log 2>&1; then
  fail "M3 regression failed" /tmp/ep037-m5-m3regress.log
fi
if ! sh scripts/ep037-m4-tests.sh > /tmp/ep037-m5-m4regress.log 2>&1; then
  fail "M4 regression failed" /tmp/ep037-m5-m4regress.log
fi
ok "M1+M2+M3+M4 regression green"

echo "EP-037 M5 gate: ok"
