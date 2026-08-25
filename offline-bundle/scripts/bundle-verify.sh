#!/usr/bin/env sh
# EP-042 M5 real offline bundle verification (SPEC-016 behavior 5,
# SPEC-024). Digest-bound: every declared file must exist, every digest
# must match real bytes, the release manifest binding must hold, the
# release id must match, no duplicate/escape path, and the bundle
# self-digest must hold. Any denial exits nonzero with a typed class.
#
# Usage:
#   bundle-verify.sh <bundle-dir>
set -eu

BUNDLE_DIR="${1:?bundle dir required}"

node --experimental-transform-types --experimental-loader "$(cd "$(dirname "$0")" && pwd)/ts-resolve-loader.mjs" \
  "$(cd "$(dirname "$0")/.." && pwd)/src/cli.ts" verify \
  --bundle-dir "$BUNDLE_DIR"
