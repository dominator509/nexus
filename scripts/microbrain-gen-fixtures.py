#!/usr/bin/env python3
"""Generate EP-041 M4 training fixtures through the real M1/M4 models.

The candidate manifest and training plan manifest are written through
the real nexus_microbrain contract so the committed files are
guaranteed contract-valid (same pattern as the M3 eval fixture). These
are LOCAL TEST FIXTURES, never production training data.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "python"))

from nexus_microbrain import (  # noqa: E402
    CandidateStatus,
    QuantizationFormat,
    Role,
    TrainingCandidate,
    build_training_plan,
)

OUT_DIR = REPO_ROOT / "microbrain" / "training" / "plans"
OUT_DIR.mkdir(parents=True, exist_ok=True)

CANDIDATE_JSON = OUT_DIR / "nexus-candidate-v1.candidate.json"
PLAN_JSON = OUT_DIR / "nexus-training-plan-v1.plan.json"

candidate = TrainingCandidate(
    candidate_id="nexus-candidate-v1",
    role=Role.INTERPRETATION,
    model_ref="nexus-microbrain-interpretation-1",
    base_model="deepseek-v3-base",
    dataset_ref="nexus-synthetic-role-ops-v1",
    status=CandidateStatus.CANDIDATE,
)

plan = build_training_plan(
    plan_id="nexus-training-plan-v1",
    candidate_ref=candidate.candidate_id,
    role=candidate.role,
    base_model=candidate.base_model,
    quantization_format=QuantizationFormat.GGUF,
    hyperparameters={"rank": 16, "alpha": 32, "seed": 7},
    created_at="2026-08-11T00:00:00Z",
)

CANDIDATE_JSON.write_text(
    json.dumps(candidate.to_dict(), indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
PLAN_JSON.write_text(
    json.dumps(plan.to_dict(), indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)

print(f"wrote {CANDIDATE_JSON}")
print(f"wrote {PLAN_JSON}")
print(f"candidate_id={candidate.candidate_id}")
print(f"plan_digest={plan.plan_digest}")
