#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
python3 scripts/blueprint_validate.py >/dev/null
if [ -f Cargo.toml ]; then cargo fmt --all -- --check; fi
if [ -f pnpm-lock.yaml ]; then pnpm exec prettier --check .; fi
if [ -f pyproject.toml ]; then uv run --frozen ruff format --check python tests; fi
if [ -f apps/mobile/pubspec.yaml ]; then dart format --output=none --set-exit-if-changed apps/mobile; fi
if [ -f packages/mobile-contracts/pubspec.yaml ]; then (cd packages/mobile-contracts && dart format --output=none --set-exit-if-changed lib test); fi
if [ -f tests/e2e/mobile/pubspec.yaml ]; then (cd tests/e2e/mobile && dart format --output=none --set-exit-if-changed lib test); fi
if [ -f tests/accessibility/mobile/pubspec.yaml ]; then (cd tests/accessibility/mobile && dart format --output=none --set-exit-if-changed lib test); fi
if [ -f tests/livefire/mobile/pubspec.yaml ]; then (cd tests/livefire/mobile && dart format --output=none --set-exit-if-changed lib test); fi
echo "format check: ok"
