#!/usr/bin/env python3
from pathlib import Path
import sys
root = Path(__file__).resolve().parents[1]
for rel in ["provider-certification/RESULTS.md", "hardware/CERTIFICATION_RESULTS.md"]:
    path = root / rel
    if not path.is_file():
        print(f"certification validation: FAIL - missing {rel}", file=sys.stderr)
        raise SystemExit(1)
    text = path.read_text(encoding="utf-8")
    if "RELEASE-BLOCKING-PENDING" in text:
        print(f"certification validation: FAIL - pending row in {rel}", file=sys.stderr)
        raise SystemExit(1)
print("certification validation: ok")
