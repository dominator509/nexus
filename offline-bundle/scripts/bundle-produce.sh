#!/usr/bin/env sh
# EP-042 M5 real offline bundle production (SPEC-016 behavior 5,
# SPEC-024). Produces a REAL bundle from REAL files: validated release
# manifest + real artifact/sbom/license/migration/recovery payloads
# copied with real sha256 digests into a bundle directory.
#
# Usage:
#   bundle-produce.sh <bundle-dir> <bundle-id> <release-id> \
#     <manifest.json> \
#     <componentId=artifactPath:kind[,componentId=artifactPath:kind...]> \
#     [sboms.csv] [licenses.csv] [migrations.csv] [recovery.csv] \
#     [signing-key.pub.jwk]
#
#   csv entries: name=path (sbom/license/migration/recovery)
#   signing-key.pub.jwk: optional path to the bundle signing public key
#     (JWK). When supplied, the key is copied into the bundle root so
#     verification can cryptographically check component signatures.
set -eu

BUNDLE_DIR="${1:?bundle dir required}"
BUNDLE_ID="${2:?bundle id required}"
RELEASE_ID="${3:?release id required}"
MANIFEST="${4:?manifest required}"
ARTIFACTS="${5:?artifacts csv required}"
SBOMS="${6:-}"
LICENSES="${7:-}"
MIGRATIONS="${8:-}"
RECOVERY="${9:-}"
SIGNING_KEY="${10:-}"

rm -rf "$BUNDLE_DIR"
mkdir -p "$BUNDLE_DIR"

node --experimental-transform-types --experimental-loader "$(cd "$(dirname "$0")" && pwd)/ts-resolve-loader.mjs" \
  "$(cd "$(dirname "$0")/.." && pwd)/src/cli.ts" produce \
  --bundle-dir "$BUNDLE_DIR" \
  --bundle-id "$BUNDLE_ID" \
  --release-id "$RELEASE_ID" \
  --manifest "$MANIFEST" \
  --artifacts "$ARTIFACTS" \
  --sboms "$SBOMS" \
  --licenses "$LICENSES" \
  --migrations "$MIGRATIONS" \
  --recovery "$RECOVERY"

# The signing public key rides inside the bundle (AUD-065): verification
# reads it from the bundle root, so an attacker cannot swap the key
# without breaking the bundle's own digest binding.
if [ -n "$SIGNING_KEY" ]; then
  [ -f "$SIGNING_KEY" ] || { echo "bundle-produce: FAIL - signing key missing: $SIGNING_KEY" >&2; exit 1; }
  cp "$SIGNING_KEY" "$BUNDLE_DIR/signing-key.pub.jwk"
fi
