#!/usr/bin/env sh
# EP-042 M3 gate: real release transport integration proofs through the
# REAL container machinery with vacuity guards (EP-001 gate-masking
# class).
#
# M3 owns infra/release/ (real SigV4 S3 transport over Web Crypto +
# global fetch, digest-bound publish/fetch, readiness probe, idempotent
# publish, timeout/cancellation, redacted audit events, provider +
# container manifests, fixtures) and the ep042_integration_* proofs in
# tests/release/src/integration/ against a REAL digest-pinned SeaweedFS
# S3-gateway container with runtime-generated credentials.
#
# Vacuous green is impossible: a green M3 must observe a real non-zero
# integration proof count, zero failures, real transport script output,
# real probe verification, and zero EP-042-owned residue after teardown.
set -eu
export CI=true
export NO_COLOR=1

log="/tmp/ep042-m3-tests.log"
: > "$log"

SWF_IMAGE="chrislusf/seaweedfs:4.43@sha256:4d5118c198a6b9c0470c04b1822a0671459301625d995d1764c352bb553b3160"
SWF_CONTAINER="nexus-ep042-m3-seaweedfs"
SWF_S3_PORT="19443"
SWF_FILER_PORT="19444"
SWF_VOLUME_PORT="19445"
SWF_CFG_DIR="/tmp/nexus-ep042-m3-s3cfg"
SWF_ACCESS="nexus-ep042-m3-access"
SWF_KEY_NAME="secretKey"
SWF_PW="ep042-m3-"$(date +%s | tail -c 9)"-x7"
SWF_BUCKET="nexus-release-artifacts"
RUN_ID="ep042-m3-"$(date +%s)
GIT_COMMIT="$(git rev-parse HEAD 2>/dev/null || echo unknown)"

fail() {
  echo "EP-042 M3 gate: FAIL - $1" >&2
  tail -60 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-042 M3 gate: $1"; }

owned_containers() {
  docker ps -aq --filter name=^/nexus-ep042-m3- 2>/dev/null | wc -l
}
owned_volumes() {
  docker volume ls -q --filter name=^nexus-ep042-m3- 2>/dev/null | wc -l
}
owned_networks() {
  docker network ls -q --filter name=^nexus-ep042-m3- 2>/dev/null | wc -l
}
pressure() {
  echo "pressure: disk_free=$(df -P / | tail -1 | awk '{print $4}') owned_containers=$(owned_containers) owned_volumes=$(owned_volumes) owned_networks=$(owned_networks)"
}

cleanup() {
  # Exact EP-042 M3 ownership only: the shared provider container and
  # any nexus-ep042-m3-* owned fixtures + temp config roots. Never a
  # broad prune.
  docker rm -f "$SWF_CONTAINER" >/dev/null 2>&1 || true
  for c in $(docker ps -aq --filter name=^/nexus-ep042-m3- 2>/dev/null); do
    docker rm -f "$c" >/dev/null 2>&1 || true
  done
  rm -rf "$SWF_CFG_DIR"
  rm -rf /tmp/nexus-ep042-m3-*-cfg
  rm -rf /tmp/nexus-ep042-m3-*-out
}
trap cleanup EXIT

# --- resource preflight (fence B; never misclassify exhaustion) -----------
disk_free=$(df -P / | tail -1 | awk '{print $4}')
if [ "${disk_free:-0}" -lt 5000000 ]; then
  fail "disk pressure: ${disk_free} KB free (< 5GB) - classify RESOURCE_EXHAUSTION"
fi
pressure >> "$log"
ok "resource preflight passed (disk_free=${disk_free})"

# --- M1 + M2 regressions first ---------------------------------------------
if ! sh scripts/ep042-m1-tests.sh >>"$log" 2>&1; then
  fail "M1 regression gate failed" "$log"
fi
ok "M1 regression green"
if ! sh scripts/ep042-m2-tests.sh >>"$log" 2>&1; then
  fail "M2 regression gate failed" "$log"
fi
ok "M2 regression green"

# --- material presence ------------------------------------------------------
for path in \
  infra/release/package.json \
  infra/release/tsconfig.json \
  infra/release/src/index.ts \
  infra/release/src/errors.ts \
  infra/release/src/sigv4.ts \
  infra/release/src/s3.ts \
  infra/release/src/transport.ts \
  infra/release/src/cli.ts \
  infra/release/scripts/release-probe.sh \
  infra/release/scripts/release-publish.sh \
  infra/release/scripts/release-fetch.sh \
  infra/release/providers/seaweedfs.yaml \
  infra/release/containers/seaweedfs.yaml \
  infra/release/README.md \
  infra/release/fixtures/release-manifest.json \
  infra/release/fixtures/components/nexus-core \
  infra/release/fixtures/components/nexus-model \
  tests/release/vitest.integration.config.ts \
  tests/release/src/integration/ep042_integration_transport.test.ts; do
  [ -f "$path" ] || fail "missing owned path: $path"
done
ok "M3-owned paths present"

# --- workspace registration -------------------------------------------------
grep -q 'infra/release:' pnpm-lock.yaml || fail "infra/release not registered in pnpm-lock.yaml"
grep -q '"@nexus/release-infra": "workspace:\*"' tests/release/package.json || fail "@nexus/release-infra not a workspace dep of tests/release"
grep -q '4d5118c198a6b9c0470c04b1822a0671459301625d995d1764c352bb553b3160' COMPONENT_REGISTRY.yaml || fail "seaweedfs digest not pinned in COMPONENT_REGISTRY"
ok "workspace + registry registration verified"

# --- anti-masking sentinels (node M3 wired to gate) -------------------------
grep -q 'ep042-m3-tests.sh' scripts/nodes/EP-042.sh || fail "node M3 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-042 M3' scripts/nodes/EP-042.sh; then
  fail "node M3 still uses artifact-check masking"
fi
ok "node M3 wired to real gate"

# --- sh syntax ---------------------------------------------------------------
for s in infra/release/scripts/*.sh; do
  sh -n "$s" || fail "sh syntax: $s"
done
ok "transport scripts sh -n clean"

# --- runtime s3.config with runtime-generated credentials --------------------
rm -rf "$SWF_CFG_DIR"
mkdir -p "$SWF_CFG_DIR"
printf '{"identities":[{"name":"nexus-ep042-m3","credentials":[{"accessKey":"%s","%s":"%s"}],"actions":["Read","Write","List","Tagging","Admin"]}]}' \
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
[ "$ready" -eq 1 ] || fail "SeaweedFS S3 gateway healthz timeout"
ok "SeaweedFS S3 gateway /healthz confirmed"

# --- real transport environment + scripts ------------------------------------
export NEXUS_RELEASE_S3_ENDPOINT="127.0.0.1:${SWF_S3_PORT}"
export NEXUS_RELEASE_ACCESS_KEY="$SWF_ACCESS"
export NEXUS_RELEASE_SECRET_KEY="$SWF_PW"
export NEXUS_RELEASE_BUCKET="$SWF_BUCKET"
export NEXUS_RELEASE_RUN_ID="$RUN_ID"
export NEXUS_RELEASE_GIT_COMMIT="$GIT_COMMIT"
export NEXUS_RELEASE_TIMEOUT_MS="10000"

# Real probe via the transport CLI (healthz + PUT/GET/digest/DELETE).
if ! sh infra/release/scripts/release-probe.sh >>"$log" 2>&1; then
  fail "release-probe.sh failed" "$log"
fi
grep -q "probe_verified: true" "$log" || fail "probe_verified: true not observed" "$log"
ok "transport probe verified (real PUT/GET/digest/DELETE)"

# Real publish via the transport CLI + fixtures.
if ! sh infra/release/scripts/release-publish.sh \
  "nexus-1.0.0-rc1" \
  infra/release/fixtures/release-manifest.json \
  infra/release/fixtures/components >>"$log" 2>&1; then
  fail "release-publish.sh failed" "$log"
fi
grep -q "published: nexus-1.0.0-rc1" "$log" || fail "publish sentinel missing" "$log"
ok "release-publish.sh executed for real"

# Real fetch + verify via the transport CLI.
rm -rf /tmp/nexus-ep042-m3-fetch-out
if ! sh infra/release/scripts/release-fetch.sh \
  "nexus-1.0.0-rc1" \
  /tmp/nexus-ep042-m3-fetch-out/manifest.json \
  /tmp/nexus-ep042-m3-fetch-out/components \
  "nexus-core,nexus-model" >>"$log" 2>&1; then
  fail "release-fetch.sh failed" "$log"
fi
grep -q "fetched: nexus-1.0.0-rc1" "$log" || fail "fetch sentinel missing" "$log"
cmp -s infra/release/fixtures/components/nexus-core \
  /tmp/nexus-ep042-m3-fetch-out/components/nexus-core || fail "fetched nexus-core bytes mismatch"
cmp -s infra/release/fixtures/components/nexus-model \
  /tmp/nexus-ep042-m3-fetch-out/components/nexus-model || fail "fetched nexus-model bytes mismatch"
ok "release-fetch.sh verified real bytes (cmp clean)"

# --- vitest integration suite (REAL container) -------------------------------
if ! (cd tests/release && node_modules/.bin/vitest run --config vitest.integration.config.ts >>"$log" 2>&1); then
  fail "vitest integration suite failed" "$log"
fi
if ! grep -Eq 'Tests[[:space:]]+[1-9][0-9]* passed' "$log"; then
  fail "no integration tests ran (vacuity guard)" "$log"
fi
count=$(grep -Eo 'Tests[[:space:]]+[0-9]+ passed' "$log" | grep -Eo '[0-9]+' | head -1)
if [ "${count:-0}" -lt 14 ]; then
  fail "too few integration proofs passed: ${count:-0} (need >= 14)"
fi
if grep -Eq '[1-9][0-9]* failed' "$log"; then
  fail "failures present in vitest integration output" "$log"
fi
ok "vitest integration ${count:-0} proofs passed, zero failed"

# --- anti-masking sentinels: owned integration proofs must have run ----------
for sentinel in \
  ep042_integration_transport_readiness \
  ep042_integration_transport_publish_fetch \
  ep042_integration_transport_digest_binding_fails_closed \
  ep042_integration_transport_auth_fails_closed \
  ep042_integration_transport_timeout_fails_closed \
  ep042_integration_transport_cancellation \
  ep042_integration_transport_idempotency \
  ep042_integration_transport_audit_redaction; do
  if ! grep -rq "$sentinel" tests/release/src/integration/; then
    fail "EP-042-owned proof $sentinel missing from integration sources"
  fi
done
ok "anti-masking sentinels present (readiness/publish-fetch/digest/auth/timeout/cancel/idempotency/audit)"

# --- no-placeholder scan (production transport path only) --------------------
if grep -rniE 'placeholder|TODO|FIXME|not implemented|unimplemented!' \
  infra/release/src 2>/dev/null; then
  fail "placeholder content in infra/release/src"
fi
ok "no-placeholder scan clean (transport src)"

# --- typecheck both packages -------------------------------------------------
if ! (cd infra/release && node_modules/.bin/tsc --noEmit -p tsconfig.json >>"$log" 2>&1); then
  fail "infra/release typecheck failed" "$log"
fi
if ! (cd tests/release && node_modules/.bin/tsc --noEmit -p tsconfig.json >>"$log" 2>&1); then
  fail "tests/release typecheck failed" "$log"
fi
ok "typecheck clean (infra/release + tests/release)"

# --- teardown + residue verification -----------------------------------------
cleanup
left=$(owned_containers)
vols=$(owned_volumes)
nets=$(owned_networks)
[ "$left" -eq 0 ] || fail "EP-042 M3 container residue: $left"
[ "$vols" -eq 0 ] || fail "EP-042 M3 volume residue: $vols"
[ "$nets" -eq 0 ] || fail "EP-042 M3 network residue: $nets"
[ ! -d "$SWF_CFG_DIR" ] || fail "EP-042 M3 temp config residue"
ok "zero EP-042 M3-owned residue (containers/volumes/networks/temp)"

echo "EP-042 M3 gate: ok"
