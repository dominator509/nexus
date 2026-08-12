#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
for compose in infra/compose/core.yaml infra/compose/full.yaml; do
  [ -f "$compose" ] || continue
  docker compose -f "$compose" down --remove-orphans
done
echo "local stop: ok"
