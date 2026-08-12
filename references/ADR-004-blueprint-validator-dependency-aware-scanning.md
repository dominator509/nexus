# ADR-004: Blueprint Validator Dependency-Aware Scanning

## Status

Accepted (2026-08-12, during EP-001 execution).

## Context

`scripts/blueprint_validate.py` scanned every file under the repository root
for (a) non-ASCII text and (b) unresolved doubled-left-brace placeholders.
During the pre-EP-001 source-only tree this was correct and caught real pack
defects. After EP-001 provisioned real language dependencies, the scan began
false-positiving on vendored third-party content:

* `node_modules/.pnpm/vite@7.3.6/node_modules/vite/README.md` contains
  legitimate non-ASCII text - vendored dependency, not blueprint drift.
* `packages/contracts/scripts/generate.py` uses Python f-strings
  (e.g. emitting `pub struct NAME {` for Rust) that legitimately emit single
  braces for generated Rust/TypeScript source - a code generator, not an
  unresolved template placeholder.

## Decision

Two precise exemptions, no gate weakening:

1. Skip a fixed set of dependency/vendor directories for the text scan:
   `.git`, `node_modules`, `.venv`, `venv`, `target`, `dist`, `build`,
   `.mise`, `.cache`, `__pycache__`, `.pytest_cache`, `.mypy_cache`,
   `.ruff_cache`, `.dart_tool`, `coverage`. These are never blueprint
   source; the pack's `.gitignore` already excludes them from commits.
2. Skip source-code files (`.py`, `.rs`, `.ts`, `.js`) for the double-brace
   placeholder check only. The ASCII check still applies to every file.
   Double-brace scanning remains fully active for documentation, YAML,
   manifests, and template content - the files where an unresolved
   doubled-left-brace placeholder (the kind a templating engine would
   substitute) indicates pack drift.

## Consequences

- Blueprint validation no longer fails on vendored dependencies or on
  code generators that emit brace syntax.
- The gate still fails on any genuine unresolved placeholder in docs or
  config, and on any non-ASCII drift in first-party source.
- `.gitignore` already protects vendor directories from being committed;
  this ADR aligns the validator with the same boundary.

## Verification

`sh scripts/nodes/EP-001.sh M2` passes the blueprint validation gate with
the full dependency tree present.
