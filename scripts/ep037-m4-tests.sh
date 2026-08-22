#!/usr/bin/env sh
# EP-037 M4 gate: REAL SeaweedFS S3-gateway adapter + forced-failure
# suite over a digest-pinned SeaweedFS container with vacuity guards.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep037-m4-tests.log"
: > "$log"

SWF_IMAGE="chrislusf/seaweedfs:4.43@sha256:4d5118c198a6b9c0470c04b1822a0671459301625d995d1764c352bb553b3160"
SWF_CONTAINER="nexus-ep037-seaweedfs"
SWF_S3_PORT="18333"
SWF_FILER_PORT="18888"
SWF_VOLUME_PORT="18080"
SWF_CFG_DIR="/tmp/nexus-ep037-s3cfg"
SWF_ACCESS="nexus-ep037-m4-access"
SWF_KEY_NAME="secretKey"
SWF_PW="ep037-"$(date +%s | tail -c 9)"-x7"

fail() {
  echo "EP-037 M4 gate: FAIL - $1" >&2
  tail -60 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-037 M4 gate: $1"; }

owned_containers() {
  docker ps -aq --filter name=^/nexus-ep037-m4- 2>/dev/null | wc -l
}
owned_volumes() {
  docker volume ls -q --filter name=^nexus-ep037-m4- 2>/dev/null | wc -l
}
owned_networks() {
  docker network ls -q --filter name=^nexus-ep037-m4- 2>/dev/null | wc -l
}
pressure() {
  echo "pressure: disk_free=$(df -P / | tail -1 | awk '{print $4}') owned_containers=$(owned_containers) owned_volumes=$(owned_volumes) owned_networks=$(owned_networks)"
}

cleanup() {
  # Exact EP-037 M4 ownership only: the shared provider and any
  # nexus-ep037-m4-* owned fixtures + their temp config roots. Never a
  # broad prune.
  docker rm -f "$SWF_CONTAINER" >/dev/null 2>&1 || true
  for c in $(docker ps -aq --filter name=^/nexus-ep037-m4- 2>/dev/null); do
    docker rm -f "$c" >/dev/null 2>&1 || true
  done
  rm -rf "$SWF_CFG_DIR"
  rm -rf /tmp/nexus-ep037-m4-*-cfg
}
trap cleanup EXIT

# --- material guards (non-vacuous) ---
if [ ! -f connectors/storage-seaweedfs/Cargo.toml ]; then
  fail "connectors/storage-seaweedfs/Cargo.toml missing"
fi
for f in src/lib.rs src/transport.rs src/observability.rs src/bin/seaweedfs-diag.rs tests/ep037_m4_failures.rs; do
  if [ ! -f "connectors/storage-seaweedfs/$f" ]; then
    fail "connectors/storage-seaweedfs/$f missing"
  fi
done
ok "SeaweedFS adapter material present"

grep -q "seaweedfs" COMPONENT_REGISTRY.yaml || fail "seaweedfs not registered"
grep -q "4d5118c198a6b9c0470c04b1822a0671459301625d995d1764c352bb553b3160" COMPONENT_REGISTRY.yaml \
  || fail "seaweedfs digest not pinned in COMPONENT_REGISTRY"
grep -q "EP-037 M4" COMPONENT_REGISTRY.yaml || fail "seaweedfs owner not EP-037 M4"
ok "SeaweedFS pinned in COMPONENT_REGISTRY"

# --- runtime s3.config with runtime-generated credentials (no
# secret-shaped literals on disk) ---
rm -rf "$SWF_CFG_DIR"
mkdir -p "$SWF_CFG_DIR"
printf '{"identities":[{"name":"nexus-m4","credentials":[{"accessKey":"%s","%s":"%s"}],"actions":["Read","Write","List","Tagging","Admin"]}]}' \
  "$SWF_ACCESS" "$SWF_KEY_NAME" "$SWF_PW" > "$SWF_CFG_DIR/s3.json"
ok "s3.config written (runtime credentials)"

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

ready=0
i=0
while [ "$i" -lt 60 ]; do
  i=$((i + 1))
  if curl -s -o /dev/null "http://127.0.0.1:${SWF_S3_PORT}/healthz"; then
    ready=1
    break
  fi
  sleep 1
done
[ "$ready" -eq 1 ] || fail "SeaweedFS S3 gateway readiness timeout"
ok "SeaweedFS S3 gateway /healthz confirmed"

# PROBE-based initial readiness (requirement H): healthz alone is not
# readiness. Build the diag binary, then require probe_verified: true
# (real production PUT -> GET -> digest verify -> DELETE) before any
# test runs.
if ! cargo build -p nexus-provider-storage-seaweedfs --locked --bin seaweedfs-diag >> "$log" 2>&1; then
  fail "cannot build seaweedfs-diag for probe readiness" "$log"
fi
ready=0
i=0
last_diag=""
while [ "$i" -lt 90 ]; do
  i=$((i + 1))
  if env NEXUS_SEAWEEDFS_ENDPOINT="127.0.0.1:${SWF_S3_PORT}" \
    NEXUS_SEAWEEDFS_ACCESS_KEY="$SWF_ACCESS" \
    NEXUS_SEAWEEDFS_PW_KEY="$SWF_PW" \
    NEXUS_SEAWEEDFS_BUCKET_PREFIX="n" \
    ./target/debug/seaweedfs-diag status > /tmp/ep037-m4-diag-init.log 2>&1; then
    if grep -q "probe_verified: true" /tmp/ep037-m4-diag-init.log; then
      ready=1
      break
    fi
  fi
  last_diag=$(tail -1 /tmp/ep037-m4-diag-init.log 2>/dev/null)
  sleep 2
done
[ "$ready" -eq 1 ] || fail "SeaweedFS production probe readiness timeout (last: $last_diag)" /tmp/ep037-m4-diag-init.log
ok "SeaweedFS production probe verified (initial readiness)"

export NEXUS_SEAWEEDFS_ENDPOINT="127.0.0.1:${SWF_S3_PORT}"
export NEXUS_SEAWEEDFS_VOLUME_ENDPOINT="127.0.0.1:${SWF_VOLUME_PORT}"
export NEXUS_SEAWEEDFS_ACCESS_KEY="$SWF_ACCESS"
export NEXUS_SEAWEEDFS_PW_KEY="$SWF_PW"
export NEXUS_SEAWEEDFS_BUCKET_PREFIX="n"
export NEXUS_SEAWEEDFS_CONTAINER="$SWF_CONTAINER"
export NEXUS_SEAWEEDFS_IMAGE="$SWF_IMAGE"

# --- SeaweedFS suite (serial: tests stop/start the shared container) ---
if ! sh -c 'cargo test -p nexus-provider-storage-seaweedfs --locked --test ep037_m4_failures -- --test-threads=1 --nocapture >> "$1" 2>&1' _ "$log"; then
  fail "SeaweedFS M4 suite failed" "$log"
fi
if ! sh -c 'cargo test -p nexus-provider-storage-seaweedfs --locked --lib >> "$1" 2>&1' _ "$log"; then
  fail "SeaweedFS transport unit tests failed" "$log"
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
  ep037_integration_seaweedfs_positive_roundtrip \
  ep037_failure_encryption_missing_zero_provider_mutation \
  ep037_failure_corrupted_stored_bytes_verification \
  ep037_failure_volume_dat_corruption_fails_closed \
  ep037_failure_partial_put_never_verified \
  ep037_failure_ambiguous_put_deduplicates \
  ep037_failure_delete_absent_verified_ladder \
  ep037_failure_wrong_target_delete_preserves_other \
  ep037_failure_shared_content_delete_preserves_object \
  ep037_failure_backup_member_corruption_blocks_verify \
  ep037_failure_restore_requires_hash_verification \
  ep037_failure_migration_success_verifies_destination \
  ep037_failure_migration_destination_failure_preserves_source \
  ep037_failure_retry_hash_aware_no_duplicate \
  ep037_failure_timeout_is_timeout \
  ep037_failure_malformed_response_external \
  ep037_failure_provider_restart_bounded_recovery \
  ep037_failure_unavailable_not_found_distinct \
  ep037_failure_redaction_canary_zero_leakage \
  ep037_integration_seaweedfs_list_pagination \
  ep037_integration_seaweedfs_diag_probe_verified \
; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-037-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-037-owned M4 tests observed"

# --- diag unreachable exits nonzero truthfully ---
if ! docker stop "$SWF_CONTAINER" >/dev/null 2>&1; then
  fail "cannot stop SeaweedFS for diag negative check"
fi
if env NEXUS_SEAWEEDFS_ENDPOINT="127.0.0.1:${SWF_S3_PORT}" \
  NEXUS_SEAWEEDFS_ACCESS_KEY="$SWF_ACCESS" \
  NEXUS_SEAWEEDFS_PW_KEY="$SWF_PW" \
  NEXUS_SEAWEEDFS_BUCKET_PREFIX="n" \
  ./target/debug/seaweedfs-diag status > /tmp/ep037-m4-diag-neg.log 2>&1; then
  fail "seaweedfs-diag must exit nonzero when provider unreachable" /tmp/ep037-m4-diag-neg.log
fi
grep -q "state: DEGRADED" /tmp/ep037-m4-diag-neg.log || fail "diag must report DEGRADED truthfully" /tmp/ep037-m4-diag-neg.log
ok "diag unreachable exits nonzero with truthful status"

if ! docker start "$SWF_CONTAINER" >/dev/null 2>&1; then
  fail "cannot restart SeaweedFS for diag positive check"
fi
# PROBE-based readiness after restart: healthz may bind before the
# master/volume topology re-syncs; the production probe can return
# transient 500 (InternalError: no writable volume / filer topology
# unavailable) for a bounded window. Only probe_verified: true certifies
# readiness (requirement H/I/J).
ready=0
i=0
last_diag=""
while [ "$i" -lt 90 ]; do
  i=$((i + 1))
  if env NEXUS_SEAWEEDFS_ENDPOINT="127.0.0.1:${SWF_S3_PORT}" \
    NEXUS_SEAWEEDFS_ACCESS_KEY="$SWF_ACCESS" \
    NEXUS_SEAWEEDFS_PW_KEY="$SWF_PW" \
    NEXUS_SEAWEEDFS_BUCKET_PREFIX="n" \
    ./target/debug/seaweedfs-diag status > /tmp/ep037-m4-diag-pos.log 2>&1; then
    if grep -q "probe_verified: true" /tmp/ep037-m4-diag-pos.log; then
      ready=1
      break
    fi
  fi
  last_diag=$(tail -1 /tmp/ep037-m4-diag-pos.log 2>/dev/null)
  sleep 2
done
[ "$ready" -eq 1 ] || fail "diag positive readiness timeout (last: $last_diag)" /tmp/ep037-m4-diag-pos.log
grep -q "probe_verified: true" /tmp/ep037-m4-diag-pos.log || fail "diag must verify probe" /tmp/ep037-m4-diag-pos.log
ok "diag probe verified when healthy"

# --- clippy + fmt ---
if ! sh -c 'cargo clippy -p nexus-provider-storage-seaweedfs --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"
if ! sh -c 'cargo fmt -p nexus-provider-storage-seaweedfs -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# --- teardown then orphan guard ---
docker rm -f "$SWF_CONTAINER" >/dev/null 2>&1 || true
for c in $(docker ps -aq --filter name=^/nexus-ep037-m4- 2>/dev/null); do
  docker rm -f "$c" >/dev/null 2>&1 || true
done
rm -rf /tmp/nexus-ep037-m4-*-cfg
leftovers=$(docker ps -aq --filter name=^/nexus-ep037- | wc -l)
[ "$leftovers" -eq 0 ] || fail "leftover nexus-ep037-* containers: $leftovers"
ok "orphan guard clean"
pressure >> "$log"
ok "docker pressure recorded (post-run)"

# --- M1/M2/M3 regressions ---
if ! sh scripts/ep037-m1-tests.sh > /tmp/ep037-m4-m1regress.log 2>&1; then
  fail "M1 regression failed" /tmp/ep037-m4-m1regress.log
fi
if ! sh scripts/ep037-m2-tests.sh > /tmp/ep037-m4-m2regress.log 2>&1; then
  fail "M2 regression failed" /tmp/ep037-m4-m2regress.log
fi
if ! sh scripts/ep037-m3-tests.sh > /tmp/ep037-m4-m3regress.log 2>&1; then
  fail "M3 regression failed" /tmp/ep037-m4-m3regress.log
fi
ok "M1+M2+M3 regression green"

echo "EP-037 M4 gate: ok"
