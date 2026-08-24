# microbrain/datasets

EP-041-owned data root for Microbrain dataset artifacts (SPEC-025).

Holds versioned training datasets as `MicrobrainDataset` records:
TrainingExamples with canonical roles, provenance classes
(deterministic generation, teacher consensus, hard negatives, opted-in
scrubbed corrections), license references, and lineage. Dataset
contracts live in `python/nexus_microbrain/`.

No runtime dependency of Nexus V1. Dataset artifacts are consumed only
by the separate Microbrain training factory.
