#!/usr/bin/env python3
from pathlib import Path
import sys
root = Path(__file__).resolve().parents[1]
node = sys.argv[1]
max_milestone = int(sys.argv[2][1:])
for number in range(1, max_milestone + 1):
    manifest = root / ".agent/milestone-files" / f"{node}-M{number}.txt"
    if not manifest.is_file():
        print(f"node artifact check: FAIL - missing {manifest.relative_to(root)}", file=sys.stderr)
        raise SystemExit(1)
    for raw in manifest.read_text(encoding="utf-8").splitlines():
        path = raw.strip()
        if not path or path.startswith("#"):
            continue
        target = root / path.rstrip("/")
        if path.endswith("/") and not target.is_dir():
            print(f"node artifact check: FAIL - missing directory {path}", file=sys.stderr)
            raise SystemExit(1)
        if not path.endswith("/") and not target.exists():
            print(f"node artifact check: FAIL - missing {path}", file=sys.stderr)
            raise SystemExit(1)
print(f"node artifact check {node} M{max_milestone}: ok")
