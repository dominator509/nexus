#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
fail() { echo "install: FAIL - $1" >&2; exit 1; }
for tool in git sh awk grep sed python3 curl openssl docker; do
  command -v "$tool" >/dev/null 2>&1 || fail "install bootstrap prerequisite $tool through the operating system package manager"
done
if [ -f infra/devcontainer/Dockerfile ]; then
  docker build --pull=false -f infra/devcontainer/Dockerfile -t nexus-devtoolchain:locked .
fi
if [ -f Cargo.toml ]; then cargo fetch --locked; fi
if [ -f pnpm-lock.yaml ]; then corepack enable; pnpm install --frozen-lockfile --offline; fi
if [ -f uv.lock ]; then uv sync --frozen --offline; fi
if [ -f apps/mobile/pubspec.lock ]; then (cd apps/mobile && flutter pub get --offline); fi
echo "install: ok"
