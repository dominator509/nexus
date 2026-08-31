#!/usr/bin/env sh
# scripts/sbom/generate.sh - real SBOM evidence generator (EP-039 M4).
#
# Generates a redacted, state-bound SBOM evidence document from the
# REAL workspace Cargo.lock, the REAL cargo registry cache, and the
# checked-in policies/licenses/ files. The generator is the certified
# transport adapter (policies/licenses/examples/sbom_generate.rs) run
# through cargo; this script supplies the current-run bindings:
#
#   run_id             ep039-sbom-<git_commit>
#   git_commit         current HEAD (short)
#   lockfile_fingerprint  sha256(Cargo.lock)
#   policy_fingerprint sha256(concatenated policies/licenses/*.toml)
#   inventory_fingerprint sha256(evidence.json) written as
#                        evidence.json.sha256 (the seal)
#
# Fail closed: missing Cargo.lock, malformed Cargo.lock, or a failed
# inventory evaluation aborts with a non-zero exit - an empty or
# guessed SBOM is never emitted.
#
# Usage: sh scripts/sbom/generate.sh [output_dir]
#   output_dir defaults to a fresh mktemp dir under /tmp.

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
  echo "sbom/generate: FAIL - $1" >&2
  exit 1
}

OUT_DIR="${1:-}"
if [ -z "$OUT_DIR" ]; then
  OUT_DIR=$(mktemp -d /tmp/ep039-sbom-evidence.XXXXXX)
fi
mkdir -p "$OUT_DIR"

# Input validation: the generator cannot proceed without the real
# repository state (fail closed, never guess).
[ -f Cargo.lock ] || fail "Cargo.lock missing at $REPO_ROOT/Cargo.lock"
[ -f policies/licenses/allowlist.toml ] || fail "policy files missing"
[ -d policies/licenses/src ] || fail "transport crate source missing"
if command -v /usr/bin/git >/dev/null 2>&1; then
  GIT_BIN=/usr/bin/git
else
  GIT_BIN=git
fi
GIT_COMMIT=$("$GIT_BIN" rev-parse --short=12 HEAD 2>/dev/null || echo "unknown-commit")
[ "$GIT_COMMIT" != "unknown-commit" ] || fail "not a git repository"

RUN_ID="ep039-sbom-$GIT_COMMIT"
LOCKFILE_FINGERPRINT=$(sha256sum Cargo.lock | awk '{print $1}')
POLICY_FINGERPRINT=$(sha256sum policies/licenses/*.toml | awk '{print $1}' | sha256sum | awk '{print $1}')

EVIDENCE="$OUT_DIR/evidence.json"
if ! cargo run -q -p nexus-supply-chain-policy-io --example sbom_generate -- \
  "$REPO_ROOT" "$RUN_ID" "$GIT_COMMIT" "$LOCKFILE_FINGERPRINT" \
  "$POLICY_FINGERPRINT" "$EVIDENCE" >"$OUT_DIR/generate.log" 2>&1; then
  cat "$OUT_DIR/generate.log" >&2
  fail "inventory evaluation failed closed (see log)"
fi

# Inventory fingerprint: the sha256 seal over the evidence file. Any
# tampering after this point breaks verify.sh.
sha256sum "$EVIDENCE" | awk '{print $1}' >"$OUT_DIR/evidence.json.sha256"

# Cryptographic seal (AUD-059): a bare sha256 checksum stored beside the
# evidence can be recomputed by anyone able to change the evidence. The
# real seal is an Ed25519 signature over the evidence digest. The
# private key is taken from NEXUS_EVIDENCE_SIGNING_KEY (PKCS#8 v2 DER,
# base64) or generated fresh for the run; the public key is written next
# to the evidence so verify.sh can check the signature without the
# private key.
if [ -n "${NEXUS_EVIDENCE_SIGNING_KEY:-}" ]; then
  printf '%s' "$NEXUS_EVIDENCE_SIGNING_KEY" | base64 -d >"$OUT_DIR/signing-key.der"
  cargo run -q -p nexus-supply-chain --example evidence_sign -- \
    sign "$EVIDENCE" "$OUT_DIR/signing-key.der" \
    "$OUT_DIR/evidence.json.sig" "$OUT_DIR/evidence.json.pub" \
    >>"$OUT_DIR/generate.log" 2>&1 || fail "cryptographic evidence signing failed"
  rm -f "$OUT_DIR/signing-key.der"
else
  cargo run -q -p nexus-supply-chain --example evidence_sign -- \
    keygen "$OUT_DIR/signing-key.der" "$OUT_DIR/evidence.json.pub" \
    >>"$OUT_DIR/generate.log" 2>&1 || fail "evidence signing keygen failed"
  cargo run -q -p nexus-supply-chain --example evidence_sign -- \
    sign "$EVIDENCE" "$OUT_DIR/signing-key.der" \
    "$OUT_DIR/evidence.json.sig" "$OUT_DIR/evidence.json.pub" \
    >>"$OUT_DIR/generate.log" 2>&1 || fail "cryptographic evidence signing failed"
  rm -f "$OUT_DIR/signing-key.der"
fi
[ -f "$OUT_DIR/evidence.json.sig" ] || fail "evidence signature missing"
[ -f "$OUT_DIR/evidence.json.pub" ] || fail "evidence public key missing"

cat "$OUT_DIR/generate.log"
echo "sbom/generate: evidence $EVIDENCE"
echo "sbom/generate: run_id $RUN_ID"
echo "sbom/generate: git_commit $GIT_COMMIT"
echo "sbom/generate: lockfile_fingerprint $LOCKFILE_FINGERPRINT"
echo "sbom/generate: policy_fingerprint $POLICY_FINGERPRINT"
echo "sbom/generate: inventory_fingerprint $(cat "$OUT_DIR/evidence.json.sha256")"
echo "sbom/generate: ok"
