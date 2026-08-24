#!/usr/bin/env sh
# scripts/sbom/observability.sh - redacted operational evidence (EP-039 M4).
#
# Prints the supply-chain observability surface the fence requires:
# run_id, git_commit, lockfile/policy/inventory fingerprints, package /
# resolved / green / denied / unknown / missing-license counts, policy
# verdict, SBOM verification state, provenance state, advisory source
# status, redaction result, and the verification failure class.
#
# The output is redacted by construction: it reads only the redacted
# evidence + verification documents and never emits tokens, registry
# credentials, private URLs, or raw secrets.
#
# Usage: sh scripts/sbom/observability.sh <evidence_dir>

set -eu
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
cd "$REPO_ROOT"

fail() {
  echo "sbom/observability: FAIL - $1" >&2
  exit 1
}

OUT_DIR="${1:-}"
[ -n "$OUT_DIR" ] || fail "usage: observability.sh <evidence_dir>"
EVIDENCE="$OUT_DIR/evidence.json"
VERDICT="$OUT_DIR/verification.json"
[ -f "$EVIDENCE" ] || fail "evidence.json missing (run generate.sh first)"

python3 - "$EVIDENCE" "$VERDICT" <<'EOF'
import json
import os
import sys

evidence_path, verdict_path = sys.argv[1], sys.argv[2]
with open(evidence_path, encoding="utf-8") as f:
    e = json.load(f)

failure_class = "NONE"
verdict_state = "NOT_VERIFIED"
if os.path.isfile(verdict_path):
    with open(verdict_path, encoding="utf-8") as f:
        v = json.load(f)
    failure_class = v.get("failure_class", "NONE")
    verdict_state = v.get("verdict", "NOT_VERIFIED")

def row(key, value):
    print(f"{key}: {value}")

row("run_id", e.get("run_id", ""))
row("git_commit", e.get("git_commit", ""))
row("lockfile", e.get("lockfile", ""))
row("lockfile_fingerprint", e.get("lockfile_fingerprint", ""))
row("policy_fingerprint", e.get("policy_fingerprint", ""))
row("generated_at_ts", e.get("generated_at_ts", 0))
row("package_count", e.get("package_count", 0))
row("resolved_count", e.get("resolved_count", 0))
row("transitive_count", e.get("transitive_count", 0))
row("workspace_count", e.get("workspace_count", 0))
row("green_count", e.get("green_count", 0))
row("review_count", e.get("review_count", 0))
row("sidecar_count", e.get("sidecar_count", 0))
row("external_count", e.get("external_count", 0))
row("prohibited_count", e.get("prohibited_count", 0))
row("unknown_count", e.get("unknown_count", 0))
row("missing_license_count", e.get("missing_license_count", 0))
row("denied_count", e.get("denied_count", 0))
row("policy_verdict", e.get("policy_verdict", ""))
row("policy_passed", e.get("policy_passed", False))
row("verification_state", e.get("verification_state", ""))
row("verification_verdict", verdict_state)
row("completeness_state", e.get("completeness_state", ""))
row("legal_approved", e.get("legal_approved", False))
row("provenance_state", e.get("provenance_state", ""))
row("advisory_source_status", e.get("advisory_source_status", ""))
row("redaction", e.get("redaction", ""))
row("failure_class", failure_class)
EOF
