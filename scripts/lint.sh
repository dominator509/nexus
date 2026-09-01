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
# Flutter package resolution first: `analyze --no-pub` on a fresh
# checkout has no .dart_tool/package_config.json and the analyzer
# reports every package symbol as undefined. Mirror the local closure
# ladder (install.sh runs pub get; ep034 runs analyze after deps
# resolved). pubspec.lock is committed, so pub get is deterministic.
# FLUTTER_BIN mirrors ep034-m1-tests.sh (mise shim; CI PATH also works).
FLUTTER="${FLUTTER_BIN:-mise exec flutter -- flutter}"
for mobile_dir in \
  apps/mobile \
  packages/mobile-contracts \
  tests/e2e/mobile \
  tests/accessibility/mobile \
  tests/livefire/mobile; do
  if [ -f "$mobile_dir/pubspec.yaml" ]; then
    (cd "$mobile_dir" && $FLUTTER pub get) >/dev/null 2>&1 || { echo "lint: FAIL - flutter pub get failed in $mobile_dir" >&2; exit 1; }
    (cd "$mobile_dir" && $FLUTTER analyze --no-pub)
  fi
done
echo "lint: ok"
