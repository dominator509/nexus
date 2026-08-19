#!/usr/bin/env sh
# EP-026 M4 mail fixture teardown. Idempotent; every failure surfaced.
set -eu

STATE="/tmp/ep026-mail-stack-state.json"
# Remove EVERY ep026-mail-* container (hygiene: leaked stacks from
# interrupted runs must not survive).
for NAME in $(docker ps -aq --filter "name=ep026-mail-" || true); do
  docker rm -f "$NAME" >/dev/null 2>&1 || true
done
rm -f "$STATE" /tmp/ep026-mail.env
echo "ep026 mail fixture: torn down"
