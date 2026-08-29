#!/usr/bin/env sh
# Canonical build. RX-004: BUILD GREEN != PRODUCTION ARTIFACT EXISTS.
# Every release-critical artifact gets an explicit build target and a
# post-build existence check. A missing required artifact is a FAIL;
# there is no silent fallback (the previous `uv build --package
# nexus-voice 2>/dev/null || compileall` masked the missing wheel).
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

fail() { echo "build: FAIL - $1" >&2; exit 1; }

python3 scripts/blueprint_validate.py >/dev/null

# 1. Rust workspace: release-critical control-plane and sidecar binaries.
if [ -f Cargo.toml ]; then
  cargo build --workspace --all-targets --locked
  for bin in target/debug/nexus-control-plane target/debug/nexus-sidecar; do
    [ -f "$bin" ] && [ -x "$bin" ] || fail "required artifact missing: $bin"
    echo "build: artifact ok $bin"
  done
fi

# 2. TypeScript/pnpm workspace: every app/package dist directory.
if [ -f pnpm-lock.yaml ]; then
  pnpm -r build
  for d in \
    apps/web/dist apps/desktop/dist apps/setup/dist \
    packages/contracts/dist packages/connector-sdk/dist packages/onboarding/dist \
    packages/ui/dist packages/workflows/dist; do
    [ -d "$d" ] && [ -n "$(ls -A "$d" 2>/dev/null)" ] || fail "required artifact missing: $d"
    echo "build: artifact ok $d"
  done
fi

# 3. Python: build the real workspace package (nexus-contracts wheel covers
#    python/nexus_contracts + python/nexus_microbrain). No compileall
#    fallback: a wheel/sdist must exist after uv build.
if [ -f pyproject.toml ]; then
  uv build
  wheel=$(ls dist/nexus_contracts-*.whl 2>/dev/null || true)
  sdist=$(ls dist/nexus_contracts-*.tar.gz 2>/dev/null || true)
  [ -n "$wheel" ] || fail "required artifact missing: dist/nexus_contracts-*.whl"
  [ -n "$sdist" ] || fail "required artifact missing: dist/nexus_contracts-*.tar.gz"
  echo "build: artifact ok $wheel"
  echo "build: artifact ok $sdist"
fi

# 4. Mobile: real Dart-layer build (flutter build bundle). The Android APK
#    build requires the native Android SDK toolchain and remains NOT
#    ASSERTED until the native milestone owns it; that unavailability is
#    advertised, never silently certified.
if [ -f apps/mobile/pubspec.yaml ]; then
  (cd apps/mobile && flutter build bundle --debug --no-pub)
  [ -d apps/mobile/build/flutter_assets ] && [ -n "$(ls -A apps/mobile/build/flutter_assets 2>/dev/null)" ] \
    || fail "required artifact missing: apps/mobile/build/flutter_assets"
  echo "build: artifact ok apps/mobile/build/flutter_assets"
fi

echo "build: ok"
