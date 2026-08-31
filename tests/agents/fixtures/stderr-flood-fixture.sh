#!/usr/bin/env sh
# CONTROLLED_TEST_FIXTURE: stderr flood harness for RX-007 AUD-022.
#
# Writes ~360 KB to stderr (filling the 64 KB pipe buffer several
# times) while keeping stdout open, then prints a final marker and
# exits 0. Under the pre-RX-007 drain order (stdout to EOF before
# stderr, no timeout) this deadlocks the parent; the concurrent-drain
# ProcessRunner completes it normally.
set -eu
i=0
while [ $i -lt 20000 ]; do
  echo "stderr-line-$i" >&2
  i=$((i + 1))
done
echo "stdout-done"
exit 0
