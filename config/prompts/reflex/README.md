# Canonical Reflex Prompt Segments (SPEC-009 required behavior 4)

This directory is the canonical, versioned prompt segment catalog for
the reflex plane (EP-014, ADR-021). Every file is a canonical
`PromptSegmentVersion` payload: `segment`, `version`, `content`.

Order (immutable head first, volatile tail last):

1. `CONSTITUTION` - immutable constitution
2. `SCHEMAS` - canonical control-object schema
3. `CAPABILITY_TAXONOMY` - capability registry vocabulary
4. `RISK_POLICY` - risk classes and routing rules
5. `EXAMPLES` - canonical exemplars
6. `TENANT_CONTEXT` - stable tenant context
7. `SESSION_CONTEXT` - volatile session context (tail)
8. `DYNAMIC_REQUEST` - volatile dynamic request (tail)

`catalog.json` declares the canonical order and which segments are the
stable (cacheable) prefix. Volatile ids and timestamps stay in the tail.
The prefix is the cacheable corpus: stable bytes serialize identically
across requests and contribute to the 0.97 cache-hit target.

Canonical serialization (`PromptSegmentCatalog::canonical()`) fixes
segment order, version tagging, and whitespace, producing byte-stable
output. `crates/nexus-reflex` loads this directory through
`PromptSegmentCatalog::from_canonical_dir` and rejects missing,
out-of-order, or unversioned segments.
