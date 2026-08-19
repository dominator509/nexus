#!/usr/bin/env sh
# EP-027 M3 gate: real HylaFAX connector suite with vacuity guards.
#
# The M3 changed-file fence is connectors/hylafax/ (+ infra/hylafax/
# fixture provisioning, the gate itself, the node script, plan files,
# evidence). The authoritative gate is:
#   - the nexus-hylafax host suite (unit tests; live tests are
#     env-gated and expected to skip on the host);
#   - the in-fixture LIVE suite against the real pinned HylaFAX
#     server (hfaxd + faxq), run inside the fixture netns where the
#     EPRT data listener is reachable by hfaxd;
#   - twelve reality guards (see below) + anti-masking name guard.
#
# Vacuity guards are required: `cargo test <filter>` exits 0 on a
# zero-match filter (EP-001 gate-masking class), so a green M3 must
# observe real non-zero passing counts, the nexus-hylafax-specific
# live test names, and zero skipped/filtered live tests.
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

log="/tmp/ep027-m3-tests.log"
clog="/tmp/ep027-m3-live.log"
: > "$log"
: > "$clog"

fail() {
  echo "EP-027 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-027 M3 gate: $1"; }

# ---- Guard 0 / 0b: owned production sources exist -------------------
if [ ! -f connectors/hylafax/Cargo.toml ]; then
  fail "connectors/hylafax/Cargo.toml missing"
fi
for f in src/transport.rs src/adapter.rs src/observability.rs src/lib.rs; do
  if [ ! -f "connectors/hylafax/$f" ]; then
    fail "connectors/hylafax/$f missing"
  fi
done
if [ ! -f scripts/ep027-m3-tests.sh ] || [ ! -f infra/hylafax/provision-fixture.sh ]; then
  fail "M3 gate or fixture provisioning script missing"
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
if [ -z "$unit_passed" ] || [ "$unit_passed" -lt 7 ]; then
  fail "nexus-hylafax unit floor (7) not met (got ${unit_passed:-0})" "$log"
fi
ok "host suite green ($unit_passed unit tests)"

# ---- Fixture provisioning (idempotent) ------------------------------
sh infra/hylafax/provision-fixture.sh >/dev/null 2>&1 || {
  echo "EP-027 M3 gate: FAIL - fixture provisioning" >&2
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

# ---- Sync connector source (derived from repo; not the manifest) ----
docker cp "$REPO_ROOT/connectors/hylafax/src/." "$FIXTURE:/build/hylafax/src/" >/dev/null
docker cp "$REPO_ROOT/connectors/hylafax/tests/." "$FIXTURE:/build/hylafax/tests/" >/dev/null

# ---- Guard 12: zero-orphan baseline ---------------------------------
before="$(docker exec "$FIXTURE" sh -c 'ls /var/spool/hylafax/sendq/ | grep "^q" | sort')"

# ---- In-fixture LIVE suite (HYLAFAX_LIVE=1; no skips allowed) -------
# --test-threads=1: the M4 failure binary (hfaxd-down test terminates
# the real hfaxd process) must not race other tests in the same run.
if ! docker exec "$FIXTURE" sh -c "
  export PATH=\"$TC_BIN:\$PATH\"
  cd /build/hylafax
  HYLAFAX_LIVE=1 HYLAFAX_HOST='$HF_HOST' HYLAFAX_PORT='$HF_PORT' \
  HYLAFAX_USER='$HF_USER' HYLAFAX_PASS='$HF_PASS' \
  cargo test -- --nocapture --test-threads=1
" >>"$clog" 2>&1; then
  fail "in-fixture live suite failed" "$clog"
fi

after="$(docker exec "$FIXTURE" sh -c 'ls /var/spool/hylafax/sendq/ | grep "^q" | sort')"
tmpb="$(mktemp)"
tmpa="$(mktemp)"
printf '%s\n' "$before" | grep -v '^$' | sort >"$tmpb"
printf '%s\n' "$after" | grep -v '^$' | sort >"$tmpa"
new_jobs="$(comm -13 "$tmpb" "$tmpa")"
rm -f "$tmpb" "$tmpa"
new_count="$(printf '%s\n' "$new_jobs" | grep -v '^$' | wc -l | tr -d ' ')"
if [ "$new_count" -ne 1 ]; then
  fail "zero-orphan violated: expected exactly 1 new evidence job, got $new_count: $new_jobs" "$clog"
fi
ok "zero-orphan teardown (exactly 1 evidence job: $new_jobs)"

# ---- Vacuity + reality guards on the live output --------------------
if grep -q 'skipping live hylafax' "$clog"; then
  fail "a required live test was skipped (HYLAFAX_LIVE not honored)" "$clog"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed; 0 failed' "$clog"; then
  fail "no passing non-vacuous live result (vacuity guard)" "$clog"
fi
in_unit_line=$(awk '/Running unittests src\/lib.rs/{found=1} found && /test result: ok\. [0-9]+ passed/{print; exit}' "$clog")
in_unit_passed=$(printf '%s\n' "$in_unit_line" | grep -oE '[0-9]+' | head -1 || true)
if [ -z "$in_unit_passed" ] || [ "$in_unit_passed" -lt 7 ]; then
  fail "in-fixture unit floor (7) not met (got ${in_unit_passed:-0})" "$clog"
fi
live_line=$(awk '/Running tests\/live_hylafax.rs/{found=1} found && /test result: ok\. [0-9]+ passed/{print; exit}' "$clog")
live_passed=$(printf '%s\n' "$live_line" | grep -oE '[0-9]+' | head -1 || true)
if [ -z "$live_passed" ] || [ "$live_passed" -lt 3 ]; then
  fail "live integration floor (3) not met (got ${live_passed:-0})" "$clog"
fi
ok "in-fixture suite green ($in_unit_passed unit + $live_passed live)"

# Anti-masking name guard: the sentinel live test names unique to
# nexus-hylafax must be observed; running only nexus-fax/nexus-ictfax
# suites would fail here (EP-001 masking class).
for sentinel in \
  'test ep027_live_hylafax_full_submission_round_trip' \
  'test ep027_live_hylafax_wrong_password_fails_closed' \
  'test ep027_live_hylafax_scheduler_nak_not_submitted'; do
  if ! grep -Fq "$sentinel" "$clog"; then
    fail "anti-masking guard: missing sentinel '$sentinel'" "$clog"
  fi
done
ok "anti-masking: all nexus-hylafax live sentinels observed"

# Guard 5: real authentication (round-trip auth + explicit 530).
if ! grep -qE 'carrier job id = [0-9]+' "$clog"; then
  fail "no real authenticated submission observed (carrier job id missing)" "$clog"
fi
if ! grep -Fq 'wrong password rejected: hylafax authentication failed (530)' "$clog"; then
  fail "real 530 wrong-password rejection not observed" "$clog"
fi
ok "real authentication (230 path) and real 530 rejection observed"

# Guard 6+7+8: EPRT data channel, MODE Z/STOT upload, JSUBM success are
# proven by the round-trip test passing: the in-test spool oracle
# asserts the stored docq artifact matches the uploaded digest
# byte-for-byte, which requires EPRT + MODE Z + STOT + JSUBM to have
# worked end-to-end.
if ! grep -Fq 'test ep027_live_hylafax_full_submission_round_trip' "$clog"; then
  fail "round-trip (EPRT/MODE Z/STOT/JSUBM/readback) not green" "$clog"
fi
ok "EPRT data channel + MODE Z/STOT upload + JSUBM + exact-target readback green"

# Guard 9: provider-assigned CarrierJobId observed + replay dedup.
if ! grep -Fq 'replay deduplicated to' "$clog"; then
  fail "idempotent replay dedup not observed" "$clog"
fi
ok "provider CarrierJobId observed; replay deduplicated"

# Guard 10: exact-target query_job/LIST readback (round-trip status()
# binds the exact carrier job id; in-test assertion).
if ! grep -Fq 'test ep027_live_hylafax_full_submission_round_trip' "$clog"; then
  fail "exact-target readback not green" "$clog"
fi

# Guard 11: wrong-auth/failure executed (already grepped above) + the
# scheduler NAK regression (real 460 observed).
if ! grep -Fq 'scheduler NAK' "$clog"; then
  fail "scheduler NAK regression not executed" "$clog"
fi
ok "wrong-auth and scheduler NAK failure paths executed (real 530 / 460)"

echo "EP-027 M3: ok"
