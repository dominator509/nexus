#!/usr/bin/env bash
# RX-000 / doctrine section 2 verifier for .agent/remediation/AUDIT_FINDINGS.tsv.
# Fails unless the full register satisfies every invariant. A red exit here is
# the CORRECT state while any row is pending authoritative import.
set -u
TSV="$(cd "$(dirname "$0")" && pwd)/AUDIT_FINDINGS.tsv"
FAIL=0

# Valid repair nodes RX-000..RX-023
RX_NODES=$(printf 'RX-%03d\n' $(seq 0 23))

if [ ! -f "$TSV" ]; then
  echo "VERIFY_REMEDIATION_REGISTER: FAIL (missing $TSV)"
  exit 1
fi

# 1. row count and header
TOTAL=$(($(wc -l < "$TSV") - 1))
[ "$TOTAL" -eq 90 ] || { echo "finding_count == 90: FAIL (got $TOTAL)"; FAIL=1; }

# 2. duplicate / missing / unknown ids
IDS=$(tail -n +2 "$TSV" | cut -f1 | sort)
DUPS=$(echo "$IDS" | uniq -d)
[ -z "$DUPS" ] || { echo "duplicate_ids == 0: FAIL ($(echo $DUPS | tr '\n' ' '))"; FAIL=1; }
EXPECTED=$(printf 'AUD-%03d\n' $(seq 1 90) | sort)
MISSING=$(comm -23 <(echo "$EXPECTED") <(echo "$IDS"))
[ -z "$MISSING" ] || { echo "missing_ids == 0: FAIL ($(echo $MISSING | tr '\n' ' '))"; FAIL=1; }
UNKNOWN=$(comm -13 <(echo "$EXPECTED") <(echo "$IDS"))
[ -z "$UNKNOWN" ] || { echo "unknown_ids == 0: FAIL ($(echo $UNKNOWN | tr '\n' ' '))"; FAIL=1; }

# 3. severity validity (P0/P1/P2/P3 only; PENDING blocks)
while IFS=$'\t' read -r aid sev title paths cause owner status rest; do
  case "$sev" in P0|P1|P2|P3) ;; *)
    echo "every_finding_has_valid_severity: FAIL ($aid severity=$sev)"; FAIL=1;;
  esac
  # 4. owner exists in repair graph (RX-000..RX-023, comma/slash separated)
  if [ "$owner" != "PENDING_AUTHORITATIVE_IMPORT" ]; then
    for o in $(echo "$owner" | tr ',' '\n' | tr '/' '\n'); do
      echo "$RX_NODES" | grep -qx "$o" || { echo "every_owner_exists_in_repair_graph: FAIL ($aid owner=$o)"; FAIL=1; }
    done
  else
    echo "every_finding_has_owner: FAIL ($aid)"; FAIL=1
  fi
  # 5. status progression only
  case "$status" in OPEN|IN_REPAIR|FIXED_UNVERIFIED|VERIFIED_FIXED) ;; *)
    echo "status_progression_only: FAIL ($aid status=$status)"; FAIL=1;;
  esac
  # 6. verified findings must carry regression test + commit-bound evidence
  if [ "$status" = "VERIFIED_FIXED" ]; then
    rt=$(echo "$rest" | cut -f1)
    ev=$(echo "$rest" | cut -f2)
    [ -n "$rt" ] || { echo "every_verified_finding_has_regression_test: FAIL ($aid)"; FAIL=1; }
    [ -n "$ev" ] || { echo "every_verified_finding_has_commit_bound_evidence: FAIL ($aid)"; FAIL=1; }
  fi
done < <(tail -n +2 "$TSV")

if [ "$FAIL" -eq 0 ]; then
  echo "VERIFY_REMEDIATION_REGISTER: PASS (90/90 registered, quarantine active)"
  exit 0
fi
echo "VERIFY_REMEDIATION_REGISTER: FAIL"
exit 1
