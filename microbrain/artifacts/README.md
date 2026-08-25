# microbrain/artifacts

EP-041-owned data root for Microbrain quantized artifacts (SPEC-025).

Holds `QuantizedArtifact` records in GGUF format. Identity is the
digest (`alg:hex`, >=32 hex chars); a name or tag is never the artifact
identity. Model, adapter, dataset, code, evaluation, and voice or
language licenses are recorded separately per artifact.

## M5 fixtures (local test fixtures, never production output)

- `fixtures/nexus-artifact-v1.gguf.marker` - fixture-only GGUF marker.
  This is NOT a real quantized model. Real GGUF quantization NOT
  ASSERTED; this marker must never be certified as production model
  output.
- `fixtures/nexus-artifact-v1.artifact.json` - real `QuantizedArtifact`
  contract record (GGUF, Q4_K_M, candidate_ref nexus-candidate-v1)
  whose digest is the REAL sha256 of the committed marker file bytes,
  generated through the real M1 contract.

Regenerate with: `python3 scripts/microbrain-gen-artifact.py`

Boundary: GGUF ARTIFACT EXISTS != QUANTIZATION VERIFIED; DIGEST PRESENT
!= ARTIFACT VERIFIED; SHADOW PASSED != PROMOTED; PROMOTION DECISION !=
AUTONOMOUS DEPLOYMENT.
