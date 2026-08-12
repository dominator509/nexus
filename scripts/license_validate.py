#!/usr/bin/env python3
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
registry = (root / "COMPONENT_REGISTRY.yaml").read_text(encoding="utf-8")
policy = (root / "LICENSE_POLICY.md").read_text(encoding="utf-8")
required = ["license:", "integration_mode:", "replacement_contract:", "commercial_review:"]
for marker in required:
    if marker not in registry:
        print(f"license validation: FAIL - component registry lacks {marker}", file=sys.stderr)
        raise SystemExit(1)
if "AGPL" in registry and "isolated-sidecar" not in registry:
    print("license validation: FAIL - AGPL component lacks isolated sidecar boundary", file=sys.stderr)
    raise SystemExit(1)
if "GPL" in registry and "copyleft" not in policy.lower():
    print("license validation: FAIL - GPL policy absent", file=sys.stderr)
    raise SystemExit(1)
print("license validation: ok")
