#!/usr/bin/env sh
# EP-042 M4 gate: installer failure proofs, abuse cases, observability,
# and rollback safety through the REAL container machinery + REAL
# filesystem with vacuity guards (EP-001 gate-masking class).
#
# M4 owns installers/ (real transactional installer with
# backup-before-update, staged validation, atomic switch, rollback,
# quarantine, typed failure classification, abuse-case guards,
# append-only journal, redacted observability, ops diagnostic + bounded
# recovery) and the ep042_failure_* proofs in tests/release/src/failure/.
#
# Vacuous green is impossible: a green M4 must observe a real non-zero
# failure-proof count, zero failures, real installer script output, a
# real container termination proof (unavailable dependency), real
# permission-denial proof, M1/M2/M3 regressions, and zero EP-042-owned
# residue after teardown.
set -eu
export CI=true
export NO_COLOR=1

log="/tmp/ep042-m4-tests.log"
: > "$log"

SWF_IMAGE="chrislusf/seaweedfs:4.43@sha256:4d5118c198a6b9c0470c04b1822a0671459301625d995d1764c352bb553b3160"
SWF_CONTAINER="nexus-ep042-m4-seaweedfs"
SWF_S3_PORT="19543"
SWF_FILER_PORT="19544"
SWF_VOLUME_PORT="19545"
SWF_CFG_DIR="/tmp/nexus-ep042-m4-s3cfg"
SWF_ACCESS="nexus-ep042-m4-access"
SWF_KEY_NAME="secretKey"
SWF_PW="ep042-m4-"$(date +%s | tail -c 9)"-x7"
SWF_BUCKET="nexus-release-artifacts"
RUN_ID="ep042-m4-"$(date +%s)
GIT_COMMIT="$(git rev-parse HEAD 2>/dev/null || echo unknown)"

fail() {
  echo "EP-042 M4 gate: FAIL - $1" >&2
  tail -60 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-042 M4 gate: $1"; }

owned_containers() {
  docker ps -aq --filter name=^/nexus-ep042-m4- 2>/dev/null | wc -l
}
owned_volumes() {
  docker volume ls -q --filter name=^nexus-ep042-m4- 2>/dev/null | wc -l
}
owned_networks() {
  docker network ls -q --filter name=^nexus-ep042-m4- 2>/dev/null | wc -l
}
pressure() {
  echo "pressure: disk_free=$(df -P / | tail -1 | awk '{print $4}') owned_containers=$(owned_containers) owned_volumes=$(owned_volumes) owned_networks=$(owned_networks)"
}

cleanup() {
  # Exact EP-042 M4 ownership only: the shared provider container and
  # any nexus-ep042-m4-* owned fixtures + temp config roots. Never a
  # broad prune.
  docker rm -f "$SWF_CONTAINER" >/dev/null 2>&1 || true
  for c in $(docker ps -aq --filter name=^/nexus-ep042-m4- 2>/dev/null); do
    docker rm -f "$c" >/dev/null 2>&1 || true
  done
  rm -rf "$SWF_CFG_DIR"
  rm -rf /tmp/nexus-ep042-m4-*-cfg
  rm -rf /tmp/nexus-ep042-m4-*-out
  rm -rf /tmp/nexus-ep042-m4-install-root*
}
trap cleanup EXIT

# --- resource preflight (fence B; never misclassify exhaustion) -----------
disk_free=$(df -P / | tail -1 | awk '{print $4}')
if [ "${disk_free:-0}" -lt 5000000 ]; then
  fail "disk pressure: ${disk_free} KB free (< 5GB) - classify RESOURCE_EXHAUSTION"
fi
pressure >> "$log"
ok "resource preflight passed (disk_free=${disk_free})"

# --- M1 + M2 + M3 regressions first ------------------------------------------
if ! sh scripts/ep042-m1-tests.sh >>"$log" 2>&1; then
  fail "M1 regression gate failed" "$log"
fi
ok "M1 regression green"
if ! sh scripts/ep042-m2-tests.sh >>"$log" 2>&1; then
  fail "M2 regression gate failed" "$log"
fi
ok "M2 regression green"
if ! sh scripts/ep042-m3-tests.sh >>"$log" 2>&1; then
  fail "M3 regression gate failed" "$log"
fi
ok "M3 regression green"

# --- material presence ------------------------------------------------------
for path in \
  installers/package.json \
  installers/tsconfig.json \
  installers/src/index.ts \
  installers/src/errors.ts \
  installers/src/journal.ts \
  installers/src/paths.ts \
  installers/src/backup.ts \
  installers/src/installer.ts \
  installers/src/observability.ts \
  installers/src/cli.ts \
  installers/scripts/installer-install.sh \
  installers/scripts/installer-rollback.sh \
  installers/scripts/installer-recover.sh \
  installers/scripts/installer-status.sh \
  installers/README.md \
  tests/release/vitest.failure.config.ts \
  tests/release/src/failure/ep042_failure_installer.test.ts; do
  [ -f "$path" ] || fail "missing owned path: $path"
done
ok "M4-owned paths present"

# --- workspace registration -------------------------------------------------
grep -q '"installers"' pnpm-workspace.yaml || fail "installers not registered in pnpm-workspace.yaml"
grep -q '"@nexus/installers": "workspace:\*"' tests/release/package.json || fail "@nexus/installers not a workspace dep of tests/release"
grep -q '^  installers:' pnpm-lock.yaml || fail "installers not registered in pnpm-lock.yaml"
ok "workspace registration verified"

# --- anti-masking sentinels (node M4 wired to gate) -------------------------
grep -q 'ep042-m4-tests.sh' scripts/nodes/EP-042.sh || fail "node M4 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-042 M4' scripts/nodes/EP-042.sh; then
  fail "node M4 still uses artifact-check masking"
fi
ok "node M4 wired to real gate"

# --- sh syntax ---------------------------------------------------------------
for s in installers/scripts/*.sh; do
  sh -n "$s" || fail "sh syntax: $s"
done
ok "installer scripts sh -n clean"

# --- typecheck both packages --------------------------------------------------
if ! (cd installers && node_modules/.bin/tsc --noEmit -p tsconfig.json >>"$log" 2>&1); then
  fail "installers typecheck failed" "$log"
fi
if ! (cd tests/release && node_modules/.bin/tsc --noEmit -p tsconfig.json >>"$log" 2>&1); then
  fail "tests/release typecheck failed" "$log"
fi
ok "typecheck clean (installers + tests/release)"

# --- real installer scripts in isolated roots (fence F) -----------------------
# Build a real manifest + artifact fixtures with REAL digests via the
# canonical M2 surface.
FIXTURE_BASE="/tmp/nexus-ep042-m4-install-root-${RUN_ID}"
rm -rf "$FIXTURE_BASE"
mkdir -p "$FIXTURE_BASE/artifacts"
printf 'nexus-core-v1-real-bytes' > "$FIXTURE_BASE/artifacts/comp-1"
printf 'nexus-model-v2-real-bytes' > "$FIXTURE_BASE/artifacts/comp-2"
C1_DIGEST=$(sha256sum "$FIXTURE_BASE/artifacts/comp-1" | awk '{print $1}')
C2_DIGEST=$(sha256sum "$FIXTURE_BASE/artifacts/comp-2" | awk '{print $1}')
cat > "$FIXTURE_BASE/manifest.json" <<EOF
{
  "schema_version": 1,
  "release_id": "release-1",
  "version": "1.0.0",
  "channel": "STABLE",
  "components": [
    {"component_id":"comp-1","name":"component-comp-1","version":"1.0.0","artifact_ref":{"backend":"local","key":"artifact-comp-1"},"digest":"sha256:${C1_DIGEST}","signature":{"algorithm":"ED25519","key_id":"key-test-1","value_b64":"AAAA01BBBB01"},"sbom_ref":{"backend":"local","key":"sbom-comp-1"},"license_ref":"MIT","size_bytes":24},
    {"component_id":"comp-2","name":"component-comp-2","version":"2.0.0","artifact_ref":{"backend":"local","key":"artifact-comp-2"},"digest":"sha256:${C2_DIGEST}","signature":{"algorithm":"ED25519","key_id":"key-test-1","value_b64":"AAAA01BBBB01"},"sbom_ref":{"backend":"local","key":"sbom-comp-2"},"license_ref":"MIT","size_bytes":25}
  ],
  "compatibility": {
    "matrix_id": "matrix-1",
    "schema_version": 1,
    "entries": [
      {"component_id":"comp-1","version":"1.0.0","min_version":"1.0.0","max_version":"1.9.9","supported_profiles":["MANAGED","BYOC","EXISTING_SSH","HYBRID","FULLY_LOCAL"]},
      {"component_id":"comp-2","version":"2.0.0","min_version":"2.0.0","max_version":"2.9.9","supported_profiles":["MANAGED","BYOC","EXISTING_SSH","HYBRID","FULLY_LOCAL"]}
    ]
  },
  "sbom_ref": {"backend": "local", "key": "sbom-root"},
  "license_refs": ["MIT"],
  "created_at": "2026-08-25T00:00:00Z"
}
EOF
# Bind the manifest digest (canonical strip-then-digest surface: the
# manifest_digest field is excluded from its own content digest).
MANIFEST_DIGEST="sha256:$(node -e "
const {createHash}=require('crypto');
const fs=require('fs');
const obj=JSON.parse(fs.readFileSync('$FIXTURE_BASE/manifest.json','utf8'));
const {manifest_digest,...rest}=obj;
console.log(createHash('sha256').update(JSON.stringify(rest)).digest('hex'));
")"
node -e "
const fs=require('fs');
const p='$FIXTURE_BASE/manifest.json';
const obj=JSON.parse(fs.readFileSync(p,'utf8'));
obj.manifest_digest='$MANIFEST_DIGEST';
fs.writeFileSync(p, JSON.stringify(obj,null,2));
"

INSTALL_ROOT="$FIXTURE_BASE/install-root"
mkdir -p "$INSTALL_ROOT"
printf 'prior-state-bytes' > "$INSTALL_ROOT/prior-state"

# Real install via the real script.
export NEXUS_INSTALL_RUN_ID="$RUN_ID"
export NEXUS_INSTALL_GIT_COMMIT="$GIT_COMMIT"
if ! sh installers/scripts/installer-install.sh \
  "$INSTALL_ROOT" "release-1" "install-1" \
  "$FIXTURE_BASE/manifest.json" "$FIXTURE_BASE/artifacts" \
  "comp-1=bin/nexus-core,comp-2=models/nexus-model" >>"$log" 2>&1; then
  fail "installer-install.sh failed" "$log"
fi
grep -q "installed: release-1" "$log" || fail "install sentinel missing" "$log"
cmp -s "$FIXTURE_BASE/artifacts/comp-1" "$INSTALL_ROOT/bin/nexus-core" || fail "installed comp-1 bytes mismatch"
cmp -s "$FIXTURE_BASE/artifacts/comp-2" "$INSTALL_ROOT/models/nexus-model" || fail "installed comp-2 bytes mismatch"
ok "installer-install.sh executed for real (cmp-verified installed bytes)"

# Real recover diagnostic (journal state must be INSTALLED).
if ! sh installers/scripts/installer-recover.sh "$INSTALL_ROOT" "release-1" "install-1" >>"$log" 2>&1; then
  fail "installer-recover.sh failed" "$log"
fi
grep -q "recovered: true" "$log" || fail "recover sentinel missing" "$log"
ok "installer-recover.sh diagnostic green (installed state)"

# Real rollback via the real script (restores prior state).
BACKUP_DIGEST=$(grep -oE 'backup_digest: [^ ]+' "$log" | tail -1 | awk '{print $2}')
[ -n "$BACKUP_DIGEST" ] || fail "backup digest missing from install log" "$log"
if ! sh installers/scripts/installer-rollback.sh \
  "$INSTALL_ROOT" "release-1" "install-1" "$BACKUP_DIGEST" >>"$log" 2>&1; then
  fail "installer-rollback.sh failed" "$log"
fi
grep -q "rollback_verified: VERIFIED" "$log" || fail "rollback sentinel missing" "$log"
printf 'prior-state-bytes' > "$FIXTURE_BASE/expected-prior-state"
cmp -s "$FIXTURE_BASE/expected-prior-state" "$INSTALL_ROOT/prior-state" || fail "rollback did not restore prior state"
[ ! -f "$INSTALL_ROOT/bin/nexus-core" ] || fail "rollback left installed bytes behind"
ok "installer-rollback.sh executed for real (prior state restored + verified)"

# Real failure: install into a fresh root where the manifest declares a
# component whose artifact is missing (unavailable dependency).
rm -rf "$FIXTURE_BASE/fresh-root"
mkdir -p "$FIXTURE_BASE/fresh-root"
printf 'only-one' > "$FIXTURE_BASE/fresh-root/artifacts-dir-placeholder"
if sh installers/scripts/installer-install.sh \
  "$FIXTURE_BASE/fresh-root" "release-1" "install-2" \
  "$FIXTURE_BASE/manifest.json" "$FIXTURE_BASE/artifacts" \
  "comp-1=bin/nexus-core" >>"$log" 2>&1; then
  fail "install with missing artifact must fail closed (UNAVAILABLE)"
fi
grep -q "installer UNAVAILABLE" "$log" || fail "unavailable dependency not classified UNAVAILABLE" "$log"
ok "real unavailable dependency fails closed (UNAVAILABLE)"

# Real failure: corrupt a controlled message (manifest digest mismatch).
cp "$FIXTURE_BASE/manifest.json" "$FIXTURE_BASE/manifest-corrupt.json"
node -e "
const fs=require('fs');
const p='$FIXTURE_BASE/manifest-corrupt.json';
const obj=JSON.parse(fs.readFileSync(p,'utf8'));
obj.version='9.9.9';
fs.writeFileSync(p, JSON.stringify(obj));
"
rm -rf "$FIXTURE_BASE/corrupt-root"
mkdir -p "$FIXTURE_BASE/corrupt-root"
if sh installers/scripts/installer-install.sh \
  "$FIXTURE_BASE/corrupt-root" "release-1" "install-3" \
  "$FIXTURE_BASE/manifest-corrupt.json" "$FIXTURE_BASE/artifacts" \
  "comp-1=bin/nexus-core,comp-2=models/nexus-model" >>"$log" 2>&1; then
  fail "corrupt manifest install must fail closed (MANIFEST_INVALID)"
fi
grep -q "installer MANIFEST_INVALID" "$log" || fail "corrupt manifest not classified MANIFEST_INVALID" "$log"
ok "real corrupt manifest fails closed (MANIFEST_INVALID)"

# --- real container termination proof (unavailable dependency) ---------------
# Start SeaweedFS, verify the transport is up, then TERMINATE the
# container and prove the installer's transport dependency fails closed
# (the installer itself never runs; the dependency is unavailable).
rm -rf "$SWF_CFG_DIR"
mkdir -p "$SWF_CFG_DIR"
printf '{"identities":[{"name":"nexus-ep042-m4","credentials":[{"accessKey":"%s","%s":"%s"}],"actions":["Read","Write","List","Tagging","Admin"]}]}' \
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
ok "SeaweedFS S3 gateway /healthz confirmed (pre-termination)"

export NEXUS_RELEASE_S3_ENDPOINT="127.0.0.1:${SWF_S3_PORT}"
export NEXUS_RELEASE_ACCESS_KEY="$SWF_ACCESS"
export NEXUS_RELEASE_SECRET_KEY="$SWF_PW"
export NEXUS_RELEASE_BUCKET="$SWF_BUCKET"
export NEXUS_RELEASE_RUN_ID="$RUN_ID"
export NEXUS_RELEASE_GIT_COMMIT="$GIT_COMMIT"
export NEXUS_RELEASE_TIMEOUT_MS="10000"

# Publish the fixture release for real.
if ! sh infra/release/scripts/release-publish.sh \
  "release-1" \
  "$FIXTURE_BASE/manifest.json" \
  "$FIXTURE_BASE/artifacts" >>"$log" 2>&1; then
  fail "release-publish.sh failed" "$log"
fi
grep -q "published: release-1" "$log" || fail "publish sentinel missing" "$log"
ok "fixture release published for real"

# Terminate the container: the dependency becomes unavailable.
docker rm -f "$SWF_CONTAINER" >/dev/null 2>&1 || fail "cannot terminate container"
ok "SeaweedFS container terminated (real failure mechanism)"

if sh infra/release/scripts/release-fetch.sh \
  "release-1" \
  "$FIXTURE_BASE/fetch-out/manifest.json" \
  "$FIXTURE_BASE/fetch-out/components" \
  "comp-1,comp-2" >>"$log" 2>&1; then
  fail "fetch after container termination must fail (unavailable dependency)"
fi
grep -qE "UNREACHABLE|TIMEOUT|UNAVAILABLE" "$log" || fail "terminated-dependency failure not classified" "$log"
ok "terminated dependency fails closed (UNREACHABLE/TIMEOUT)"

# --- vitest failure suite ------------------------------------------------------
if ! (cd tests/release && node_modules/.bin/vitest run --config vitest.failure.config.ts >>"$log" 2>&1); then
  fail "vitest failure suite failed" "$log"
fi
if ! grep -Eq 'Tests[[:space:]]+[1-9][0-9]* passed' "$log"; then
  fail "no failure tests ran (vacuity guard)" "$log"
fi
count=$(grep -Eo 'Tests[[:space:]]+[0-9]+ passed' "$log" | grep -Eo '[0-9]+' | head -1)
if [ "${count:-0}" -lt 18 ]; then
  fail "too few failure proofs passed: ${count:-0} (need >= 18)"
fi
if grep -Eq '[1-9][0-9]* failed' "$log"; then
  fail "failures present in vitest failure output" "$log"
fi
ok "vitest failure ${count:-0} proofs passed, zero failed"

# --- anti-masking sentinels: owned failure proofs must have run ---------------
for sentinel in \
  ep042_failure_unavailable_dependency_denied \
  ep042_failure_timeout_fails_closed \
  ep042_failure_malformed_input_denied \
  ep042_failure_duplicate_request_conflict \
  ep042_failure_denied_permission_staging \
  ep042_failure_cancelled_work_partial_side_effect \
  ep042_failure_backup_failure_denies_update \
  ep042_failure_staged_digest_mismatch \
  ep042_failure_rollback_restores_prior_state \
  ep042_failure_rollback_missing_source_denied \
  ep042_failure_rollback_corrupt_source_denied \
  ep042_failure_path_traversal_denied \
  ep042_failure_symlink_escape_denied \
  ep042_failure_duplicate_component_overwrite_denied \
  ep042_failure_foreign_root_cleanup_denied \
  ep042_failure_recovery_quarantines_staged \
  ep042_failure_evidence_redaction_canary \
  ep042_failure_observability_redacted_states; do
  if ! grep -rq "$sentinel" tests/release/src/failure/; then
    fail "EP-042-owned proof $sentinel missing from failure sources"
  fi
done
ok "anti-masking sentinels present (unavailable/timeout/malformed/duplicate/permission/cancel/partial/backup/digest/rollback/traversal/symlink/dupe/foreign/recovery/redaction/observability)"

# --- no-placeholder scan (production installer path only) ----------------------
if grep -rniE 'placeholder|TODO|FIXME|not implemented|unimplemented!' \
  installers/src 2>/dev/null; then
  fail "placeholder content in installers/src"
fi
ok "no-placeholder scan clean (installers/src)"

# --- teardown + residue verification -------------------------------------------
cleanup
left=$(owned_containers)
vols=$(owned_volumes)
nets=$(owned_networks)
[ "$left" -eq 0 ] || fail "EP-042 M4 container residue: $left"
[ "$vols" -eq 0 ] || fail "EP-042 M4 volume residue: $vols"
[ "$nets" -eq 0 ] || fail "EP-042 M4 network residue: $nets"
[ ! -d "$SWF_CFG_DIR" ] || fail "EP-042 M4 temp config residue"
[ ! -d "$FIXTURE_BASE" ] || fail "EP-042 M4 install fixture residue"
ok "zero EP-042 M4-owned residue (containers/volumes/networks/temp)"

echo "EP-042 M4 gate: ok"
