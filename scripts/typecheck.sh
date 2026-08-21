#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
python3 scripts/blueprint_validate.py >/dev/null
if [ -f Cargo.toml ]; then cargo check --workspace --all-targets --all-features --locked; fi
if [ -f pnpm-lock.yaml ]; then pnpm -r typecheck; fi
if [ -f pyproject.toml ]; then uv run --frozen mypy python; fi
if [ -f apps/mobile/pubspec.yaml ]; then (cd apps/mobile && flutter analyze --no-pub); fi
if [ -f packages/mobile-contracts/pubspec.yaml ]; then (cd packages/mobile-contracts && flutter analyze --no-pub); fi
if [ -f tests/e2e/mobile/pubspec.yaml ]; then (cd tests/e2e/mobile && flutter analyze --no-pub); fi
if [ -f tests/accessibility/mobile/pubspec.yaml ]; then (cd tests/accessibility/mobile && flutter analyze --no-pub); fi
if [ -f tests/livefire/mobile/pubspec.yaml ]; then (cd tests/livefire/mobile && flutter analyze --no-pub); fi
echo "typecheck: ok"
