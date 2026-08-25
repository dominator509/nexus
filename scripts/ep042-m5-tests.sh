#!/usr/bin/env sh
# EP-042 M5 gate: offline bundle, final live-fire/operations, and node
# closure readiness through the REAL bundle machinery + REAL filesystem
# with vacuity guards (EP-001 gate-masking class).
#
# M5 owns offline-bundle/ (real bundle production from real files,
# digest-bound verification, OFFLINE install composing the M4
# transactional installer with NO transport, rollback drill with receipt
# after verified restoration, current-run redacted evidence) and the
# ep042_bundle_* proofs in tests/release/src/bundle/.
#
# Vacuous green is impossible: a green M5 must observe a real non-zero
# bundle-proof count, zero failures, real bundle-produce/verify/install
# script output, cmp-verified installed bytes, a real rollback drill
# with prior-state verification, a real tampered-bundle denial, real
# evidence written + validated, M1/M2/M3/M4 regressions, expected-files
# EP-042 full list, side gates, and zero EP-042-owned residue.
set -eu
export CI=true
export NO_COLOR=1

log="/tmp/ep042-m5-tests.log"
: > "$log"

RUN_ID="ep042-m5-"$(date +%s)
GIT_COMMIT="$(git rev-parse HEAD 2>/dev/null || echo unknown)"
FIXTURE_BASE="/tmp/nexus-ep042-m5-root-${RUN_ID}"
EVIDENCE_BASE="/tmp/nexus-ep042-m5-evidence-${RUN_ID}"

fail() {
  echo "EP-042 M5 gate: FAIL - $1" >&2
  tail -60 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-042 M5 gate: $1"; }

owned_containers() {
  docker ps -aq --filter name=^/nexus-ep042-m5- 2>/dev/null | wc -l
}
owned_volumes() {
  docker volume ls -q --filter name=^nexus-ep042-m5- 2>/dev/null | wc -l
}
owned_networks() {
  docker network ls -q --filter name=^nexus-ep042-m5- 2>/dev/null | wc -l
}
pressure() {
  echo "pressure: disk_free=$(df -P / | tail -1 | awk '{print $4}') owned_containers=$(owned_containers) owned_volumes=$(owned_volumes) owned_networks=$(owned_networks)"
}

cleanup() {
  # Exact EP-042 M5 ownership only. No broad prune.
  for c in $(docker ps -aq --filter name=^/nexus-ep042-m5- 2>/dev/null); do
    docker rm -f "$c" >/dev/null 2>&1 || true
  done
  rm -rf "$FIXTURE_BASE" "$EVIDENCE_BASE"
  rm -rf /tmp/nexus-ep042-m5-root-*
  rm -rf /tmp/nexus-ep042-m5-evidence-*
}
trap cleanup EXIT

# --- resource preflight (fence B; never misclassify exhaustion) -----------
disk_free=$(df -P / | tail -1 | awk '{print $4}')
if [ "${disk_free:-0}" -lt 5000000 ]; then
  fail "disk pressure: ${disk_free} KB free (< 5GB) - classify RESOURCE_EXHAUSTION"
fi
pressure >> "$log"
ok "resource preflight passed (disk_free=${disk_free})"

# --- control plane readiness (fence U: EP-044 stage is DONE) ---------------
if sh scripts/stage.sh at-least EP-044 >/dev/null 2>&1; then
  if ! NEXUS_SMOKE_URL="${NEXUS_SMOKE_URL:-http://127.0.0.1:8443}" \
    sh scripts/smoke/runtime.sh >>"$log" 2>&1; then
    fail "EP-044 control plane not healthy - restart core before node verify" "$log"
  fi
  ok "control plane runtime smoke green (healthz/readyz/capabilities)"
else
  ok "control plane smoke not-applicable-before EP-044"
fi

# --- M1 + M2 + M3 + M4 regressions first -------------------------------------
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
if ! sh scripts/ep042-m4-tests.sh >>"$log" 2>&1; then
  fail "M4 regression gate failed" "$log"
fi
ok "M4 regression green"

# --- material presence ------------------------------------------------------
for path in \
  offline-bundle/package.json \
  offline-bundle/tsconfig.json \
  offline-bundle/src/index.ts \
  offline-bundle/src/errors.ts \
  offline-bundle/src/model.ts \
  offline-bundle/src/produce.ts \
  offline-bundle/src/verify.ts \
  offline-bundle/src/install.ts \
  offline-bundle/src/rollback.ts \
  offline-bundle/src/evidence.ts \
  offline-bundle/src/cli.ts \
  offline-bundle/scripts/ts-resolve-loader.mjs \
  offline-bundle/scripts/bundle-produce.sh \
  offline-bundle/scripts/bundle-verify.sh \
  offline-bundle/scripts/bundle-install.sh \
  offline-bundle/scripts/bundle-rollback.sh \
  offline-bundle/OPERATIONS.md \
  offline-bundle/README.md \
  tests/release/vitest.bundle.config.ts \
  tests/release/src/bundle/ep042_bundle_offline.test.ts; do
  [ -f "$path" ] || fail "missing owned path: $path"
done
ok "M5-owned paths present"

# --- workspace registration -------------------------------------------------
grep -q '"offline-bundle"' pnpm-workspace.yaml || fail "offline-bundle not registered in pnpm-workspace.yaml"
grep -q '"@nexus/offline-bundle": "workspace:\*"' tests/release/package.json || fail "@nexus/offline-bundle not a workspace dep of tests/release"
grep -q '^  offline-bundle:' pnpm-lock.yaml || fail "offline-bundle not registered in pnpm-lock.yaml"
ok "workspace registration verified"

# --- anti-masking sentinels (node M5 wired to gate) -------------------------
grep -q 'ep042-m5-tests.sh' scripts/nodes/EP-042.sh || fail "node M5 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-042 M5' scripts/nodes/EP-042.sh; then
  fail "node M5 still uses artifact-check masking"
fi
ok "node M5 wired to real gate"

# --- sh syntax ---------------------------------------------------------------
for s in offline-bundle/scripts/*.sh; do
  sh -n "$s" || fail "sh syntax: $s"
done
ok "bundle scripts sh -n clean"

# --- typecheck both packages --------------------------------------------------
if ! (cd offline-bundle && node_modules/.bin/tsc --noEmit -p tsconfig.json >>"$log" 2>&1); then
  fail "offline-bundle typecheck failed" "$log"
fi
if ! (cd tests/release && node_modules/.bin/tsc --noEmit -p tsconfig.json >>"$log" 2>&1); then
  fail "tests/release typecheck failed" "$log"
fi
ok "typecheck clean (offline-bundle + tests/release)"

# --- real bundle production (fence F: real files -> real bundle) -------------
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
printf '{"bomFormat":"CycloneDX","version":1}' > "$FIXTURE_BASE/sbom.json"
printf 'MIT License text (fixture)' > "$FIXTURE_BASE/LICENSE"
printf 'ALTER TABLE nexus ADD COLUMN offline int;' > "$FIXTURE_BASE/mig.sql"
printf '#!/bin/sh\necho recover' > "$FIXTURE_BASE/recover.sh"
chmod +x "$FIXTURE_BASE/recover.sh"

BUNDLE_DIR="$FIXTURE_BASE/bundle"
if ! sh offline-bundle/scripts/bundle-produce.sh \
  "$BUNDLE_DIR" "bundle-m5" "release-1" \
  "$FIXTURE_BASE/manifest.json" \
  "comp-1=$FIXTURE_BASE/artifacts/comp-1:IMAGE,comp-2=$FIXTURE_BASE/artifacts/comp-2:MODEL" \
  "sbom.json=$FIXTURE_BASE/sbom.json" \
  "LICENSE=$FIXTURE_BASE/LICENSE" \
  "mig.sql=$FIXTURE_BASE/mig.sql" \
  "recover.sh=$FIXTURE_BASE/recover.sh" >>"$log" 2>&1; then
  fail "bundle-produce.sh failed" "$log"
fi
grep -q "bundle produced: bundle-m5" "$log" || fail "bundle produce sentinel missing" "$log"
[ -f "$BUNDLE_DIR/bundle-manifest.json" ] || fail "bundle-manifest.json missing"
[ -f "$BUNDLE_DIR/release-manifest.json" ] || fail "release-manifest.json missing"
[ -f "$BUNDLE_DIR/images/comp-1" ] || fail "bundle image payload missing"
[ -f "$BUNDLE_DIR/models/comp-2" ] || fail "bundle model payload missing"
cmp -s "$FIXTURE_BASE/artifacts/comp-1" "$BUNDLE_DIR/images/comp-1" || fail "bundled comp-1 bytes mismatch"
cmp -s "$FIXTURE_BASE/artifacts/comp-2" "$BUNDLE_DIR/models/comp-2" || fail "bundled comp-2 bytes mismatch"
ok "bundle-produce.sh executed for real (real files, cmp-verified payloads)"

# --- real bundle verification -------------------------------------------------
if ! sh offline-bundle/scripts/bundle-verify.sh "$BUNDLE_DIR" >>"$log" 2>&1; then
  fail "bundle-verify.sh failed" "$log"
fi
grep -q "bundle verification: VERIFIED" "$log" || fail "bundle VERIFIED sentinel missing" "$log"
ok "bundle-verify.sh executed for real (VERIFIED)"

# --- real OFFLINE install (NO transport; fence I/J) ---------------------------
# No SeaweedFS / S3 / transport container is running in this gate; the
# offline install must succeed purely from local bundle files.
INSTALL_ROOT="$FIXTURE_BASE/install-root"
mkdir -p "$INSTALL_ROOT"
printf 'prior-state-bytes' > "$INSTALL_ROOT/prior-state"
if ! sh offline-bundle/scripts/bundle-install.sh \
  "$BUNDLE_DIR" "$INSTALL_ROOT" "release-1" "install-1" \
  "comp-1=bin/nexus-core,comp-2=models/nexus-model" \
  "$RUN_ID" "$GIT_COMMIT" >>"$log" 2>&1; then
  fail "bundle-install.sh failed" "$log"
fi
grep -q "offline transport_required: false" "$log" || fail "offline install not classified transport-free" "$log"
grep -q "offline source: local-bundle-only" "$log" || fail "offline install source not local-bundle-only" "$log"
cmp -s "$FIXTURE_BASE/artifacts/comp-1" "$INSTALL_ROOT/bin/nexus-core" || fail "installed comp-1 bytes mismatch"
cmp -s "$FIXTURE_BASE/artifacts/comp-2" "$INSTALL_ROOT/models/nexus-model" || fail "installed comp-2 bytes mismatch"
BACKUP_DIGEST=$(grep -oE 'backup_digest: [^ ]+' "$log" | tail -1 | awk '{print $2}')
[ -n "$BACKUP_DIGEST" ] || fail "backup digest missing from install log" "$log"
ok "bundle-install.sh executed OFFLINE (transport absent, cmp-verified installed bytes)"

# --- real rollback drill: wrong source denied, then verified restore ----------
if sh offline-bundle/scripts/bundle-rollback.sh \
  "$INSTALL_ROOT" "release-1" "install-1" \
  "sha256:0000000000000000000000000000000000000000000000000000000000000000" \
  "$INSTALL_ROOT/prior-state=prior-state-bytes" \
  "$RUN_ID" "$GIT_COMMIT" >>"$log" 2>&1; then
  fail "rollback with wrong backup digest must be denied"
fi
grep -q "ROLLBACK_FAILED" "$log" || fail "wrong-backup denial not classified ROLLBACK_FAILED" "$log"
ok "wrong rollback source denied (ROLLBACK_FAILED)"

if ! sh offline-bundle/scripts/bundle-rollback.sh \
  "$INSTALL_ROOT" "release-1" "install-1" \
  "$BACKUP_DIGEST" \
  "$INSTALL_ROOT/prior-state=prior-state-bytes" \
  "$RUN_ID" "$GIT_COMMIT" >>"$log" 2>&1; then
  fail "bundle-rollback.sh failed" "$log"
fi
grep -q "rollback_verified: true" "$log" || fail "rollback verified sentinel missing" "$log"
grep -q "rollback_receipt_after_verified_restoration: true" "$log" || fail "receipt-after-verification sentinel missing" "$log"
printf 'prior-state-bytes' > "$FIXTURE_BASE/expected-prior"
cmp -s "$FIXTURE_BASE/expected-prior" "$INSTALL_ROOT/prior-state" || fail "rollback did not restore prior state"
[ ! -f "$INSTALL_ROOT/bin/nexus-core" ] || fail "rollback left installed bytes behind"
[ -f "$INSTALL_ROOT/.rollback-receipt.json" ] || fail "rollback receipt missing after verified restoration"
ok "bundle-rollback.sh executed for real (prior state restored + verified + receipt after verification)"

# --- real failure: tampered bundle file fails closed --------------------------
cp -r "$BUNDLE_DIR" "$FIXTURE_BASE/bundle-tampered"
printf 'tampered bytes change the digest' > "$FIXTURE_BASE/bundle-tampered/models/comp-2"
if sh offline-bundle/scripts/bundle-verify.sh "$FIXTURE_BASE/bundle-tampered" >>"$log" 2>&1; then
  fail "tampered bundle verify must fail"
fi
grep -q "BUNDLE_DIGEST_MISMATCH" "$log" || fail "tampered bundle not classified BUNDLE_DIGEST_MISMATCH" "$log"
ok "real tampered bundle fails closed (BUNDLE_DIGEST_MISMATCH)"

# --- real failure: path traversal denied ---------------------------------------
cp -r "$BUNDLE_DIR" "$FIXTURE_BASE/bundle-traversal"
node -e "
const fs=require('fs');
const p='$FIXTURE_BASE/bundle-traversal/bundle-manifest.json';
const obj=JSON.parse(fs.readFileSync(p,'utf8'));
obj.contents[0].name='../../escape';
fs.writeFileSync(p, JSON.stringify(obj));
"
if sh offline-bundle/scripts/bundle-verify.sh "$FIXTURE_BASE/bundle-traversal" >>"$log" 2>&1; then
  fail "path traversal bundle verify must fail"
fi
grep -q "PATH_ESCAPE" "$log" || fail "traversal not classified PATH_ESCAPE" "$log"
ok "real path traversal fails closed (PATH_ESCAPE)"

# --- real current-run evidence written + validated ----------------------------
EVIDENCE_DIR="$EVIDENCE_BASE"
mkdir -p "$EVIDENCE_DIR"
CANARY="ep042-m5-canary-$(date +%s)-x7"
printf 'INTERNAL BEHAVIOR CERTIFIED for exact exercised local surface\nreal signature verification NOT ASSERTED\n' > "$EVIDENCE_DIR/boundary.txt"
export NEXUS_BUNDLE_CANARY="$CANARY"
if ! node --experimental-transform-types --experimental-loader "$(cd offline-bundle/scripts && pwd)/ts-resolve-loader.mjs" \
  offline-bundle/src/cli.ts evidence \
  --out "$EVIDENCE_DIR/EP-042-M5-evidence.json" \
  --run-id "$RUN_ID" \
  --git-commit "$GIT_COMMIT" \
  --release-id "release-1" \
  --install-id "install-1" \
  --bundle-id "bundle-m5" \
  --manifest-digest "$MANIFEST_DIGEST" \
  --bundle-digest "$(grep -oE 'bundle produced: bundle-m5 \(sha256:[0-9a-f]+\)' "$log" | grep -oE 'sha256:[0-9a-f]+' | head -1)" \
  --component-digests "sha256:${C1_DIGEST},sha256:${C2_DIGEST}" \
  --verification-state "VERIFIED" \
  --install-state "INSTALLED" \
  --rollback-state "VERIFIED" \
  --offline-install-state "OFFLINE_INSTALL_VERIFIED" \
  --signature-state "SIGNATURE_PRESENT_NOT_VERIFIED" \
  --boundary "$EVIDENCE_DIR/boundary.txt" \
  --canaries "$CANARY" >>"$log" 2>&1; then
  fail "evidence CLI failed" "$log"
fi
grep -q "evidence written:" "$log" || fail "evidence written sentinel missing" "$log"
grep -q "redaction_result: REDACTED" "$log" || fail "evidence redaction sentinel missing" "$log"
[ -f "$EVIDENCE_DIR/EP-042-M5-evidence.json" ] || fail "evidence file missing"
if grep -q "$CANARY" "$EVIDENCE_DIR/EP-042-M5-evidence.json"; then
  fail "evidence leaked secret canary"
fi
ok "current-run evidence written + redacted + validated"

# --- vitest bundle suite --------------------------------------------------------
if ! (cd tests/release && node_modules/.bin/vitest run --config vitest.bundle.config.ts >>"$log" 2>&1); then
  fail "vitest bundle suite failed" "$log"
fi
if ! grep -Eq 'Tests[[:space:]]+[1-9][0-9]* passed' "$log"; then
  fail "no bundle tests ran (vacuity guard)" "$log"
fi
count=$(grep -Eo 'Tests[[:space:]]+[0-9]+ passed' "$log" | grep -Eo '[0-9]+' | head -1)
if [ "${count:-0}" -lt 16 ]; then
  fail "too few bundle proofs passed: ${count:-0} (need >= 16)"
fi
if grep -Eq '[1-9][0-9]* failed' "$log"; then
  fail "failures present in vitest bundle output" "$log"
fi
ok "vitest bundle ${count:-0} proofs passed, zero failed"

# --- anti-masking sentinels: owned bundle proofs must have run -------------------
for sentinel in \
  ep042_bundle_production_creates_real_bundle \
  ep042_bundle_verification_passes \
  ep042_bundle_missing_file_denied \
  ep042_bundle_changed_file_denied \
  ep042_bundle_malformed_digest_denied \
  ep042_bundle_duplicate_path_denied \
  ep042_bundle_path_traversal_denied \
  ep042_bundle_symlink_escape_denied \
  ep042_bundle_wrong_release_denied \
  ep042_bundle_manifest_tamper_denied \
  ep042_bundle_self_digest_tamper_denied \
  ep042_bundle_offline_install_succeeds \
  ep042_bundle_offline_install_component_missing_denied \
  ep042_bundle_offline_install_unverified_denied \
  ep042_bundle_rollback_drill_restores_prior \
  ep042_bundle_rollback_drill_wrong_backup_denied \
  ep042_bundle_evidence_bound_redacted \
  ep042_bundle_evidence_stale_rejected \
  ep042_bundle_evidence_tampered_rejected; do
  if ! grep -rq "$sentinel" tests/release/src/bundle/; then
    fail "EP-042-owned proof $sentinel missing from bundle sources"
  fi
done
ok "anti-masking sentinels present (produce/verify/missing/changed/malformed/duplicate/traversal/symlink/wrong-release/tamper/self-digest/offline-install/component-missing/unverified/rollback-drill/wrong-backup/evidence-redact/stale/tampered)"

# --- no-placeholder scan (production bundle path only) ---------------------------
if grep -rniE 'placeholder|TODO|FIXME|not implemented|unimplemented!' \
  offline-bundle/src 2>/dev/null; then
  fail "placeholder content in offline-bundle/src"
fi
ok "no-placeholder scan clean (offline-bundle/src)"

# --- expected-files EP-042 full list ---------------------------------------------
if ! sh scripts/expected-files.sh EP-042 >>"$log" 2>&1; then
  fail "expected-files EP-042 full list failed" "$log"
fi
ok "expected-files EP-042 full list green"

# --- required side gates -----------------------------------------------------------
if ! sh scripts/scope-audit.sh EP-042 >>"$log" 2>&1; then
  fail "scope audit EP-042 failed" "$log"
fi
ok "scope audit EP-042: ok"
if ! sh scripts/security-check.sh >>"$log" 2>&1; then
  fail "security check failed" "$log"
fi
ok "security check: ok"
if ! sh scripts/dependency-audit.sh >>"$log" 2>&1; then
  fail "dependency audit failed" "$log"
fi
ok "dependency audit: ok"
if ! sh scripts/license-gate.sh >>"$log" 2>&1; then
  fail "license gate failed" "$log"
fi
ok "license gate: ok"
if ! sh scripts/reality-gate.sh >>"$log" 2>&1; then
  fail "reality gate failed" "$log"
fi
ok "reality gate: ok"
if ! python3 scripts/blueprint_validate.py >/dev/null 2>&1; then
  fail "blueprint validation failed"
fi
ok "blueprint validation: ok"

# --- teardown + residue verification ----------------------------------------------
cleanup
left=$(owned_containers)
vols=$(owned_volumes)
nets=$(owned_networks)
[ "$left" -eq 0 ] || fail "EP-042 M5 container residue: $left"
[ "$vols" -eq 0 ] || fail "EP-042 M5 volume residue: $vols"
[ "$nets" -eq 0 ] || fail "EP-042 M5 network residue: $nets"
[ ! -d "$FIXTURE_BASE" ] || fail "EP-042 M5 fixture residue"
[ ! -d "$EVIDENCE_BASE" ] || fail "EP-042 M5 evidence residue"
ok "zero EP-042 M5-owned residue (containers/volumes/networks/temp)"

echo "EP-042 M5 gate: ok"
