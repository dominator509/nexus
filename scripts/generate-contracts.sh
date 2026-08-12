#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
[ -f schemas/nexus-control-object.schema.json ] || { echo "contract generation: FAIL - schemas absent" >&2; exit 1; }
if [ -f packages/contracts/package.json ]; then pnpm --filter @nexus/contracts generate; fi
if [ -f crates/nexus-contracts/Cargo.toml ]; then cargo test -p nexus-contracts generated_contracts_match --locked; fi
echo "contract generation: ok"
