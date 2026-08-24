# microbrain/training

EP-041-owned data root for Microbrain training runs (SPEC-025).

Holds `QloraRun` records and adapters for the separate QLoRA pipeline.
Reproducibility is contract-level: every run carries seed and
config_digest so the factory is reproducible and safe even if no model
meets promotion thresholds (DeepSeek remains the functioning V1
provider).

## M4 fixtures (local test fixtures, never production data)

- `plans/nexus-candidate-v1.candidate.json` - real `TrainingCandidate`
  contract record (role INTERPRETATION, dataset_ref
  nexus-synthetic-role-ops-v1), generated through the real M1 model.
- `plans/nexus-training-plan-v1.plan.json` - real deterministic
  `TrainingPlan` (GGUF, rank 16 / alpha 32 / seed 7) with its canonical
  `plan_digest`; PLAN_READY only - a plan never implies training
  executed or a QLoRA run certified.

Regenerate with: `python3 scripts/microbrain-gen-fixtures.py`

Boundary: TRAINING PLAN EXISTS != TRAINING EXECUTED; QLORA RUN EXISTS !=
TRAINING CERTIFIED; CANDIDATE EXISTS != ELIGIBLE TO TRAIN.
