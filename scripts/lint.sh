#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
python3 scripts/blueprint_validate.py >/dev/null
if [ -f Cargo.toml ]; then cargo clippy --workspace --all-targets --all-features --locked -- -D warnings; fi
if [ -f pnpm-lock.yaml ]; then pnpm -r lint; fi
if [ -f pyproject.toml ]; then uv run --frozen ruff check python tests; fi
if [ -f apps/mobile/pubspec.yaml ]; then (cd apps/mobile && flutter analyze --no-pub); fi
echo "lint: ok"
