# Nexus Agent Skills (SPEC-010 behavior 6; ADR-025; EP-018)

This directory holds portable Agent Skills packages. A skill is an
immutable, signed, versioned bundle: Nexus metadata in `manifest.json`,
a single payload file, and the canonical content hash computed from the
payload at load time (scan before install).

## Bundle layout

```
skills/
  <namespace>/<skill-name>/<version>/
    manifest.json   # canonical SkillManifest JSON (SPEC-010 behavior 6)
    SKILL.md        # payload (what the skill does); hash = content_hash
```

`manifest.json` serializes the canonical `SkillManifest` contract from
`crates/nexus-skills` (ADR-025): skill id, tenant, name, semantic
version, description, declared permissions, dependencies, network
rules, license, provenance, trust tier, and a signature.

## Authority semantics (ADR-025)

- Declared permissions are REQUESTS, never grants. Effective authority
  is the intersection of the closure's declared requirements, the
  caller's grants, the tenant policy allowance, and the trust ceiling.
- A signature is an integrity statement. It is NOT trust, NOT an
  authorized installation, and NOT execution permission. Structural
  signature validation is contract-level (M2); real cryptographic
  verification is owned by the M3 behavior boundary.
- Community skills begin inspect-only or sandboxed; they can never
  request privileged host authority beyond their ceiling.
- Network rules are requested constraints, not automatic network
  access.
- A package is immutable by version: changed content under the same
  name/version is a conflict, never a silent mutation.

## Built-in skills

| Skill             | Trust tier | Declared permissions | Notes                         |
| ----------------- | ---------- | -------------------- | ----------------------------- |
| `nexus/summarize` | SANDBOXED  | READ                 | Read-only summarization       |
| `nexus/notify`    | SANDBOXED  | READ, WRITE          | Requires a TRUSTED+ installer |
| `community/echo`  | SANDBOXED  | NONE                 | Community skill, sandboxed    |

Signatures in built-in bundles use a controlled, well-formed test key
(hex-encoded, structurally valid per ADR-025). Real key material and
cryptographic verification are owned by the M3 behavior boundary; these
bundles are CONTROLLED_TEST_FIXTURE inputs for the load/scan pipeline
that M2 proves.
