#!/usr/bin/env sh
# EP-027 M5 gate: live-fire, operations, and node closure.
#
# The M5 changed-file fence is tests/fax/ (LF-030 live-fire E2E crate),
# scripts/live-fire/LF-030.sh, scripts/ep027-m5-tests.sh, the node
# script, docs/operations/EP-027-fax.md, plan files, and evidence. The
# authoritative gate is:
#   - the nexus-fax-e2e host build (LF-030 test is env-gated);
#   - the in-fixture LF-030 live-fire: REAL governed submit -> carrier
#     job id -> exact-target readback -> spool oracle -> replay dedup
#     -> real 530 failure path, writing current-run machine-readable
#     evidence embedding EP027_M5_RUN_ID (stale evidence never
#     satisfies);
#   - evidence current-run + redaction + zero-orphan guards.
#
# Vacuity guards are required (EP-001 gate-masking class): a green M5
# must observe the LF-030 test name, the current-run evidence file, a
# matching run id, zero credential leakage, and exactly one new spool
# job (the LF-030 evidence job).
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
EVIDENCE=".agent/state/evidence/LF-030-ep027-m5.json"

log="/tmp/ep027-m5-tests.log"
clog="/tmp/ep027-m5-live.log"
: > "$log"
: > "$clog"

fail() {
  echo "EP-027 M5 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-027 M5 gate: $1"; }

# ---- Guard 0 / 0b: owned production sources exist -------------------
for f in tests/fax/Cargo.toml tests/fax/tests/lf030_live_fire.rs \
         scripts/live-fire/LF-030.sh docs/operations/EP-027-fax.md; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done

# ---- Host workspace suite (LF-030 test skips without HYLAFAX_LIVE) --
if ! cargo test --locked -p nexus-fax-e2e --all-targets >>"$log" 2>&1; then
  fail "host cargo test -p nexus-fax-e2e --all-targets failed" "$log"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed; 0 failed' "$log"; then
  fail "host e2e suite produced no passing result (vacuity guard)" "$log"
fi
ok "host nexus-fax-e2e suite green"

# ---- Fixture provisioning (idempotent) ------------------------------
sh infra/hylafax/provision-fixture.sh >/dev/null 2>&1 || {
  echo "EP-027 M5 gate: FAIL - fixture provisioning" >&2
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

# ---- Current-run evidence id (stale evidence never satisfies) -------
RUN_ID="ep027-m5-$(date +%s)-$$"
ok "run id $RUN_ID"

# ---- Zero-orphan baseline -------------------------------------------
before="$(docker exec "$FIXTURE" sh -c 'ls /var/spool/hylafax/sendq/ | grep "^q" | sort')"

# ---- In-fixture LF-030 live-fire ------------------------------------
if ! docker exec "$FIXTURE" sh -c "
  export PATH=\"$TC_BIN:\$PATH\"
  cd /build
  HYLAFAX_LIVE=1 HYLAFAX_HOST='$HF_HOST' HYLAFAX_PORT='$HF_PORT' \
  HYLAFAX_USER='$HF_USER' HYLAFAX_PASS='$HF_PASS' EP027_M5_RUN_ID='$RUN_ID' \
  cargo test -p nexus-fax-e2e -- --nocapture --test-threads=1
" >>"$clog" 2>&1; then
  fail "in-fixture LF-030 live-fire failed" "$clog"
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
  fail "zero-orphan violated: expected exactly 1 LF-030 evidence job, got $new_count: $new_jobs" "$clog"
fi
ok "zero-orphan teardown (exactly 1 LF-030 evidence job: $new_jobs)"

# ---- Anti-masking + no-skip guards on the live output ---------------
if grep -q 'skipping LF-030' "$clog"; then
  fail "LF-030 live-fire test was skipped (HYLAFAX_LIVE not honored)" "$clog"
fi
if ! grep -Fq 'test ep027_m5_lf030_lifecycle' "$clog"; then
  fail "anti-masking guard: LF-030 live-fire test did not run" "$clog"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed; 0 failed' "$clog"; then
  fail "no passing non-vacuous live-fire result (vacuity guard)" "$clog"
fi
ok "LF-030 live-fire test ran and passed"

# ---- Evidence: copy out + current-run + content guards --------------
docker cp "$FIXTURE:/build/.agent/state/evidence/LF-030-ep027-m5.json" "$EVIDENCE" >/dev/null
if [ ! -f "$EVIDENCE" ]; then
  fail "current-run evidence file missing after live-fire"
fi
if ! grep -Fq "$RUN_ID" "$EVIDENCE"; then
  fail "evidence run id mismatch (stale evidence)" "$EVIDENCE"
fi
for bad in "$HF_PASS" "wrong-password-canary"; do
  if grep -Fq "$bad" "$EVIDENCE"; then
    fail "credential leaked into evidence"
  fi
done
if ! grep -Fq '"delivered": "NOT_ASSERTED"' "$EVIDENCE"; then
  fail "evidence must record DELIVERED NOT_ASSERTED"
fi
if ! grep -Fq '"readback_state": "SUBMITTED"' "$EVIDENCE"; then
  fail "evidence must record SUBMITTED readback"
fi
ok "evidence current-run + redacted + honest ($EVIDENCE)"

echo "EP-027 M5: ok"
