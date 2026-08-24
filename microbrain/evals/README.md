# microbrain/evals

EP-041-owned data root for Microbrain evaluation suites (SPEC-025).

Holds `FrozenEvalSuite` records: the frozen hidden test set that
predates training and is never used for gradient updates or prompt
iteration, plus adversarial eval suites. Every frozen eval must carry
`created_before_training=true` at the contract boundary
(`python/nexus_microbrain/`) and the M3 behavior layer
(`python/nexus_microbrain/eval_policy.py`) enforces the timing,
immutability, dataset-policy binding, and deterministic scoring gates.

## suites/

Real JSON eval suite fixtures for the EP-041 M3 deterministic behavior
tests. These are LOCAL TEST FIXTURES - they are not real model
evaluation results and no model has been scored with them.

- `nexus-frozen-suite-v1.eval.json` - 11 frozen evals covering all 8
  narrow NexusControlObject roles, 3 out-of-distribution items, and
  2 hard-negative items, all frozen before the 2026-08-10 training
  start.
- `nexus-frozen-suite-v1.binding.json` - sidecar binding to dataset
  `nexus-synthetic-role-ops-v1` with the real sha256 digest of the
  committed dataset manifest.
