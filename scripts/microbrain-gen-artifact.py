#!/usr/bin/env python3
"""Generate EP-041 M5 artifact fixtures through the real M1/M5 models.

The QuantizedArtifact manifest is written through the real
nexus_microbrain contract with the REAL sha256 digest of the committed
fixture-only GGUF marker file, so the committed files are guaranteed
contract-valid and digest-bound. These are LOCAL TEST FIXTURES - the
marker is fixture-only and must never be certified as production model
output (real GGUF quantization NOT ASSERTED).
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "python"))

from nexus_microbrain import (  # noqa: E402
    ArtifactStatus,
    QuantizationFormat,
    QuantizedArtifact,
)

OUT_DIR = REPO_ROOT / "microbrain" / "artifacts" / "fixtures"
OUT_DIR.mkdir(parents=True, exist_ok=True)

MARKER = OUT_DIR / "nexus-artifact-v1.gguf.marker"
ARTIFACT_JSON = OUT_DIR / "nexus-artifact-v1.artifact.json"

# Fixture-only GGUF marker: not a real quantized model, labeled as such.
marker_body = (
    "# EP-041 M5 fixture-only GGUF marker (local test fixture)\n"
    "# This is NOT a real quantized model. Real GGUF quantization NOT\n"
    "# ASSERTED. Do not certify this file as production model output.\n"
    "fixture-only\n"
).encode("utf-8")
MARKER.write_bytes(marker_body)

digest = "sha256:" + hashlib.sha256(marker_body).hexdigest()

artifact = QuantizedArtifact(
    artifact_id="nexus-artifact-v1",
    candidate_ref="nexus-candidate-v1",
    format=QuantizationFormat.GGUF,
    quantization="Q4_K_M",
    digest=digest,
    size_bytes=len(marker_body),
    license_ref="nexus-synthetic-mit",
    status=ArtifactStatus.BUILT,
)

ARTIFACT_JSON.write_text(
    json.dumps(artifact.to_dict(), indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

print(f"wrote {MARKER}")
print(f"wrote {ARTIFACT_JSON}")
print(f"artifact_id={artifact.artifact_id}")
print(f"digest={digest}")
