#!/usr/bin/env sh
# EP-039 M5 gate: final supply-chain/SBOM live-fire and node closure.
#
# M5 owns:
# - tests/supply-chain/ @nexus-supply-chain-live-fire (final live-fire
#   composition: real repo state -> real Cargo.lock inventory -> real
#   policy files -> M1 contract -> M2 engine -> M3 transport -> M4
#   scripts/sbom evidence semantics -> verification -> redacted
#   observability -> final certified/non-certified decision ->
#   current-run evidence)
# - .agent/state/evidence/EP-039-m5.json (current-run evidence)
# - node M5/verify rewiring, expected-files closure, ExecPlan closure
#
# Vacuous green is impossible: the gate requires real cargo test runs
# with non-zero pass counts, every live-fire sentinel observed, the
# FULL expected-files EP-039 list green, real scripts/sbom/ execution,
# current-run evidence verified with stale evidence rejected, redaction
# proof, and M1+M2+M3+M4 regressions.
set -eu
export CI=true
export CARGO_TERM_COLOR=never
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

log="/tmp/ep039-m5-tests.log"
: > "$log"

fail() {
  echo "EP-039 M5 gate: FAIL - $1" >&2
  tail -60 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-039 M5 gate: $1"; }

# Vacuity guard 0: M5-owned material presence.
for f in \
  tests/supply-chain/Cargo.toml \
  tests/supply-chain/src/lib.rs \
  tests/supply-chain/tests/ep039_m5_live_fire.rs; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "M5 live-fire crate present"

# Vacuity guard 0b: workspace declares the live-fire crate.
if ! grep -q '"tests/supply-chain"' Cargo.toml; then
  fail "workspace Cargo.toml missing tests/supply-chain member"
fi
ok "workspace member declared"

# Vacuity guard 0c: no placeholder content in M5-owned sources.
if grep -rqiE 'placeholder|TODO|fake|sample only' tests/supply-chain; then
  fail "M5-owned sources contain placeholder content"
fi
ok "M5-owned content validated"

# Vacuity guard 0d (anti-masking): no secret-shaped literals in
# tracked M5 sources (canaries are runtime-constructed).
if grep -rniE 'sk-[A-Za-z0-9]|ghp_[A-Za-z0-9]|AKIA[0-9A-Z]|Bearer [A-Za-z0-9]' tests/supply-chain; then
  fail "secret-shaped literal in tracked M5 sources"
fi
ok "no secret literals in tracked M5 sources"

# Vacuity guard 1: the FULL expected-files list for EP-039 must be
# green (node closure requirement; tests/supply-chain/ was the last
# missing directory).
if ! sh scripts/expected-files.sh EP-039 >"$log.expected" 2>&1; then
  fail "expected-files EP-039 not green" "$log.expected"
fi
ok "expected-files EP-039 full list green"

# Real live-fire run through cargo, captured to the log. The gate sets
# EP039_M5_EVIDENCE/RUN_ID/GIT_COMMIT so the composition writes the
# canonical current-run evidence bound to the current tree.
EVIDENCE_PATH="$PWD/.agent/state/evidence/EP-039-m5.json"
RUN_ID="ep039-m5-$(date -u +%Y%m%dT%H%M%SZ)"
GIT_COMMIT=$(/usr/bin/git rev-parse HEAD 2>/dev/null || echo unknown)
if ! EP039_M5_EVIDENCE="$EVIDENCE_PATH" EP039_M5_RUN_ID="$RUN_ID" EP039_M5_GIT_COMMIT="$GIT_COMMIT" \
  sh -c 'EP039_M5_EVIDENCE="$1" EP039_M5_RUN_ID="$2" EP039_M5_GIT_COMMIT="$3" cargo test -p nexus-supply-chain-live-fire --locked -- --nocapture >> "$4" 2>&1' \
  _ "$EVIDENCE_PATH" "$RUN_ID" "$GIT_COMMIT" "$log"; then
  fail "cargo live-fire test failed" "$log"
fi

# Vacuity guard 2: a real non-zero pass was observed.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no live-fire tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 3: zero failures.
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 4: zero ignored tests.
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi

# Vacuity guard 5 (anti-masking): every final live-fire proof ran.
for sentinel in \
  ep039_live_fire_full_composition_on_real_repo \
  ep039_live_fire_evidence_verifies_against_current_tree \
  ep039_live_fire_stale_evidence_rejected \
  ep039_live_fire_tampered_evidence_rejected \
  ep039_live_fire_mismatched_run_id_rejected \
  ep039_live_fire_empty_evidence_rejected \
  ep039_live_fire_redaction_never_leaks_canaries \
  ep039_live_fire_inventory_deterministic \
  ep039_live_fire_real_denied_finding_preserved \
  ep039_live_fire_writes_current_evidence; do
  if ! grep -q "$sentinel" "$log"; then
    fail "live-fire test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "all 10 ep039_live_fire_* proofs observed"

# Vacuity guard 6: redaction canary proof must have run.
if ! grep -q 'ep039_live_fire_redaction_never_leaks_canaries' "$log"; then
  fail "redaction canary proof did not run (anti-masking)" "$log"
fi
ok "redaction proof observed"

# Vacuity guard 7: dependency direction - the live-fire crate may
# depend only on the certified supply-chain surface + serde + sha2.
bad_dep=$(cargo tree -p nexus-supply-chain-live-fire --depth 1 2>/dev/null | grep -vE 'nexus-supply-chain-live-fire|nexus-supply-chain-policy-io|nexus-supply-chain-policy|nexus-supply-chain|serde|serde_json|sha2' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-supply-chain-live-fire: $bad_dep"
fi
for forbidden in cyclonedx spdx-tools syft grype cosign sigstore slsa in-toto trivy osv-scanner aquasec anchore quay docker-registry npm pypi pip cargo-registry; do
  if cargo tree -p nexus-supply-chain-live-fire 2>/dev/null | grep -qi "$forbidden"; then
    fail "provider SDK dependency forbidden in M5: $forbidden"
  fi
done
ok "dependency-direction clean"

# Clippy -D warnings (all targets) and fmt on the live-fire crate.
if ! sh -c 'cargo clippy -p nexus-supply-chain-live-fire --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-supply-chain-live-fire -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# Vacuity guard 8: current-run evidence was written by the composition
# and is bound + honest.
if [ ! -f "$EVIDENCE_PATH" ]; then
  fail "current-run evidence not written at $EVIDENCE_PATH"
fi
if ! grep -q '"policy_verdict":"NON_GREEN"' "$EVIDENCE_PATH"; then
  fail "evidence does not carry the honest NON_GREEN verdict"
fi
if ! grep -q '"policy_passed":false' "$EVIDENCE_PATH"; then
  fail "evidence claims policy passed (must stay false)"
fi
if ! grep -q '"ship_approved":false' "$EVIDENCE_PATH"; then
  fail "evidence claims ship approved (must be blocked)"
fi
if ! grep -q '"legal_clearance":"NOT_ASSERTED"' "$EVIDENCE_PATH"; then
  fail "evidence must not claim legal clearance"
fi
if grep -qE 'sk-|ghp_|AKIA|Bearer |token=|password=|secret=' "$EVIDENCE_PATH"; then
  fail "evidence contains secret-shaped content"
fi
ok "current-run evidence written, redacted, honest verdict preserved"

# Vacuity guard 9: real scripts/sbom/ execution (M4 pipeline) on the
# current tree - generate -> verify -> observability.
WORK=$(mktemp -d /tmp/ep039-m5-gate.XXXXXX)
trap 'rm -rf "$WORK"' EXIT INT TERM

if ! sh scripts/sbom/generate.sh "$WORK/evidence" >"$WORK/generate.log" 2>&1; then
  fail "scripts/sbom/generate.sh failed" "$WORK/generate.log"
fi
if ! sh scripts/sbom/verify.sh "$WORK/evidence" >"$WORK/verify.log" 2>&1; then
  fail "scripts/sbom/verify.sh rejected fresh evidence" "$WORK/verify.log"
fi
if ! sh scripts/sbom/observability.sh "$WORK/evidence" >"$WORK/observability.log" 2>&1; then
  fail "scripts/sbom/observability.sh failed" "$WORK/observability.log"
fi
for field in run_id git_commit lockfile_fingerprint policy_fingerprint package_count \
  resolved_count green_count denied_count unknown_count missing_license_count \
  policy_verdict verification_state provenance_state advisory_source_status redaction; do
  if ! grep -q "^$field:" "$WORK/observability.log"; then
    fail "observability missing field $field" "$WORK/observability.log"
  fi
done
ok "scripts/sbom generate -> verify -> observability green"

# Vacuity guard 10: stale evidence must be rejected by the M4 pipeline
# (a file existing is not proof of current verification). The fixture is
# a VALIDLY-SIGNED evidence whose generated_at is ancient - so the only
# failing check is freshness (STALE_EVIDENCE), not the crypto seal.
mkdir -p "$WORK/stale"
cp "$WORK/evidence/evidence.json" "$WORK/stale/evidence.json"
cp "$WORK/evidence/evidence.json.sha256" "$WORK/stale/evidence.json.sha256"
python3 - "$WORK/stale/evidence.json" <<'PYEOF'
import json
import sys
p = sys.argv[1]
with open(p, encoding="utf-8") as f:
    d = json.load(f)
d["generated_at_ts"] = 1_000_000_000
with open(p, "w", encoding="utf-8") as f:
    json.dump(d, f)
PYEOF
sha256sum "$WORK/stale/evidence.json" | awk '{print $1}' >"$WORK/stale/evidence.json.sha256"
# Re-sign the stale evidence with a fresh keypair so the signature is
# cryptographically valid; staleness must be the ONLY denial.
cargo run -q -p nexus-supply-chain --example evidence_sign -- \
  keygen "$WORK/stale/signing-key.der" "$WORK/stale/evidence.json.pub" \
  >>"$log" 2>&1 || fail "stale fixture keygen failed" "$log"
cargo run -q -p nexus-supply-chain --example evidence_sign -- \
  sign "$WORK/stale/evidence.json" "$WORK/stale/signing-key.der" \
  "$WORK/stale/evidence.json.sig" "$WORK/stale/evidence.json.pub" \
  >>"$log" 2>&1 || fail "stale fixture signing failed" "$log"
rm -f "$WORK/stale/signing-key.der"
if sh scripts/sbom/verify.sh "$WORK/stale" >"$WORK/stale.log" 2>&1; then
  fail "stale evidence verified (must be rejected)"
fi
if ! grep -q 'STALE_EVIDENCE' "$WORK/stale/verification.json"; then
  fail "stale evidence failure class not typed" "$WORK/stale/verification.json"
fi
ok "stale evidence rejected (STALE_EVIDENCE)"

# Vacuity guard 11: M1 + M2 + M3 + M4 regressions (the node's own
# gates, not a wrapper).
for gate in scripts/ep039-m1-tests.sh scripts/ep039-m2-tests.sh scripts/ep039-m3-tests.sh scripts/ep039-m4-tests.sh; do
  if ! sh "$gate" >>"$log" 2>&1; then
    fail "regression gate $gate failed" "$log"
  fi
done
ok "M1 + M2 + M3 + M4 regression gates green"

echo "EP-039 M5 gate: ok"
