#!/usr/bin/env sh
# AUD-086: real release build surface. The old script invoked the
# phantom nexus-cli package. The REAL release manifest producer is the
# release-evidence CLI (manifest command), which builds
# dist/release/RELEASE_MANIFEST.json from the REAL committed product
# artifacts with real sha256 digests (AUD-082). The manifest is then
# verified against those real artifact bytes.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p dist/release
node --experimental-transform-types \
  --import "file://$REPO_ROOT/release-evidence/scripts/ts-resolve-loader.mjs" \
  "$REPO_ROOT/release-evidence/src/cli.ts" manifest --output-dir dist/release
[ -f dist/release/RELEASE_MANIFEST.json ] || { echo "release build: FAIL - manifest absent" >&2; exit 1; }
node --experimental-transform-types \
  --import "file://$REPO_ROOT/release-evidence/scripts/ts-resolve-loader.mjs" \
  "$REPO_ROOT/release-evidence/src/cli.ts" verify-manifest --manifest dist/release/RELEASE_MANIFEST.json
echo "release build: ok"
