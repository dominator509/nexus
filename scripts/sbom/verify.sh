#!/usr/bin/env sh
# scripts/sbom/verify.sh - real SBOM evidence verifier (EP-039 M4).
#
# Binds the generated SBOM evidence to the CURRENT repository state and
# rejects stale / empty / tampered / mismatched evidence with a typed
# failure class. A file existing is NOT proof of current verification:
# every binding must be recomputed and match.
#
# Rejects:
#   missing evidence            -> EMPTY_EVIDENCE
#   broken sha256 seal          -> TAMPERED_EVIDENCE
#   wrong run_id                -> MISMATCHED_RUN_ID
#   git_commit != HEAD          -> STALE_GIT_COMMIT
#   lockfile fingerprint drift  -> STALE_LOCKFILE
#   policy fingerprint drift    -> STALE_POLICY
#   evidence older than window  -> STALE_EVIDENCE
#   zero packages / empty list  -> EMPTY_EVIDENCE
#   secret-shaped content       -> REDACTION_FAILURE
#
# Usage: sh scripts/sbom/verify.sh <evidence_dir> [max_age_secs]

set -eu
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
cd "$REPO_ROOT"

fail() {
  echo "sbom/verify: FAIL - $1" >&2
  exit 1
}

OUT_DIR="${1:-}"
[ -n "$OUT_DIR" ] || fail "usage: verify.sh <evidence_dir> [max_age_secs]"
MAX_AGE="${2:-86400}"
EVIDENCE="$OUT_DIR/evidence.json"
SEAL="$OUT_DIR/evidence.json.sha256"
SIGNATURE="$OUT_DIR/evidence.json.sig"
PUBKEY="$OUT_DIR/evidence.json.pub"
ECOSYSTEMS="$OUT_DIR/ecosystems.json"
ECOSYSTEMS_SIG="$OUT_DIR/ecosystems.json.sig"
ECOSYSTEMS_PUB="$OUT_DIR/ecosystems.json.pub"
VERDICT="$OUT_DIR/verification.json"

if command -v /usr/bin/git >/dev/null 2>&1; then
  GIT_BIN=/usr/bin/git
else
  GIT_BIN=git
fi
GIT_COMMIT=$("$GIT_BIN" rev-parse --short=12 HEAD 2>/dev/null || echo "unknown-commit")
[ "$GIT_COMMIT" != "unknown-commit" ] || fail "not a git repository"
EXPECTED_RUN_ID="ep039-sbom-$GIT_COMMIT"
LOCKFILE_FINGERPRINT=$(sha256sum Cargo.lock | awk '{print $1}')
POLICY_FINGERPRINT=$(sha256sum policies/licenses/*.toml | awk '{print $1}' | sha256sum | awk '{print $1}')
NOW_TS=$(date +%s)

# Python performs the structured checks and writes verification.json so
# the typed failure class is observable. The script exits non-zero on
# any failed check (fail closed).
python3 - "$EVIDENCE" "$SEAL" "$VERDICT" "$EXPECTED_RUN_ID" "$GIT_COMMIT" \
  "$LOCKFILE_FINGERPRINT" "$POLICY_FINGERPRINT" "$NOW_TS" "$MAX_AGE" \
  "$SIGNATURE" "$PUBKEY" "$ECOSYSTEMS" "$ECOSYSTEMS_SIG" "$ECOSYSTEMS_PUB" <<'EOF'
import json
import os
import sys

evidence_path, seal_path, verdict_path, expected_run_id, git_commit = sys.argv[1:6]
lockfile_fp, policy_fp, now_ts, max_age = sys.argv[6], sys.argv[7], int(sys.argv[8]), int(sys.argv[9])

checks = {
    "evidence_present": False,
    "seal_matches": False,
    "signature_present": False,
    "signature_verified": False,
    "run_id_matches": False,
    "git_commit_matches": False,
    "lockfile_matches": False,
    "policy_matches": False,
    "freshness": False,
    "non_empty": False,
    "redaction": False,
    "ecosystems_present": False,
    "ecosystems_run_id_matches": False,
    "ecosystems_signature_verified": False,
}
failure_class = "NONE"


def write_verdict(path, checks_map, failure, reason, run_id):
    with open(path, "w", encoding="utf-8") as f:
        json.dump({
            "run_id": run_id,
            "git_commit": git_commit,
            "verdict": "REJECTED" if failure != "NONE" else "VERIFIED",
            "failure_class": failure,
            "reason": reason,
            "checks": checks_map,
        }, f, indent=2, sort_keys=True)


# 1. Evidence must exist (a missing file is not a verified SBOM).
if not os.path.isfile(evidence_path):
    write_verdict(verdict_path, checks, "EMPTY_EVIDENCE",
                  "evidence.json missing at " + evidence_path, expected_run_id)
    sys.exit(1)
checks["evidence_present"] = True

try:
    with open(evidence_path, encoding="utf-8") as f:
        evidence = json.load(f)
except Exception as exc:
    write_verdict(verdict_path, checks, "MALFORMED_EVIDENCE",
                  "evidence.json is not valid JSON: " + str(exc), expected_run_id)
    sys.exit(1)

# 2. The sha256 seal must match the file (tamper detection).
seal = ""
if os.path.isfile(seal_path):
    with open(seal_path, encoding="utf-8") as f:
        seal = f.read().strip()
    import hashlib
    with open(evidence_path, "rb") as f:
        actual = hashlib.sha256(f.read()).hexdigest()
    checks["seal_matches"] = actual == seal

# 2b. Cryptographic signature (AUD-059): the evidence must carry a real
# Ed25519 signature + public key, and the signature must verify against
# the evidence digest with that public key. A bare checksum is not a
# seal - anyone able to change evidence can change its checksum.
sig_path = sys.argv[10] if len(sys.argv) > 10 else ""
pub_path = sys.argv[11] if len(sys.argv) > 11 else ""
pinned_pub = os.environ.get("NEXUS_EVIDENCE_PUBKEY", "")
checks["signature_present"] = os.path.isfile(sig_path) and os.path.isfile(pub_path)
checks["signature_verified"] = False
if checks["signature_present"]:
    import subprocess
    # Pinned-key mode: when the caller knows the trusted public key
    # (env), the stored pubkey must match it - an attacker who swaps
    # evidence + signature + public key together is still caught.
    verify_pub = pub_path
    if pinned_pub:
        try:
            pinned_bytes = bytes.fromhex(pinned_pub)
            stored_bytes = open(pub_path, "rb").read()
            if stored_bytes != pinned_bytes:
                # Pubkey swap detected: the stored key is not the trusted
                # key, so the signature cannot verify -> SIGNATURE_INVALID.
                verify_pub = ""
        except Exception:
            verify_pub = ""
    if verify_pub:
        verify_run = subprocess.run(
            [
                "cargo", "run", "-q", "-p", "nexus-supply-chain",
                "--example", "evidence_sign", "--",
                "verify", evidence_path, verify_pub, sig_path,
            ],
            capture_output=True,
            text=True,
            env={**os.environ, "CI": "true", "CARGO_TERM_COLOR": "never"},
        )
        checks["signature_verified"] = verify_run.returncode == 0
        if not checks["signature_verified"]:
            reason_detail = (verify_run.stdout + verify_run.stderr).strip()[-300:]

# 3-6. Current-run bindings must match the recomputed state.
checks["run_id_matches"] = evidence.get("run_id") == expected_run_id
checks["git_commit_matches"] = evidence.get("git_commit") == git_commit
checks["lockfile_matches"] = evidence.get("lockfile_fingerprint") == lockfile_fp
checks["policy_matches"] = evidence.get("policy_fingerprint") == policy_fp

# 3b. Multi-ecosystem shipped-product inventory (AUD-060): the
# ecosystems evidence must exist, be bound to the current run, and be
# cryptographically signed. A Cargo-only SBOM is not a complete
# shipped-product SBOM.
eco_path = sys.argv[12] if len(sys.argv) > 12 else ""
eco_sig_path = sys.argv[13] if len(sys.argv) > 13 else ""
eco_pub_path = sys.argv[14] if len(sys.argv) > 14 else ""
checks["ecosystems_present"] = os.path.isfile(eco_path) and os.path.isfile(eco_sig_path) and os.path.isfile(eco_pub_path)
checks["ecosystems_run_id_matches"] = False
checks["ecosystems_signature_verified"] = False
if checks["ecosystems_present"]:
    try:
        with open(eco_path, encoding="utf-8") as f:
            eco = json.load(f)
        checks["ecosystems_run_id_matches"] = eco.get("run_id") == expected_run_id and eco.get("git_commit") == git_commit
    except Exception:
        checks["ecosystems_run_id_matches"] = False
    import subprocess
    eco_verify = subprocess.run(
        [
            "cargo", "run", "-q", "-p", "nexus-supply-chain",
            "--example", "evidence_sign", "--",
            "verify", eco_path, eco_pub_path, eco_sig_path,
        ],
        capture_output=True,
        text=True,
        env={**os.environ, "CI": "true", "CARGO_TERM_COLOR": "never"},
    )
    checks["ecosystems_signature_verified"] = eco_verify.returncode == 0

# 7. Freshness: generated_at must be within the bounded window.
generated = evidence.get("generated_at_ts", 0)
checks["freshness"] = (now_ts - generated) <= max_age

# 8. Non-empty: a generated SBOM with zero packages is not complete.
packages = evidence.get("packages", [])
checks["non_empty"] = evidence.get("package_count", 0) > 0 and isinstance(packages, list) and len(packages) > 0

# 9. Redaction: no secret-shaped content in the evidence.
markers = ["sk-", "pk-", "rk-", "ghp_", "gho_", "ghs_", "github_pat_", "AKIA",
           "Bearer ", "xoxb-", "glpat-", "token=", "password=", "secret=",
           "client_secret=", "aws_secret_access_key=", "private_key="]
raw = open(evidence_path, encoding="utf-8").read()
checks["redaction"] = not any(m in raw for m in markers)

# Deterministic verdict: every check must pass; otherwise the first
# failing check names the typed failure class.
if all(checks.values()):
    verdict = "VERIFIED"
    failure_class = "NONE"
    reason = "evidence bound to current repository state and redacted"
else:
    verdict = "REJECTED"
    order = [
        ("evidence_present", "EMPTY_EVIDENCE"),
        ("seal_matches", "TAMPERED_EVIDENCE"),
        ("signature_present", "SIGNATURE_MISSING"),
        ("signature_verified", "SIGNATURE_INVALID"),
        ("run_id_matches", "MISMATCHED_RUN_ID"),
        ("git_commit_matches", "STALE_GIT_COMMIT"),
        ("lockfile_matches", "STALE_LOCKFILE"),
        ("policy_matches", "STALE_POLICY"),
        ("freshness", "STALE_EVIDENCE"),
        ("non_empty", "EMPTY_EVIDENCE"),
        ("redaction", "REDACTION_FAILURE"),
        ("ecosystems_present", "ECOSYSTEMS_MISSING"),
        ("ecosystems_run_id_matches", "ECOSYSTEMS_STALE"),
        ("ecosystems_signature_verified", "ECOSYSTEMS_SIGNATURE_INVALID"),
    ]
    for key, cls in order:
        if not checks[key]:
            failure_class = cls
            break
    reason = "evidence rejected: " + failure_class

write_verdict(verdict_path, checks, failure_class, reason, expected_run_id)

if verdict != "VERIFIED":
    sys.exit(1)
EOF
echo "sbom/verify: VERIFIED (evidence bound to current repository state)"
echo "sbom/verify: $(cat "$VERDICT")"
