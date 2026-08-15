# Canonical skill schemas (SPEC-010; ADR-025; EP-018 M3)

These JSON Schema 2020-12 documents are the canonical cross-language
contract for Agent Skills. They are the machine-readable pair of the
Rust contract crate `crates/nexus-skills`; drift between the Rust
serde surface and these documents fails the `ep018_integration_schema`
suite (real `jsonschema` validator, EP-010 M3 pattern).

## Documents

| Schema | Contract | Rust type |
| ------ | -------- | --------- |
| `skill-manifest.schema.json` | Nexus metadata for a skill: id, tenant, name, semantic version, declared permissions, dependencies, network rules, license, provenance, trust tier, signature | `SkillManifest` |
| `skill-package.schema.json` | Immutable signed versioned package: manifest + real SHA-256 `content_hash` + created time | `SkillPackage` |
| `skill-registry-state.schema.json` | Persisted registry state (entries with terminal `revoked` flag) | `SkillRegistryState` |

## Authority semantics (ADR-025)

- `permissions` are declared REQUESTS; authorization requires caller
  grant, tenant policy, and the trust ceiling. The schema enforces
  uniqueness but not authority (authority is enforced by the registry).
- `content_hash` is a 64-char lowercase hex SHA-256 of the bundle
  payload, computed at scan-before-install time.
- Names are `namespace/skill-name`; versions are strict
  `major.minor.patch`; ids are canonical lowercase UUIDv7.
- Signatures are structurally constrained (hex key/signature lengths by
  algorithm); real cryptographic verification is owned by the M3
  behavior boundary and is not expressed in JSON Schema.
