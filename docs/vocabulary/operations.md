# Nexus Contract Layer Operations (EP-002)

Owned components: `crates/nexus-domain`, `crates/nexus-contracts`,
`packages/contracts`, `python/nexus_contracts`, `schemas/`, and the generated
Dart binding `packages/contracts/src/generated.dart`.

## Health

- Generated bindings are current: `python3 packages/contracts/scripts/generate.py --check`
  prints `contract generation: ok (9 schemas, 4 languages)`.
- Determinism is proven by `cargo test -p nexus-contracts generated_contracts_match`
  (regeneration must be byte-identical).
- Cross-language agreement: `uv run --frozen pytest tests/unit/test_ep002_unit_agreement.py -q`
  must report 7 passed.
- Dart binding health: `dart analyze packages/contracts/src/generated.dart`
  must report `No issues found!`.

## Readiness

- Boot sequence: `sh scripts/preflight.sh` prints `preflight: ok`.
- Toolchain: `sh scripts/toolchain-check.sh` verifies cargo-deny 0.20.2 and
  cargo-audit 0.22.2 exactly (CVSS 4.0 advisory parsing).
- Node gate: `sh scripts/nodes/EP-002.sh M5` prints `EP-002 M5: ok`.

## Backup and restore

- The contract layer is source + generated artifacts in git. The canonical
  source is `schemas/`; every generated binding is reproducible with
  `python3 packages/contracts/scripts/generate.py`.
- Restore: checkout the target commit, run `cargo fetch --locked`,
  `pnpm install --frozen-lockfile`, `uv sync --frozen`, then regenerate and
  diff: `python3 packages/contracts/scripts/generate.py --check`.

## Upgrade

- Schema changes: edit `schemas/*.json` (breaking changes require a new
  schema `$id` and a compatibility adapter), then run
  `python3 packages/contracts/scripts/generate.py` and commit ALL generated
  bindings together.
- Toolchain: advance `VERSIONS.lock.yaml` + `references/SOURCE_VERIFICATION.json`
  together, update `scripts/toolchain-check.sh` version guards, and record an
  ADR + ledger entry. Never pin an old RustSec advisory DB (hides advisories).

## Disable

- Remove the workspace member from the root `Cargo.toml` members list and
  delete the package directory; remove `packages/contracts` from
  `pnpm-workspace.yaml`; remove `python/nexus_contracts` from
  `pyproject.toml` build packages. Update the fence
  (`.agent/expected-files/EP-002.txt`) and ledger before the next gate.

## Rollback

- Milestone rollback: `git checkout <previous-milestone-commit>` under
  LOOPS.md; never cross a completed green tag.
- Wire-format rollback: reverting ADR-006 restores camelCase emission; the
  agreement tests and round-trip tests will fail until the generator and all
  four bindings are regenerated consistently.

## Verification commands

```
sh scripts/preflight.sh
sh scripts/nodes/EP-002.sh M5
sh scripts/node-verify.sh EP-002
sh scripts/scope-audit.sh EP-002
sh scripts/security-check.sh
sh scripts/license-gate.sh
python3 packages/contracts/scripts/generate.py --check
pnpm --filter @nexus/contracts test:unit
uv run --frozen pytest tests/unit -q
dart analyze packages/contracts/src/generated.dart
```
