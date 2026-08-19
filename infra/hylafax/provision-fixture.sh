#!/usr/bin/env sh
# EP-027 M3 fixture provisioning (idempotent, controlled test fixture).
#
# Bootstraps the nexus-hylafax-fixture container from the repo:
#   1. container created/started from the PINNED image digest
#      (minichip/hylafax@sha256:00decb6c...) if absent;
#   2. fixture credential ensured (nexustest user, known TEST-ONLY
#      password, crypt hash in hosts.hfaxd) - hfaxd restarted only
#      when the hash changed;
#   3. host Rust toolchain (1.96.0) copied in if missing (the fixture
#      is Ubuntu 18.04 / GLIBC 2.27 and cannot run host binaries;
#      this is a CONTROLLED_TEST_FIXTURE EXECUTION CONSTRAINT, not a
#      Nexus product requirement);
#   4. /build workspace derived from the repo (never hand-assembled):
#      fixture workspace manifest + crates with only path rewrites for
#      the in-container layout.
#
# The fixture credential is test-only and never appears in evidence.
set -eu

FIXTURE="nexus-hylafax-fixture"
IMAGE_DIGEST="sha256:00decb6c89fb4337534e9b4e82ff279cb53a492124bd083015cf82c354111613"
HF_PORT="${HYLAFAX_PORT:-4559}"
HF_USER="${HYLAFAX_USER:-nexustest}"
HF_PASS="${HYLAFAX_PASS:-nexustest-pw}"
TC_BIN="/root/.rustup/toolchains/1.96.0-x86_64-unknown-linux-gnu/bin"
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

echo "provision-fixture: begin"

# 1. Container from the pinned digest.
if ! docker inspect "$FIXTURE" >/dev/null 2>&1; then
  echo "provision-fixture: creating $FIXTURE from pinned digest"
  docker run -d --name "$FIXTURE" "minichip/hylafax@$IMAGE_DIGEST" >/dev/null
fi
if [ "$(docker inspect "$FIXTURE" | python3 -c 'import json,sys; print(json.load(sys.stdin)[0]["State"]["Running"])')" != "True" ]; then
  echo "provision-fixture: starting $FIXTURE"
  docker start "$FIXTURE" >/dev/null
fi

# Wait for hfaxd to accept connections.
i=0
until docker exec "$FIXTURE" python3 -c "
import socket
s = socket.socket(); s.settimeout(1)
try:
    s.connect(('127.0.0.1', $HF_PORT)); s.close()
except Exception:
    raise SystemExit(1)
"; do
  i=$((i + 1))
  [ "$i" -ge 30 ] && { echo "provision-fixture: FAIL - hfaxd not reachable" >&2; exit 1; }
  sleep 1
done
echo "provision-fixture: hfaxd reachable on $HF_PORT"

# 2. Fixture credential ensure. The wildcard hosts.hfaxd entry must
#    carry the crypt hash of the known TEST-ONLY password; otherwise
#    the real 530 path cannot be exercised (localhost auto-auths via
#    the empty-password host entry).
EXPECTED_HASH="$(printf '%s' "$HF_PASS" | docker exec -i "$FIXTURE" python3 -c "
import crypt, sys
print(crypt.crypt(sys.stdin.read().strip(), 'HS'))
")"
CURRENT="$(docker exec "$FIXTURE" sh -c "grep '^$HF_USER@\*' /var/spool/hylafax/etc/hosts.hfaxd" 2>/dev/null || true)"
case "$CURRENT" in
  *"$EXPECTED_HASH"*)
    echo "provision-fixture: fixture credential already correct"
    ;;
  *)
    echo "provision-fixture: updating fixture credential"
    docker exec "$FIXTURE" sh -c "
      printf 'localhost:21::\n$HF_USER@*:21:$EXPECTED_HASH\n' > /var/spool/hylafax/etc/hosts.hfaxd
      chown uucp:uucp /var/spool/hylafax/etc/hosts.hfaxd
      pkill -x hfaxd 2>/dev/null || true
      sleep 1
      nohup /usr/sbin/hfaxd -i $HF_PORT >/dev/null 2>&1 &
      sleep 1
    "
    # hfaxd restart sanity.
    docker exec "$FIXTURE" sh -c "pgrep -x hfaxd >/dev/null" || {
      echo "provision-fixture: FAIL - hfaxd did not restart" >&2
      exit 1
    }
    ;;
esac

# 3. Toolchain bootstrap (idempotent).
if ! docker exec "$FIXTURE" test -x "$TC_BIN/cargo"; then
  echo "provision-fixture: copying host Rust toolchain into fixture"
  tar -C /root/.rustup/toolchains -cf - 1.96.0-x86_64-unknown-linux-gnu \
    | docker exec -i "$FIXTURE" sh -c 'mkdir -p /root/.rustup/toolchains && tar -C /root/.rustup/toolchains -xf -'
fi
docker exec "$FIXTURE" "$TC_BIN/rustc" --version >/dev/null

# 4. Workspace bootstrap (derived from repo; path rewrites only).
docker exec "$FIXTURE" mkdir -p /build
docker cp "$REPO_ROOT/infra/hylafax/fixture-workspace/Cargo.toml" "$FIXTURE:/build/Cargo.toml" >/dev/null
docker cp "$REPO_ROOT/crates/nexus-domain/." "$FIXTURE:/build/nexus-domain/" >/dev/null
docker cp "$REPO_ROOT/crates/nexus-fax/." "$FIXTURE:/build/nexus-fax/" >/dev/null
docker exec "$FIXTURE" sh -c "sed -i 's#../../crates/nexus-domain#../nexus-domain#' /build/nexus-fax/Cargo.toml"
docker cp "$REPO_ROOT/connectors/hylafax/." "$FIXTURE:/build/hylafax/" >/dev/null
docker exec "$FIXTURE" sh -c "
  sed -i 's#../../crates/nexus-fax#../nexus-fax#' /build/hylafax/Cargo.toml
  sed -i 's#../../crates/nexus-domain#../nexus-domain#' /build/hylafax/Cargo.toml
"

echo "provision-fixture: ok"
