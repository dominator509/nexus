#!/usr/bin/env sh
# CONTROLLED_TEST_FIXTURE (TESTING.md): deterministic failing worker for
# the EP-019 self-healing integration suite.
#
# The worker is intentionally broken (the "incident"): it looks for its
# fix marker under the wrong hard-coded filename, so it crashes even when
# the operator provides the correct marker path. The EP-019 patch
# artifact fixes the filename check; reproduction before the patch FAILS
# and after the patch PASSES against the identical invocation.
set -eu

MARKER="${1:-missing}"
# BUG (the incident): the worker checks the hard-coded wrong filename
# instead of the marker path the operator passed.
if [ -f "worker.conf" ]; then
  printf 'worker: healthy\n'
  exit 0
fi

printf 'worker: crash (marker not found)\n' >&2
exit 1
