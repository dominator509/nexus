# microbrain/datasets

EP-041-owned data root for Microbrain dataset artifacts (SPEC-025).

Holds versioned training datasets as `MicrobrainDataset` records:
TrainingExamples with canonical roles, provenance classes
(deterministic generation, teacher consensus, hard negatives, opted-in
scrubbed corrections), license references, and lineage. Dataset
contracts and policy live in `python/nexus_microbrain/`
(models.py + dataset_policy.py).

## manifests/

Real JSON dataset manifest fixtures for the EP-041 M2 deterministic
policy tests. These are LOCAL TEST FIXTURES with deterministic
generation / licensed teacher provenance - they are not production
training data and are never used to train a model. Every example
carries a license_ref; the policy gate (scripts/ep041-m2-tests.sh)
parses each manifest through the real M1 contract and evaluates it
through the real dataset policy.

- `nexus-synthetic-role-ops-v1.manifest.json` - 12 examples
  (10 deterministic generation + 2 hard negatives) covering all 8
  narrow NexusControlObject roles, 3 out-of-distribution examples.
- `nexus-teacher-consensus-v1.manifest.json` - 6 examples
  (4 teacher consensus + 2 opted-in scrubbed corrections) with
  license refs and correlation ids.

No runtime dependency of Nexus V1. Dataset artifacts are consumed only
by the separate Microbrain training factory.
