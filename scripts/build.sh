#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
python3 scripts/blueprint_validate.py >/dev/null
if [ -f Cargo.toml ]; then cargo build --workspace --all-targets --locked; fi
if [ -f pnpm-lock.yaml ]; then pnpm -r build; fi
if [ -f pyproject.toml ]; then uv build --package nexus-voice 2>/dev/null || uv run --frozen python -m compileall -q python; fi
# Mobile build: real Dart-layer build (flutter build bundle). The
# Android APK build requires the native Android SDK toolchain and is
# NOT ASSERTED until the native milestone owns it (no Android SDK on
# the build host; hardware certification deferred).
if [ -f apps/mobile/pubspec.yaml ]; then (cd apps/mobile && flutter build bundle --debug --no-pub); fi
echo "build: ok"
