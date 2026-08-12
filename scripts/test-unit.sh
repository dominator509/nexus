#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
python3 scripts/blueprint_validate.py >/dev/null
if [ -f Cargo.toml ]; then cargo test --workspace --lib --bins --locked; fi
if [ -f pnpm-lock.yaml ]; then pnpm -r test:unit; fi
if [ -f pyproject.toml ] && [ -d tests/unit ]; then uv run --frozen pytest tests/unit -q; fi
if [ -f apps/mobile/pubspec.yaml ]; then (cd apps/mobile && flutter test test/unit); fi
echo "unit tests: ok"
