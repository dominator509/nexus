# Skill failure suite (EP-018 M4; SPEC-010; ADR-025)

This directory documents the forced-failure and abuse suite for the
skill plane. The executable tests live in
`crates/nexus-skills/tests/ep018_failure_suite.rs` (cargo-discovered,
names prefixed `ep018_failure_`); this README is the operations and
observability companion.

## Real failure mechanisms (no mocks of the proven component)

| Failure class          | Real mechanism                                                                      | Proven behavior                                                                                                                               |
| ---------------------- | ----------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Partial side effect    | A store whose `save` fails (controlled failure at the persistence port)             | `install_bundle` rolls back the in-memory registration; `remove` restores the entry; `revoke` undoes the flag. Memory and disk never diverge. |
| Malformed input        | Corrupted `manifest.json` on disk; unknown enum value; malformed signature encoding | Loader fails closed with `VALIDATION`.                                                                                                        |
| Unavailable dependency | Missing bundle directory; corrupted persisted state file                            | `NOT_FOUND` / `VALIDATION` fail-closed.                                                                                                       |
| Duplicate request      | Same identity re-install; tampered content under the same version                   | Idempotent duplicate; `CONFLICT` on changed content (immutable by version).                                                                   |
| Denied permission      | WRITE declared at SANDBOXED ceiling; TRUSTED skill at a SANDBOXED caller            | `POLICY` denial; nothing persisted.                                                                                                           |
| Execution boundary     | Revoked or missing version resolution                                               | `resolve_for_execution` fails closed (`POLICY` revoked / `NOT_FOUND` missing).                                                                |
| Tamper                 | Content hash changed after scan                                                     | `CONFLICT` at registration; signature tamper never reaches execution.                                                                         |
| Redaction              | Secret-like manifest content                                                        | Error messages never contain manifest content.                                                                                                |
| Recovery               | `SkillRegistry::clear(store)`                                                       | Bounded recovery: explicit, persisted, never reconstructs authority.                                                                          |

## Operations diagnostic

- `SkillRegistry::resolve_for_execution(name, version)` is the
  execution boundary: missing and revoked versions are never
  executable.
- `SkillRegistry::clear(store)` is the bounded recovery command for
  the registry; it persists the empty state and does not re-grant any
  authority.

## Observability

All errors are typed `SkillPackageError` (SPEC-006 codes) with fixed,
redacted messages; no manifest/package content leaks into error output.
