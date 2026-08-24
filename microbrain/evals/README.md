# microbrain/evals

EP-041-owned data root for Microbrain evaluation suites (SPEC-025).

Holds `FrozenEvalSuite` records: the frozen hidden test set that
predates training and is never used for gradient updates or prompt
iteration, plus adversarial eval suites. Every frozen eval must carry
`created_before_training=true` at the contract boundary
(`python/nexus_microbrain/`).
