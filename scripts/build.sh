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
if [ -f apps/mobile/pubspec.yaml ]; then (cd apps/mobile && flutter build apk --debug --no-pub); fi
echo "build: ok"
