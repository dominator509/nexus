#!/usr/bin/env python3
"""RX-000 register materializer.

Builds .agent/remediation/AUDIT_FINDINGS.tsv.

Authoritative inputs today:
  * AUD-066..AUD-090 titles + repair owners: remediation graph section 12 (verbatim titles).
  * AUD-001..AUD-065: PENDING_AUTHORITATIVE_IMPORT. The remediation doctrine forbids
    reconstructing audit history from memory. These rows are registered as OPEN placeholders
    with a machine-checkable pending marker so the verifier stays red until the authoritative
    register is imported.

The TSV has exactly 90 rows, AUD-001..AUD-090, with the required columns:
audit_id severity title affected_paths root_cause repair_node status regression_test evidence_ref fixed_commit verified_commit
"""
from __future__ import annotations

import csv
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent
OUT = ROOT / "AUDIT_FINDINGS.tsv"
COLUMNS = [
    "audit_id", "severity", "title", "affected_paths", "root_cause",
    "repair_node", "status", "regression_test", "evidence_ref",
    "fixed_commit", "verified_commit",
]

PENDING = "PENDING_AUTHORITATIVE_IMPORT"
PENDING_CAUSE = (
    "Authoritative audit register not present in workspace, remote, session store, or GitHub. "
    "Doctrine forbids reconstructing audit history from memory. Import verbatim before remediation."
)

# Continuation findings AUD-066..AUD-090. Title + owner are authoritative from
# remediation graph section 12 (GraphlockNexusRemediation.md / GraphlockWireMudderRemediation.md).
CONTINUATION = {
    "AUD-066": ("P1", "wrong-backup VERIFIED rollback", "RX-011"),
    "AUD-067": ("P1", "destructive pseudo-atomic switch", "RX-011"),
    "AUD-068": ("P1", "installer not bound to manifest", "RX-011"),
    "AUD-069": ("P1", "install lacks durable idempotency", "RX-011"),
    "AUD-070": ("P1", "arbitrary approval string", "RX-012"),
    "AUD-071": ("P1", "live-fire presence-only PASS", "RX-002/RX-021"),
    "AUD-072": ("P1", "drills omitted by readiness", "RX-002/RX-021"),
    "AUD-073": ("P1", "review filename = PASS", "RX-002/RX-021"),
    "AUD-074": ("P1", "textual certification = signed", "RX-002/RX-021"),
    "AUD-075": ("P1", "fresh-clone filename = proof", "RX-002/RX-022"),
    "AUD-076": ("P1", ".git/HEAD accepted as release tag", "RX-010"),
    "AUD-077": ("P1", "text-decoded binary hashing", "RX-010"),
    "AUD-078": ("P1", "manifest digest omits nested content", "RX-010"),
    "AUD-079": ("P1", "evidence digest omits nested content", "RX-010"),
    "AUD-080": ("P1", "EP-043 closes while NOT_READY", "RX-001/RX-002"),
    "AUD-081": ("P1", "deploy is dry-run only", "RX-012/RX-013"),
    "AUD-082": ("P1", "release verifies fixture bytes", "RX-013"),
    "AUD-083": ("P1", "telemetry bootstrap absent", "RX-008"),
    "AUD-084": ("P1", "runtime is health shell", "RX-008"),
    "AUD-085": ("P1", "scheduler violates DONE definition", "RX-001"),
    "AUD-086": ("P1", "phantom canonical CLI packages", "RX-013"),
    "AUD-087": ("P1", "production-readiness command incomplete", "RX-002/RX-021"),
    "AUD-088": ("P1", "default branch bypasses normal push CI", "RX-003"),
    "AUD-089": ("P1", "release workflow does not release", "RX-003/RX-022"),
    "AUD-090": ("P1", "fresh clone does not run ship standard", "RX-022"),
}

VALID_STATUS = {"OPEN", "IN_REPAIR", "FIXED_UNVERIFIED", "VERIFIED_FIXED"}


def row(audit_id: str, severity: str, title: str, affected: str, cause: str,
        owner: str, status: str = "OPEN") -> dict:
    assert status in VALID_STATUS, f"bad status {status}"
    return {
        "audit_id": audit_id,
        "severity": severity,
        "title": title,
        "affected_paths": affected,
        "root_cause": cause,
        "repair_node": owner,
        "status": status,
        "regression_test": "",
        "evidence_ref": "",
        "fixed_commit": "",
        "verified_commit": "",
    }


def main() -> int:
    rows = []
    for i in range(1, 91):
        aid = f"AUD-{i:03d}"
        if aid in CONTINUATION:
            sev, title, owner = CONTINUATION[aid]
            rows.append(row(aid, sev, title, PENDING,
                            f"Title authoritative from remediation graph sec.12; full root-cause text pending register import.",
                            owner))
        else:
            rows.append(row(aid, PENDING, PENDING, PENDING, PENDING_CAUSE, PENDING))

    with OUT.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=COLUMNS, delimiter="\t", lineterminator="\n")
        w.writeheader()
        w.writerows(rows)

    ids = [r["audit_id"] for r in rows]
    assert len(ids) == 90, len(ids)
    assert len(set(ids)) == 90
    assert ids == [f"AUD-{i:03d}" for i in range(1, 91)]
    print(f"wrote {OUT} rows={len(rows)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
