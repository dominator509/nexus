#!/usr/bin/env sh
set -eu
# Canonical environment: mise shims PATH + non-interactive exports.
. scripts/env.sh
node="${1:?node id}"
sh scripts/expected-files.sh "$node"
sh scripts/verify.sh
if [ -f "scripts/nodes/$node.sh" ]; then sh "scripts/nodes/$node.sh" verify; fi
echo "node verify $node: ok"
