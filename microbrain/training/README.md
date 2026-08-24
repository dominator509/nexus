# microbrain/training

EP-041-owned data root for Microbrain training runs (SPEC-025).

Holds `QloraRun` records and adapters for the separate QLoRA pipeline.
Reproducibility is contract-level: every run carries seed and
config_digest so the factory is reproducible and safe even if no model
meets promotion thresholds (DeepSeek remains the functioning V1
provider).
