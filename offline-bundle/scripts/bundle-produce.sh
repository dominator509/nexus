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
#     [sboms.csv] [licenses.csv] [migrations.csv] [recovery.csv]
#
#   csv entries: name=path (sbom/license/migration/recovery)
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
