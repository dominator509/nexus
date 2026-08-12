# ADR-006: Canonical snake_case wire names and pinned security tools

Status: Accepted
Date: 2026-08-12
Node: EP-002 (M2)

## Context

EP-002 M2 checkpoint demanded proof that:

1. Canonical JSON Schemas and generated Rust, TypeScript, Python, and Dart
   wire models agree on field names, types, nullability, required fields,
   enum values, UUID formats, additional-properties behavior, and
   schema-version types/constants.
2. `schema_version` is not left as an unconstrained `serde_json::Value` when
   the canonical schema requires a string or constant.
3. Language-safe identifiers such as Python `class_` preserve the canonical
   serialized field name through aliases and round-trip tests.
4. Any camelCase conversion is consistent with the actual canonical wire
   schema across all generated languages.
5. cargo-deny 0.20.2 is pinned consistently in every applicable surface so a
   fresh clone reproduces the working toolchain.

## Decision

### 1. Canonical wire names are schema property names verbatim (snake_case)

The JSON Schema documents under `schemas/` are the single cross-language
contract source (`schemas/README.md`). Their property names are snake_case
(`schema_version`, `approval_required`, `required_capabilities`, `class`).
A payload serialized with camelCase names would fail schema validation under
`additionalProperties: false`.

The generator previously converted wire names to camelCase for Rust
(`#[serde(rename_all = "camelCase")]`) and TypeScript (`camel(pname)`), while
Python kept snake_case - a cross-language inconsistency that also diverged
from the canonical schemas. Fix: the generator now emits the schema property
name verbatim as the wire name in all four languages. No camelCase conversion
is applied anywhere.

Language-specific keyword safety is handled with explicit aliases that map
back to the canonical name:

- Python: `class_` identifier; `WIRE_ALIASES` + `to_wire`/`from_wire` helpers
  rename to `class` on the wire; round-trip tests prove it.
- Dart: `class_` identifier; `fromJson`/`toJson` use the canonical `class`
  key.

### 2. `schema_version` const is typed, never an unconstrained Value

`nexus-control-object.schema.json` pins `schema_version` to
`{"const": "1.0.0"}`. The generator previously emitted `serde_json::Value`
(Rust), `unknown` (TS), and `object` (Python). Fix: the generator infers the
const's type and emits:

- Rust: `String` (with `deny_unknown_fields` when `additionalProperties:
  false`), enforced to the exact constant in the validated wrapper
  (`ValidationError::Const`).
- TypeScript: literal type `"1.0.0"`.
- Python: `Literal["1.0.0"]`.
- Dart: `String` plus a `static const` member carrying the constant value.

### 3. `$ref` properties resolve to generated type names

`action-request.schema.json` references `invocation-context.schema.json` via
`$ref`. The generator now resolves `$ref` to the generated type
(`InvocationContext`) in all languages instead of falling back to
`serde_json::Value`/`unknown`/`object`/`dynamic`.

### 4. Security tools are pinned like every other locked component

cargo-deny 0.20.2 (required for CVSS 4.0 advisory vectors; see EP-001 M5) and
cargo-audit 0.22.2 were ambient `~/.cargo/bin` installs outside any lock.
They are now pinned in:

- `VERSIONS.lock.yaml` (component records, `policy: pinned`)
- `references/SOURCE_VERIFICATION.json` (verified source records;
  cargo-deny 0.20.2 from EmbarkStudios/cargo-deny, MIT OR Apache-2.0,
  released 2026-07-09; cargo-audit 0.22.2 from rustsec/rustsec)
- `scripts/toolchain-check.sh` (preflight verifies exact versions)
- `scripts/install.sh` (pinned `cargo install --version`)
- `scripts/dependency-audit.sh` (version guard before running)
- `.github/workflows/ci.yml` (install step + dependency-audit job)
- This ADR and the execplan decision log (decision surface)

## Consequences

- Wire names now agree with the canonical schemas in Rust, TypeScript,
  Python, and Dart; a JSON Schema validator accepts generated payloads.
- `schema_version` is a typed constant everywhere; the validated wrapper
  rejects non-conforming values with a precise `ValidationError::Const`.
- Existing tests that asserted camelCase wire names were corrected to the
  canonical snake_case names (they had encoded the drift).
- Fresh clones reproduce the security toolchain exactly (version-pinned
  install + preflight verification).
- No gate was weakened; the generator is still deterministic and prettier
  byte-identical (verified by `generated_contracts_match` and
  `pnpm exec prettier --check`).
