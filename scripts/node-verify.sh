#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
node="${1:?node id}"
sh scripts/expected-files.sh "$node"
sh scripts/verify.sh
if [ -f "scripts/nodes/$node.sh" ]; then sh "scripts/nodes/$node.sh" verify; fi
echo "node verify $node: ok"
