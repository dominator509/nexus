#!/usr/bin/env sh
# RX-009 remediation battery: cryptographic supply-chain evidence sealing
# (AUD-059), multi-ecosystem shipped-product SBOM inventory (AUD-060),
# cryptographic release-bundle signature verification (AUD-065).
set -eu
cd "$(dirname "$0")/.."
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

pass=0
fail=0
note() { echo "ok - $1"; pass=$((pass + 1)); }
bad() { echo "FAIL - $1"; fail=$((fail + 1)); }

# --- AUD-059: real Ed25519 signer + cryptographic evidence sealing (M1) ---
out=$(cargo test -p nexus-supply-chain 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "${n:-0}" -ge 50 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "nexus-supply-chain suite ($n tests: ArtifactSigner keygen/sign/verify, deterministic RFC 8032, fail-closed typed errors)"
else
  bad "nexus-supply-chain suite"
  echo "$out" | tail -25
fi

# --- AUD-059: sign/verify adapter + EP-039 SBOM crypto gates (M1/M2) ---
if sh scripts/ep039-m1-tests.sh >/tmp/rx009-ep039-m1.log 2>&1; then
  note "EP-039 M1 gate green (dependency fence + signer surface)"
else
  bad "EP-039 M1 gate"
  tail -20 /tmp/rx009-ep039-m1.log
fi
if sh scripts/ep039-m4-tests.sh >/tmp/rx009-ep039-m4.log 2>&1; then
  note "EP-039 M4 gate green (forced failures: tamper, reseal, pubkey swap, ecosystems tamper)"
else
  bad "EP-039 M4 gate"
  tail -20 /tmp/rx009-ep039-m4.log
fi
if sh scripts/ep039-m5-tests.sh >/tmp/rx009-ep039-m5.log 2>&1; then
  note "EP-039 M5 gate green (crypto-sealed SBOM evidence + ecosystems inventory)"
else
  bad "EP-039 M5 gate"
  tail -20 /tmp/rx009-ep039-m5.log
fi

# --- AUD-065: vitest bundle proofs with REAL signatures (M3) ---
out=$( (cd tests/release && node_modules/.bin/vitest run --config vitest.bundle.config.ts) 2>&1 | sed 's/\x1b\[[0-9;]*m//g' || true)
n=$(echo "$out" | grep -Eo "Tests +[0-9]+ passed" | grep -Eo "[0-9]+" | head -1)
if [ "${n:-0}" -ge 16 ] && ! echo "$out" | grep -qE "failed"; then
  note "vitest bundle proofs ($n tests: produce/verify/missing/changed/malformed/duplicate/traversal/symlink/wrong-release/tamper/self-digest/offline-install/rollback/evidence with real Ed25519 component signatures)"
else
  bad "vitest bundle proofs"
  echo "$out" | tail -25
fi

# --- AUD-065: typecheck clean (offline-bundle + tests/release) ---
if (cd tests/release && node_modules/.bin/tsc --noEmit -p tsconfig.json) >/tmp/rx009-tsc.log 2>&1; then
  note "typecheck clean (offline-bundle + tests/release)"
else
  bad "typecheck"
  tail -20 /tmp/rx009-tsc.log
fi

# --- AUD-065: hostile signature proofs via the real bundle CLI (M3) ---
RUN_ID="rx009-$(date +%s)"
FIXTURE_BASE="/tmp/nexus-rx009-hostile-${RUN_ID}"
BUNDLE_DIR="$FIXTURE_BASE/bundle"
mkdir -p "$FIXTURE_BASE/artifacts"
printf 'nexus-core-v1-real-bytes' > "$FIXTURE_BASE/artifacts/comp-1"
printf 'nexus-model-v2-real-bytes' > "$FIXTURE_BASE/artifacts/comp-2"
C1_DIGEST=$(sha256sum "$FIXTURE_BASE/artifacts/comp-1" | cut -d' ' -f1)
C2_DIGEST=$(sha256sum "$FIXTURE_BASE/artifacts/comp-2" | cut -d' ' -f1)
node -e "
const {generateKeyPairSync, sign: cryptoSign} = require('crypto');
const fs = require('fs');
const {publicKey, privateKey} = generateKeyPairSync('ed25519');
fs.writeFileSync('$FIXTURE_BASE/signing-key.pub.jwk', JSON.stringify(publicKey.export({format:'jwk'})));
const doSign = (digestHex) => cryptoSign(null, Buffer.from('sha256:' + digestHex), privateKey).toString('base64');
fs.writeFileSync('$FIXTURE_BASE/c1.sig', doSign('$C1_DIGEST'));
fs.writeFileSync('$FIXTURE_BASE/c2.sig', doSign('$C2_DIGEST'));
"
C1_SIG=$(cat "$FIXTURE_BASE/c1.sig")
C2_SIG=$(cat "$FIXTURE_BASE/c2.sig")
[ ${#C1_SIG} -gt 60 ] || bad "real c1 signature not produced"
[ ${#C2_SIG} -gt 60 ] || bad "real c2 signature not produced"
cat > "$FIXTURE_BASE/manifest.json" <<EOF
{
  "schema_version": 1,
  "release_id": "release-1",
  "version": "1.0.0",
  "channel": "STABLE",
  "components": [
    {"component_id":"comp-1","name":"component-comp-1","version":"1.0.0","artifact_ref":{"backend":"local","key":"artifact-comp-1"},"digest":"sha256:${C1_DIGEST}","signature":{"algorithm":"ED25519","key_id":"key-test-1","value_b64":"${C1_SIG}"},"sbom_ref":{"backend":"local","key":"sbom-comp-1"},"license_ref":"MIT","size_bytes":24},
    {"component_id":"comp-2","name":"component-comp-2","version":"2.0.0","artifact_ref":{"backend":"local","key":"artifact-comp-2"},"digest":"sha256:${C2_DIGEST}","signature":{"algorithm":"ED25519","key_id":"key-test-1","value_b64":"${C2_SIG}"},"sbom_ref":{"backend":"local","key":"sbom-comp-2"},"license_ref":"MIT","size_bytes":25}
  ],
  "compatibility": {
    "matrix_id": "matrix-1",
    "schema_version": 1,
    "entries": [
      {"component_id":"comp-1","version":"1.0.0","min_version":"1.0.0","max_version":"1.9.9","supported_profiles":["MANAGED","BYOC","EXISTING_SSH","HYBRID","FULLY_LOCAL"]},
      {"component_id":"comp-2","version":"2.0.0","min_version":"2.0.0","max_version":"2.9.9","supported_profiles":["MANAGED","BYOC","EXISTING_SSH","HYBRID","FULLY_LOCAL"]}
    ]
  },
  "sbom_ref": {"backend":"local","key":"sbom-root"},
  "license_refs": ["MIT"],
  "created_at": "2026-08-25T00:00:00Z"
}
EOF
node -e "
const {createHash}=require('crypto');
const fs=require('fs');
const p='$FIXTURE_BASE/manifest.json';
const obj=JSON.parse(fs.readFileSync(p,'utf8'));
const {manifest_digest,...rest}=obj;
obj.manifest_digest='sha256:'+createHash('sha256').update(JSON.stringify(rest)).digest('hex');
fs.writeFileSync(p, JSON.stringify(obj));
"
printf '{"bomFormat":"CycloneDX","version":1}' > "$FIXTURE_BASE/sbom.json"
printf 'MIT License text (fixture)' > "$FIXTURE_BASE/LICENSE"
printf 'ALTER TABLE nexus ADD COLUMN offline int;' > "$FIXTURE_BASE/migration-1.sql"
printf '#!/bin/sh\necho recover\n' > "$FIXTURE_BASE/recover.sh"
chmod +x "$FIXTURE_BASE/recover.sh"
log="$FIXTURE_BASE/produce.log"
sh offline-bundle/scripts/bundle-produce.sh \
  "$BUNDLE_DIR" "bundle-rx009" "release-1" \
  "$FIXTURE_BASE/manifest.json" \
  "comp-1=$FIXTURE_BASE/artifacts/comp-1:IMAGE,comp-2=$FIXTURE_BASE/artifacts/comp-2:MODEL" \
  "sbom.json=$FIXTURE_BASE/sbom.json" \
  "LICENSE=$FIXTURE_BASE/LICENSE" \
  "migration-1.sql=$FIXTURE_BASE/migration-1.sql" \
  "recover.sh=$FIXTURE_BASE/recover.sh" \
  "$FIXTURE_BASE/signing-key.pub.jwk" >"$log" 2>&1 \
  || bad "bundle-produce.sh failed"
[ -f "$BUNDLE_DIR/signing-key.pub.jwk" ] || bad "signing key missing from bundle"
if [ -f "$BUNDLE_DIR/signing-key.pub.jwk" ] && sh offline-bundle/scripts/bundle-verify.sh "$BUNDLE_DIR" >"$FIXTURE_BASE/verify-ok.log" 2>&1; then
  note "bundle with real signatures verifies (VERIFIED)"
else
  bad "bundle with real signatures verifies"
fi
# Hostile 1: dummy signature fails closed.
cp -r "$BUNDLE_DIR" "$FIXTURE_BASE/bundle-dummy"
node -e "
const fs=require('fs');
const p='$FIXTURE_BASE/bundle-dummy/release-manifest.json';
const obj=JSON.parse(fs.readFileSync(p,'utf8'));
obj.components[0].signature.value_b64='AAAA01BBBB01';
const {manifest_digest,...rest}=obj;
obj.manifest_digest='sha256:'+require('crypto').createHash('sha256').update(JSON.stringify(rest)).digest('hex');
fs.writeFileSync(p, JSON.stringify(obj));
"
if sh offline-bundle/scripts/bundle-verify.sh "$FIXTURE_BASE/bundle-dummy" >"$FIXTURE_BASE/verify-dummy.log" 2>&1; then
  bad "dummy signature must fail closed"
else
  grep -q "SIGNATURE_INVALID" "$FIXTURE_BASE/verify-dummy.log" \
    && note "dummy/placeholder signature fails closed (SIGNATURE_INVALID)" \
    || bad "dummy signature not classified SIGNATURE_INVALID"
fi
# Hostile 2: wrong key signature fails closed.
cp -r "$BUNDLE_DIR" "$FIXTURE_BASE/bundle-wrongkey"
node -e "
const {generateKeyPairSync, sign: cryptoSign}=require('crypto');
const fs=require('fs');
const p='$FIXTURE_BASE/bundle-wrongkey/release-manifest.json';
const obj=JSON.parse(fs.readFileSync(p,'utf8'));
const comp=obj.components[0];
const {publicKey, privateKey}=generateKeyPairSync('ed25519');
comp.signature.value_b64=cryptoSign(null, Buffer.from(comp.digest), privateKey).toString('base64');
const {manifest_digest,...rest}=obj;
obj.manifest_digest='sha256:'+require('crypto').createHash('sha256').update(JSON.stringify(rest)).digest('hex');
fs.writeFileSync(p, JSON.stringify(obj));
"
if sh offline-bundle/scripts/bundle-verify.sh "$FIXTURE_BASE/bundle-wrongkey" >"$FIXTURE_BASE/verify-wrongkey.log" 2>&1; then
  bad "wrong-key signature must fail closed"
else
  grep -q "SIGNATURE_INVALID" "$FIXTURE_BASE/verify-wrongkey.log" \
    && note "wrong-key signature fails closed (SIGNATURE_INVALID)" \
    || bad "wrong-key signature not classified SIGNATURE_INVALID"
fi
# Hostile 3: missing signing key fails closed.
cp -r "$BUNDLE_DIR" "$FIXTURE_BASE/bundle-nokey"
rm -f "$FIXTURE_BASE/bundle-nokey/signing-key.pub.jwk"
if sh offline-bundle/scripts/bundle-verify.sh "$FIXTURE_BASE/bundle-nokey" >"$FIXTURE_BASE/verify-nokey.log" 2>&1; then
  bad "missing signing key must fail closed"
else
  grep -q "SIGNING_KEY_MISSING" "$FIXTURE_BASE/verify-nokey.log" \
    && note "missing signing key fails closed (SIGNING_KEY_MISSING)" \
    || bad "missing signing key not classified SIGNING_KEY_MISSING"
fi
rm -rf "$FIXTURE_BASE"

# --- workspace check + clippy ---
if cargo check --workspace >/tmp/rx009-check.log 2>&1; then
  note "workspace check clean"
else
  bad "workspace check (see /tmp/rx009-check.log)"
fi
if cargo clippy --workspace --all-targets --all-features --locked -- -D warnings >/tmp/rx009-clippy.log 2>&1; then
  note "workspace clippy clean (-D warnings)"
else
  bad "clippy (see /tmp/rx009-clippy.log)"
fi

# --- remediation register must pass (90/90, quarantine active) ---
if reg=$(bash .agent/remediation/verify-remediation-register.sh 2>&1); then
  note "remediation register: $(echo "$reg" | tail -1)"
else
  bad "remediation register"
fi

echo "---"
echo "RX-009 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
