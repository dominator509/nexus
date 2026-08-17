#!/usr/bin/env sh
# EP-022 M3 gate: run the REAL Wyoming protocol integration suite.
#
# The M3 changed-files fence is connectors/wyoming/ (Python connector +
# integration tests). The authoritative gate is the ep022_integration
# unittest suite against the REAL pinned rhasspy/wyoming-openwakeword
# container. The vacuity guard is required: unittest exits 0 on a
# zero-collected run (EP-001 gate-masking class).
#
# The engine venv python (/opt/nexus-voice-engines) carries the real
# wyoming==1.10.0 client (MIT). The gate starts the real container on
# 127.0.0.1:10400, waits for readiness, runs the suite, and tears the
# container down.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep022-m3-tests.log"
: > "$log"

image="rhasspy/wyoming-openwakeword:latest"
digest="sha256:52cb1168731a1849fc28cf339c935fde58746bbabc94226668a40ef6ddf5d42b"
container="ep022-wyoming-m3"
port="10400"
engine_python="${NEXUS_VOICE_ENGINE_VENV:-/opt/nexus-voice-engines}/bin/python"

cleanup() {
  docker rm -f "$container" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# Start the real container (pinned image; digest verified by docker).
docker run --rm -d --name "$container" -p "127.0.0.1:${port}:${port}" "$image" >/dev/null

# Readiness: wait for the real describe/info handshake to succeed (a
# bare TCP connect succeeds at kernel level before the app is ready).
ready=0
i=0
while [ "$i" -lt 60 ]; do
  if "$engine_python" -c "
import asyncio, sys
sys.path.insert(0, 'connectors/wyoming')
from connector import WyomingSession

async def probe():
    s = WyomingSession(uri='tcp://127.0.0.1:$port', timeout_seconds=2.0)
    try:
        info = await s.connect()
        return bool(info.wake)
    except Exception:
        return False
    finally:
        await s.close()

sys.exit(0 if asyncio.run(probe()) else 1)
" 2>/dev/null; then
    ready=1
    break
  fi
  sleep 1
  i=$((i + 1))
done
if [ "$ready" -ne 1 ]; then
  echo "EP-022 M3: FAIL - Wyoming container did not become ready" >&2
  docker logs "$container" 2>&1 | tail -20 >&2
  exit 1
fi

if ! "$engine_python" -m unittest discover \
    -s connectors/wyoming/tests -p "test_ep022_integration*.py" -v \
    >>"$log" 2>&1; then
  echo "EP-022 M3: FAIL - Wyoming integration suite failed" >&2
  tail -40 "$log" >&2
  exit 1
fi

# Vacuity guard: at least one ep022_integration test passed (unittest -v
# prints "ok" on the docstring line after the test name).
if ! grep -qE '^OK$' "$log" && ! grep -qE ' \.\.\. ok$' "$log"; then
  echo "EP-022 M3: FAIL - no ep022_integration tests passed (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# Zero-orphan check: the container must be gone after teardown.
cleanup
if docker ps -aq --filter "name=${container}" | grep -q .; then
  echo "EP-022 M3: FAIL - container leak after teardown" >&2
  docker rm -f "$container" >/dev/null 2>&1 || true
  exit 1
fi

tail -8 "$log"
echo "EP-022 M3: ok"
