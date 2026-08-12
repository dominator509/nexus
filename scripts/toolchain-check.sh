#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
fail() { echo "toolchain check: FAIL - $1" >&2; exit 1; }
for tool in git sh awk grep sed python3 curl openssl; do
  command -v "$tool" >/dev/null 2>&1 || fail "missing bootstrap tool $tool"
done
if sh scripts/stage.sh at-least EP-000; then
  for tool in docker jq rustc cargo node corepack pnpm uv flutter tofu age sops cargo-deny cargo-audit; do
    command -v "$tool" >/dev/null 2>&1 || fail "missing locked tool $tool; run sh scripts/install.sh"
  done
  rustc --version | grep -F '1.97.1' >/dev/null || fail 'Rust must be 1.97.1'
  node --version | grep -E '^v24\.' >/dev/null || fail 'Node must be 24 LTS'
  pnpm --version | grep -F '11.17.0' >/dev/null || fail 'pnpm must be 11.17.0'
  uv --version | grep -F '0.12.0' >/dev/null || fail 'uv must be 0.12.0'
  flutter --version | grep -F '3.44.7' >/dev/null || fail 'Flutter must be 3.44.7'
  tofu version | grep -F '1.12.1' >/dev/null || fail 'OpenTofu must be 1.12.1'
  cargo-deny --version | grep -F '0.20.2' >/dev/null || fail 'cargo-deny must be 0.20.2 (CVSS 4.0 support; see VERSIONS.lock.yaml)'
  cargo-audit --version | grep -F '0.22.2' >/dev/null || fail 'cargo-audit must be 0.22.2 (see VERSIONS.lock.yaml)'
fi
echo "toolchain check: ok"
