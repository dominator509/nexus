#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
. scripts/env.sh

# LF-003 owner-passkey-onboarding (EP-007 M5; live-fire/REGISTRY.tsv).
#
# Proof: the REAL EP-007 live-fire tests in crates/nexus-auth/tests/lf003.rs
# (2 tests):
#   - ep007_live_fire_owner_passkey_onboarding
#   - ep007_live_fire_step_up_action_digest_and_fail_closed
# These prove the canonical human owner-passkey lifecycle through the
# nexus-auth contracts with real state machines (challenge issuance,
# passkey enrollment, credential assertion, STEP_UP session, revocation,
# audit records) and the STEP_UP action-digest fail-closed invariant.
#
# This script was rewritten from a stub that delegated to a nonexistent
# `nexus-cli` proof runner (EP-006 M5 precedent: LF-017.sh). The live-fire
# gate runs every proof owned by a DONE node; the stub broke `verify` for
# every node after EP-007. The REAL proof is the committed Rust test suite
# below - no mocks, no stubs.

log="/tmp/lf003-cargo.log"
: > "$log"

cargo test --locked -p nexus-auth --test lf003 >>"$log" 2>&1 || {
  echo "LF-003: FAIL - live-fire cargo test run failed" >&2
  tail -40 "$log" >&2
  exit 1
}

# Vacuity guard: at least two passing tests must appear in the summary
# (cargo exits 0 even when the filter matches nothing - the EP-001
# gate-masking class).
if ! grep -E 'test result: ok\.[[:space:]]+2 passed' "$log" | grep -q '2 passed'; then
  echo "LF-003: FAIL - expected 2 passing live-fire tests (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

evidence=".agent/state/evidence/LF-003-ep007-m5.md"
if [ ! -f "$evidence" ]; then
  {
    echo "# LF-003 Owner Passkey Onboarding (EP-007 M5)"
    echo
    echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "Node: EP-007"
    echo "Command: sh scripts/live-fire/LF-003.sh"
    echo
    echo "## Real proof (no mocks, no in-memory engine)"
    echo "- ep007_live_fire_owner_passkey_onboarding: REAL passkey"
    echo "  enrollment + assertion lifecycle through nexus-auth contracts"
    echo "  (challenge issuance, enrollment, credential assertion, STEP_UP"
    echo "  session, revocation, audit records)."
    echo "- ep007_live_fire_step_up_action_digest_and_fail_closed: REAL"
    echo "  STEP_UP action-digest binding and fail-closed invariant."
    echo
    echo "## Observed sentinel"
    echo "LF-003: ok"
  } > "$evidence"
fi

echo "LF-003: ok"
