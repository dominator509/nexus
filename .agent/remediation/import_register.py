#!/usr/bin/env python3
"""RX-000 register materializer.

Builds .agent/remediation/AUDIT_FINDINGS.tsv from the authoritative register in
register_data.py (imported verbatim from the audit conversation, share
6a926876-0c84-83e8-a9da-4f3d53dd1ddc, audited commit 0460cc65f97868a80722ca7814d94be7cd6120e4).

Required columns:
audit_id severity title affected_paths root_cause repair_node status regression_test evidence_ref fixed_commit verified_commit
"""
from __future__ import annotations

import csv
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from register_data import FINDINGS  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent
OUT = ROOT / "AUDIT_FINDINGS.tsv"
COLUMNS = [
    "audit_id", "severity", "title", "affected_paths", "root_cause",
    "repair_node", "status", "regression_test", "evidence_ref",
    "fixed_commit", "verified_commit",
]

VALID_STATUS = {"OPEN", "IN_REPAIR", "FIXED_UNVERIFIED", "VERIFIED_FIXED"}


def main() -> int:
    ids = [f"AUD-{i:03d}" for i in range(1, 91)]
    missing = [a for a in ids if a not in FINDINGS]
    if missing:
        print(f"FATAL: register_data.py missing {missing}", file=sys.stderr)
        return 2
    extra = [a for a in FINDINGS if a not in set(ids)]
    if extra:
        print(f"FATAL: register_data.py has unknown ids {extra}", file=sys.stderr)
        return 2

    rows = []
    for aid in ids:
        sev, title, affected, cause, owner = FINDINGS[aid]
        assert sev in {"P0", "P1", "P2", "P3"}, (aid, sev)
        assert owner != "", aid
        rows.append({
            "audit_id": aid,
            "severity": sev,
            "title": title,
            "affected_paths": affected,
            "root_cause": cause,
            "repair_node": owner,
            "status": "OPEN",
            "regression_test": "",
            "evidence_ref": "",
            "fixed_commit": "",
            "verified_commit": "",
        })

    with OUT.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=COLUMNS, delimiter="\t", lineterminator="\n")
        w.writeheader()
        w.writerows(rows)

    print(f"wrote {OUT} rows={len(rows)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
