#!/usr/bin/env sh
# EP-027 M4 gate: forced failures, abuse cases, and observability.
#
# The M4 changed-file fence is connectors/hylafax/ (forced-failure and
# observability tests), infra/fax/ (operations diagnostic + bounded
# recovery), the gate itself, the node script, plan files, and
# evidence. The authoritative gate is:
#   - the nexus-hylafax host suite (unit tests; live tests are
#     env-gated and expected to skip on the host);
#   - the in-fixture LIVE failure suite (ep027_failure_*) exercising
#     REAL failure mechanisms: server process termination, policy
#     denial with spool proof, credential rejection with redaction
#     canaries. Runs sequentially (--test-threads=1) because the
#     hfaxd-down test mutates shared fixture state.
#
# Vacuity guards are required (EP-001 gate-masking class): a green M4
# must observe real non-zero passing counts, the nexus-hylafax
# failure test names, zero skipped live tests, hfaxd RUNNING at the
# end (bounded recovery), and zero new jobs (zero-orphan).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

FIXTURE="nexus-hylafax-fixture"
IMAGE="minichip/hylafax:latest"
DIGEST="sha256:00decb6c89fb4337534e9b4e82ff279cb53a492124bd083015cf82c354111613"
HF_HOST="${HYLAFAX_HOST:-172.17.0.3}"
HF_PORT="${HYLAFAX_PORT:-4559}"
HF_USER="${HYLAFAX_USER:-nexustest}"
HF_PASS="${HYLAFAX_PASS:-nexustest-pw}"
TC_BIN="/root/.rustup/toolchains/1.96.0-x86_64-unknown-linux-gnu/bin"

log="/tmp/ep027-m4-tests.log"
clog="/tmp/ep027-m4-live.log"
: > "$log"
: > "$clog"

fail() {
  echo "EP-027 M4 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-027 M4 gate: $1"; }

# ---- Guard 0 / 0b: owned production sources exist -------------------
if [ ! -f connectors/hylafax/Cargo.toml ]; then
  fail "connectors/hylafax/Cargo.toml missing"
fi
for f in src/transport.rs src/adapter.rs src/observability.rs src/lib.rs tests/failure_hylafax.rs; do
  if [ ! -f "connectors/hylafax/$f" ]; then
    fail "connectors/hylafax/$f missing"
  fi
done
if [ ! -f infra/fax/hylafax-diag.sh ]; then
  fail "infra/fax/hylafax-diag.sh (operations diagnostic) missing"
fi

# ---- Host workspace suite (unit tests; live tests skip here) --------
if ! cargo test --locked -p nexus-hylafax --all-targets >>"$log" 2>&1; then
  fail "host cargo test -p nexus-hylafax --all-targets failed" "$log"
fi
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  fail "host suite ran no tests (vacuity guard)" "$log"
fi
unit_passed=$(grep -oE 'test result: ok\. [0-9]+ passed' "$log" \
  | head -1 | grep -oE '[0-9]+' || true)
if [ -z "$unit_passed" ] || [ "$unit_passed" -lt 10 ]; then
  fail "nexus-hylafax unit floor (10) not met (got ${unit_passed:-0})" "$log"
fi
ok "host suite green ($unit_passed unit tests)"

# ---- Fixture provisioning (idempotent) ------------------------------
sh infra/hylafax/provision-fixture.sh >/dev/null 2>&1 || {
  echo "EP-027 M4 gate: FAIL - fixture provisioning" >&2
  sh infra/hylafax/provision-fixture.sh >&2
  exit 1
}

# ---- Guard 1: exact pinned fixture image started --------------------
if [ "$(docker inspect "$FIXTURE" | python3 -c 'import json,sys; print(json.load(sys.stdin)[0]["State"]["Running"])')" != "True" ]; then
  fail "fixture container not running"
fi
repo_digest="$(docker image inspect "$IMAGE" | python3 -c 'import json,sys; print(json.load(sys.stdin)[0]["RepoDigests"][0])')"
case "$repo_digest" in
  *"@$DIGEST") ok "fixture image pinned ($DIGEST)" ;;
  *) fail "fixture image digest mismatch: $repo_digest (expected $DIGEST)" ;;
esac

# ---- Guards 2+3: runtime version observed, hfaxd reachable ----------
banner="$(docker exec "$FIXTURE" python3 -c "
import socket
s = socket.socket(); s.settimeout(5)
s.connect(('$HF_HOST', $HF_PORT))
print(s.recv(256).decode('latin1').strip())
s.close()
")"
case "$banner" in
  *"Version 6.0.6"*) ok "HylaFAX runtime version 6.0.6 observed in greeting" ;;
  *) fail "HylaFAX runtime version not observed in greeting: $banner" ;;
esac

# ---- Guard 4: faxq live ---------------------------------------------
if ! docker exec "$FIXTURE" sh -c 'pgrep -x faxq >/dev/null'; then
  fail "faxq not running in fixture"
fi
ok "faxq live"

# ---- Sync connector source (derived from repo) ----------------------
docker cp "$REPO_ROOT/connectors/hylafax/src/." "$FIXTURE:/build/hylafax/src/" >/dev/null
docker cp "$REPO_ROOT/connectors/hylafax/tests/." "$FIXTURE:/build/hylafax/tests/" >/dev/null

# ---- Zero-orphan baseline -------------------------------------------
before="$(docker exec "$FIXTURE" sh -c 'ls /var/spool/hylafax/sendq/ | grep "^q" | sort')"

# ---- In-fixture LIVE failure suite (sequential) ---------------------
if ! docker exec "$FIXTURE" sh -c "
  export PATH=\"$TC_BIN:\$PATH\"
  cd /build/hylafax
  HYLAFAX_LIVE=1 HYLAFAX_HOST='$HF_HOST' HYLAFAX_PORT='$HF_PORT' \
  HYLAFAX_USER='$HF_USER' HYLAFAX_PASS='$HF_PASS' \
  cargo test --test failure_hylafax -- --nocapture --test-threads=1
" >>"$clog" 2>&1; then
  fail "in-fixture failure suite failed" "$clog"
fi

after="$(docker exec "$FIXTURE" sh -c 'ls /var/spool/hylafax/sendq/ | grep "^q" | sort')"
tmpb="$(mktemp)"
tmpa="$(mktemp)"
printf '%s\n' "$before" | grep -v '^$' | sort >"$tmpb"
printf '%s\n' "$after" | grep -v '^$' | sort >"$tmpa"
new_jobs="$(comm -13 "$tmpb" "$tmpa")"
rm -f "$tmpb" "$tmpa"
new_count="$(printf '%s\n' "$new_jobs" | grep -v '^$' | wc -l | tr -d ' ')"
if [ "$new_count" -ne 0 ]; then
  fail "zero-orphan violated: failure suite created jobs: $new_jobs" "$clog"
fi
ok "zero-orphan teardown (0 new jobs)"

# ---- Guard: hfaxd RUNNING at the end (bounded recovery) -------------
if ! docker exec "$FIXTURE" sh -c 'pgrep -x hfaxd >/dev/null'; then
  fail "hfaxd not running after failure suite (recovery failed)" "$clog"
fi
if ! docker exec "$FIXTURE" python3 -c "
import socket
s = socket.socket(); s.settimeout(5)
s.connect(('$HF_HOST', $HF_PORT)); s.close()
"; then
  fail "hfaxd not reachable after failure suite" "$clog"
fi
ok "hfaxd running and reachable after recovery"

# ---- Vacuity + reality guards on the live output --------------------
if grep -q 'skipping live hylafax' "$clog"; then
  fail "a required live failure test was skipped (HYLAFAX_LIVE not honored)" "$clog"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed; 0 failed' "$clog"; then
  fail "no passing non-vacuous live failure result (vacuity guard)" "$clog"
fi
fail_line=$(awk '/Running tests\/failure_hylafax.rs/{found=1} found && /test result: ok\. [0-9]+ passed/{print; exit}' "$clog")
fail_passed=$(printf '%s\n' "$fail_line" | grep -oE '[0-9]+' | head -1 || true)
if [ -z "$fail_passed" ] || [ "$fail_passed" -lt 3 ]; then
  fail "live failure floor (3) not met (got ${fail_passed:-0})" "$clog"
fi
ok "in-fixture failure suite green ($fail_passed live failure tests)"

# Anti-masking name guard: the sentinel failure test names unique to
# nexus-hylafax must be observed; running only nexus-fax/nexus-ictfax
# suites would fail here (EP-001 masking class). With --nocapture the
# harness prints the test name line and the pass marker separately, so
# the guard matches the test-name line; the result line above already
# proved all tests passed (0 failed).
for sentinel in \
  'test ep027_failure_hfaxd_down_truthful_unavailable' \
  'test ep027_failure_policy_denied_zero_mutation' \
  'test ep027_failure_redaction_canaries'; do
  if ! grep -Fq "$sentinel" "$clog"; then
    fail "anti-masking guard: missing sentinel '$sentinel'" "$clog"
  fi
done
ok "anti-masking: all nexus-hylafax failure sentinels observed"

# Real failure mechanisms observed in the output.
if ! grep -Fq 'hfaxd down -> Unavailable' "$clog"; then
  fail "server-termination failure path not observed" "$clog"
fi
if ! grep -Fq 'policy denial zero provider mutation' "$clog"; then
  fail "policy-denial zero-mutation path not observed" "$clog"
fi
if ! grep -Fq 'redaction canaries clean' "$clog"; then
  fail "redaction canary scan not observed" "$clog"
fi
ok "real failure mechanisms observed (termination / policy denial / credential rejection)"

echo "EP-027 M4: ok"
