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
echo "format check: ok"
