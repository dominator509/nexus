# microbrain/artifacts

EP-041-owned data root for Microbrain quantized artifacts (SPEC-025).

Holds `QuantizedArtifact` records in GGUF format. Identity is the
digest (`alg:hex`, >=32 hex chars); a name or tag is never the artifact
identity. Model, adapter, dataset, code, evaluation, and voice or
language licenses are recorded separately per artifact.
