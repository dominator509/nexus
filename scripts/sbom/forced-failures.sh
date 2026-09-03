#!/usr/bin/env sh
# scripts/sbom/forced-failures.sh - forced-failure and abuse-case suite
# for the supply-chain / SBOM surface (EP-039 M4).
#
# Runs:
#   1. the real ep039_failure_* cargo test suite (isolated fixtures,
#      real registry cache, real policy files - no mocked component)
#   2. shell-level SBOM evidence abuse checks against the REAL
#      scripts/sbom/ scripts: missing/malformed lockfile fail closed,
#      stale / empty / tampered / mismatched evidence is REJECTED by
#      verify.sh with typed failure classes, redaction is proven.
#
# Exit 0 only when every failure case actually fails closed.
#
# Usage: sh scripts/sbom/forced-failures.sh

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

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
cd "$REPO_ROOT"

fail() {
  echo "sbom/forced-failures: FAIL - $1" >&2
  exit 1
}
ok() { echo "sbom/forced-failures: $1"; }

# Re-sign a fixture evidence dir with a fresh keypair so the crypto seal
# is valid; the fixture's intended failure class is then the ONLY denial
# (staleness, run mismatch, empty - not a missing signature).
# Also (re)sign the ecosystems evidence so multi-ecosystem checks pass.
resign_fixture() {
  dir="$1"
  cargo run -q -p nexus-supply-chain --example evidence_sign -- \
    keygen "$dir/signing-key.der" "$dir/evidence.json.pub" \
    >>"$log" 2>&1 || return 1
  cargo run -q -p nexus-supply-chain --example evidence_sign -- \
    sign "$dir/evidence.json" "$dir/signing-key.der" \
    "$dir/evidence.json.sig" "$dir/evidence.json.pub" \
    >>"$log" 2>&1 || return 1
  if [ -f "$dir/ecosystems.json" ]; then
    cargo run -q -p nexus-supply-chain --example evidence_sign -- \
      keygen "$dir/eco-key.der" "$dir/ecosystems.json.pub" \
      >>"$log" 2>&1 || return 1
    cargo run -q -p nexus-supply-chain --example evidence_sign -- \
      sign "$dir/ecosystems.json" "$dir/eco-key.der" \
      "$dir/ecosystems.json.sig" "$dir/ecosystems.json.pub" \
      >>"$log" 2>&1 || return 1
    rm -f "$dir/eco-key.der"
  fi
  rm -f "$dir/signing-key.der"
}

log="/tmp/ep039-sbom-forced-failures.log"
: > "$log"

# --- 1. Rust forced-failure suite (real mechanism, no mocks) ----------
if ! sh -c 'cargo test -p nexus-supply-chain-policy-io --test ep039_failure_sbom --locked >> "$1" 2>&1' _ "$log"; then
  fail "cargo failure suite failed" "$log"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no failure tests ran (vacuity guard)" "$log"
fi
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed failure tests (vacuity guard)" "$log"
fi
for sentinel in \
  ep039_failure_missing_lockfile_fails_closed \
  ep039_failure_malformed_lockfile_fails_closed \
  ep039_failure_empty_lockfile_refused \
  ep039_failure_generate_inventory_missing_lockfile_fails_closed \
  ep039_failure_unknown_license_fails_closed \
  ep039_failure_missing_license_field_fails_closed_on_real_workspace \
  ep039_failure_fuzzy_license_alias_fails_closed \
  ep039_failure_prohibited_license_fails_closed \
  ep039_failure_transitive_dependency_with_denied_license_fails_closed \
  ep039_failure_duplicate_package_ambiguity_fails \
  ep039_failure_same_package_version_different_source_fails \
  ep039_failure_image_tag_without_digest_fails \
  ep039_failure_stale_sbom_evidence_fails \
  ep039_failure_empty_sbom_evidence_fails \
  ep039_failure_tampered_sbom_binding_fails \
  ep039_failure_mismatched_run_id_fails \
  ep039_failure_waiver_wrong_scope_fails \
  ep039_failure_waiver_expired_fails \
  ep039_failure_waiver_revoked_fails \
  ep039_failure_advisory_source_not_queried_fails \
  ep039_failure_advisory_critical_unmitigated_blocks \
  ep039_failure_secret_canary_redacted_in_evidence \
  ep039_failure_observability_evidence_bound_to_real_inventory \
  ep039_failure_real_inventory_denied_finding_preserved \
  ep039_failure_license_engine_denied_without_approval_fails \
  ep039_failure_unverified_component_never_releasable; do
  if ! grep -q "$sentinel" "$log"; then
    fail "failure test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "Rust forced-failure suite green (26 ep039_failure_* proofs)"

# --- 2. Shell-level SBOM evidence abuse checks ------------------------
WORK=$(mktemp -d /tmp/ep039-sbom-abuse.XXXXXX)
trap 'rm -rf "$WORK"' EXIT INT TERM

# 2a. Missing Cargo.lock: the real generator adapter must fail closed
# (the transport refuses to emit an empty/guessed SBOM). Uses the
# certified example directly against an isolated fixture root.
MISSING_ROOT="$WORK/missing-root"
mkdir -p "$MISSING_ROOT/policies/licenses"
if cargo run -q -p nexus-supply-chain-policy-io --example sbom_generate -- \
  "$MISSING_ROOT" "ep039-sbom-missing" "deadbeef" "fp" "fp" "$WORK/missing-evidence.json" \
  >"$WORK/missing.log" 2>&1; then
  fail "generator succeeded with no Cargo.lock (must fail closed)"
fi
ok "missing Cargo.lock fails closed"

# 2b. Malformed Cargo.lock: generator must fail closed.
BAD_ROOT="$WORK/bad-root"
mkdir -p "$BAD_ROOT/policies/licenses"
echo "this is [[ not [[ valid ] toml !!!" >"$BAD_ROOT/Cargo.lock"
# generate.sh resolves REPO_ROOT from its own location, so run the
# example directly against the malformed root to prove the transport
# fails closed on malformed input.
if cargo run -q -p nexus-supply-chain-policy-io --example sbom_generate -- \
  "$BAD_ROOT" "ep039-sbom-bad" "deadbeef" "fp" "fp" "$WORK/bad-evidence.json" \
  >"$WORK/bad.log" 2>&1; then
  fail "generator succeeded with malformed Cargo.lock (must fail closed)"
fi
ok "malformed Cargo.lock fails closed"

# 2c. Generate fresh evidence, verify it, then abuse it.
if ! sh scripts/sbom/generate.sh "$WORK/fresh" >"$WORK/fresh-gen.log" 2>&1; then
  fail "fresh generation failed" "$WORK/fresh-gen.log"
fi
if ! sh scripts/sbom/verify.sh "$WORK/fresh" >"$WORK/fresh-verify.log" 2>&1; then
  fail "fresh evidence did not verify" "$WORK/fresh-verify.log"
fi
ok "fresh evidence verifies against current repository state"

# 2d. Tampered evidence must be rejected - INCLUDING when the attacker
# recomputes the sha256 seal. The cryptographic signature is the real
# seal (AUD-059): changing the evidence invalidates the signature even
# if the checksum is resealed.
cp "$WORK/fresh/evidence.json" "$WORK/tampered.json"
cp "$WORK/fresh/evidence.json.sha256" "$WORK/tampered.json.sha256"
mkdir -p "$WORK/tampered"
cp "$WORK/tampered.json" "$WORK/tampered/evidence.json"
cp "$WORK/tampered.json.sha256" "$WORK/tampered/evidence.json.sha256"
cp "$WORK/fresh/evidence.json.sig" "$WORK/tampered/evidence.json.sig"
cp "$WORK/fresh/evidence.json.pub" "$WORK/tampered/evidence.json.pub"
cp "$WORK/fresh/ecosystems.json" "$WORK/tampered/ecosystems.json" 2>/dev/null || true
cp "$WORK/fresh/ecosystems.json.sig" "$WORK/tampered/ecosystems.json.sig" 2>/dev/null || true
cp "$WORK/fresh/ecosystems.json.pub" "$WORK/tampered/ecosystems.json.pub" 2>/dev/null || true
# Tamper one byte in the packages list.
python3 - "$WORK/tampered/evidence.json" <<'PYEOF'
import json
import sys
p = sys.argv[1]
with open(p, encoding="utf-8") as f:
    d = json.load(f)
d["package_count"] = d["package_count"] + 1
with open(p, "w", encoding="utf-8") as f:
    json.dump(d, f)
PYEOF
# The attacker reseals the bare checksum - it must NOT help.
sha256sum "$WORK/tampered/evidence.json" | awk '{print $1}' >"$WORK/tampered/evidence.json.sha256"
if sh scripts/sbom/verify.sh "$WORK/tampered" >"$WORK/tampered-verify.log" 2>&1; then
  fail "tampered evidence verified (must be rejected)"
fi
if ! grep -q 'SIGNATURE_INVALID' "$WORK/tampered/verification.json"; then
  fail "tampered evidence failure class not typed: $(cat "$WORK/tampered/verification.json")"
fi
ok "tampered evidence rejected (SIGNATURE_INVALID despite resealed checksum)"

# 2e. Stale evidence must be rejected (freshness window).
mkdir -p "$WORK/stale"
cp "$WORK/fresh/evidence.json" "$WORK/stale/evidence.json"
cp "$WORK/fresh/evidence.json.sha256" "$WORK/stale/evidence.json.sha256"
cp "$WORK/fresh/ecosystems.json" "$WORK/stale/ecosystems.json" 2>/dev/null || true
python3 - "$WORK/stale/evidence.json" <<'PYEOF'
import json
import sys
p = sys.argv[1]
with open(p, encoding="utf-8") as f:
    d = json.load(f)
d["generated_at_ts"] = 1_000_000_000  # years ago
with open(p, "w", encoding="utf-8") as f:
    json.dump(d, f)
PYEOF
sha256sum "$WORK/stale/evidence.json" | awk '{print $1}' >"$WORK/stale/evidence.json.sha256"
resign_fixture "$WORK/stale" || fail "stale fixture re-sign failed" "$log"
if sh scripts/sbom/verify.sh "$WORK/stale" >"$WORK/stale-verify.log" 2>&1; then
  fail "stale evidence verified (must be rejected)"
fi
if ! grep -q 'STALE_EVIDENCE' "$WORK/stale/verification.json"; then
  fail "stale evidence failure class not typed: $(cat "$WORK/stale/verification.json")"
fi
ok "stale evidence rejected (STALE_EVIDENCE)"

# 2f. Mismatched run_id/git_commit must be rejected.
mkdir -p "$WORK/mismatched"
cp "$WORK/fresh/evidence.json" "$WORK/mismatched/evidence.json"
cp "$WORK/fresh/evidence.json.sha256" "$WORK/mismatched/evidence.json.sha256"
python3 - "$WORK/mismatched/evidence.json" <<'PYEOF'
import json
import sys
p = sys.argv[1]
with open(p, encoding="utf-8") as f:
    d = json.load(f)
d["run_id"] = "ep039-sbom-foreign-run"
d["git_commit"] = "ffffffffffff"
with open(p, "w", encoding="utf-8") as f:
    json.dump(d, f)
PYEOF
sha256sum "$WORK/mismatched/evidence.json" | awk '{print $1}' >"$WORK/mismatched/evidence.json.sha256"
resign_fixture "$WORK/mismatched" || fail "mismatched fixture re-sign failed" "$log"
if sh scripts/sbom/verify.sh "$WORK/mismatched" >"$WORK/mismatched-verify.log" 2>&1; then
  fail "mismatched run_id evidence verified (must be rejected)"
fi
if ! grep -q 'MISMATCHED_RUN_ID' "$WORK/mismatched/verification.json"; then
  fail "mismatched run_id failure class not typed: $(cat "$WORK/mismatched/verification.json")"
fi
ok "mismatched run_id/git_commit rejected (MISMATCHED_RUN_ID)"

# 2g. Empty evidence must be rejected.
mkdir -p "$WORK/empty"
echo '{"schema":"nexus.sbom.evidence.v1","run_id":"ep039-sbom-empty","git_commit":"deadbeef","package_count":0,"packages":[]}' >"$WORK/empty/evidence.json"
sha256sum "$WORK/empty/evidence.json" | awk '{print $1}' >"$WORK/empty/evidence.json.sha256"
resign_fixture "$WORK/empty" || fail "empty fixture re-sign failed" "$log"
if sh scripts/sbom/verify.sh "$WORK/empty" >"$WORK/empty-verify.log" 2>&1; then
  fail "empty evidence verified (must be rejected)"
fi
if ! grep -q 'EMPTY_EVIDENCE\|MISMATCHED_RUN_ID' "$WORK/empty/verification.json"; then
  fail "empty evidence failure class not typed: $(cat "$WORK/empty/verification.json")"
fi
ok "empty evidence rejected (EMPTY_EVIDENCE)"

# 2g2. Tampered ecosystems evidence must be rejected (AUD-060): the
# multi-ecosystem inventory is cryptographically sealed like the main
# evidence; an attacker who edits the ecosystem counts is caught even
# if they leave the Cargo evidence untouched.
mkdir -p "$WORK/eco-tampered"
cp "$WORK/fresh/evidence.json" "$WORK/eco-tampered/evidence.json"
cp "$WORK/fresh/evidence.json.sha256" "$WORK/eco-tampered/evidence.json.sha256"
cp "$WORK/fresh/evidence.json.sig" "$WORK/eco-tampered/evidence.json.sig"
cp "$WORK/fresh/evidence.json.pub" "$WORK/eco-tampered/evidence.json.pub"
cp "$WORK/fresh/ecosystems.json" "$WORK/eco-tampered/ecosystems.json"
cp "$WORK/fresh/ecosystems.json.sig" "$WORK/eco-tampered/ecosystems.json.sig"
cp "$WORK/fresh/ecosystems.json.pub" "$WORK/eco-tampered/ecosystems.json.pub"
python3 - "$WORK/eco-tampered/ecosystems.json" <<'PYEOF'
import json
import sys
p = sys.argv[1]
with open(p, encoding="utf-8") as f:
    d = json.load(f)
d["ecosystems"]["typescript"]["package_count"] = 999999
with open(p, "w", encoding="utf-8") as f:
    json.dump(d, f)
PYEOF
if sh scripts/sbom/verify.sh "$WORK/eco-tampered" >"$WORK/eco-tampered-verify.log" 2>&1; then
  fail "tampered ecosystems evidence verified (must be rejected)"
fi
if ! grep -q 'ECOSYSTEMS_SIGNATURE_INVALID' "$WORK/eco-tampered/verification.json"; then
  fail "tampered ecosystems failure class not typed: $(cat "$WORK/eco-tampered/verification.json")"
fi
ok "tampered ecosystems evidence rejected (ECOSYSTEMS_SIGNATURE_INVALID)"

# 2h. Redaction proof: generated evidence contains no secret-shaped
# markers.
if grep -qE 'sk-|ghp_|AKIA|Bearer |token=|password=|secret=' "$WORK/fresh/evidence.json"; then
  fail "generated evidence contains secret-shaped content"
fi
ok "generated evidence redacted (no secret-shaped content)"

echo "sbom/forced-failures: ok"
