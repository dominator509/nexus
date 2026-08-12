# EP-001 Operations: Polyglot Workspace and Contract Pipeline

Owned components: polyglot workspace manifests, generated-contract pipeline,
stage-aware gates, CI skeleton. This node owns no long-running service, so
the operations contract covers build, test, and regeneration workflows.

## Health

- `sh scripts/preflight.sh` must print `preflight: ok`.
- `python3 scripts/blueprint_validate.py` must print `blueprint validation: ok`.
- `sh scripts/reality-gate.sh` must print `reality gate: ok`.
- A failed gate is a health failure; do not proceed past it.

## Readiness

- `sh scripts/nodes/EP-001.sh verify` runs every gate chain (lint, typecheck,
  unit, failure, integration, security, license, reality). It exits non-zero on
  any failure and prints `EP-001 verify: ok` only when all gates pass.

## Regeneration (generated contracts)

- Contracts are generated from `schemas/` by
  `packages/contracts/scripts/generate.py` into Rust (`crates/nexus-contracts/`),
  TypeScript (`packages/contracts/src/generated.ts`), and Python
  (`python/nexus_contracts/generated.py`).
- `python3 packages/contracts/scripts/generate.py` regenerates all three.
- `--check` verifies committed bindings are current; the unit and integration
  suites enforce this (`generated_contracts_match` / `generated_bindings_are_current`).
- Never hand-edit generated files; change `schemas/` and regenerate.

## Dependency install

- Rust: `cargo generate-lockfile` after changing `Cargo.toml`; commits must
  include `Cargo.lock`.
- TypeScript: `pnpm install --no-frozen-lockfile` only when intentionally
  updating the lockfile; normal CI uses the frozen lockfile. `allowBuilds`
  in `pnpm-workspace.yaml` governs postinstall scripts.
- Python: `uv sync --extra dev` after changing `pyproject.toml`; commits must
  include `uv.lock`. The project installs as a package (`package = true`) so
  `nexus_contracts` is importable.

## Backup and restore

- Source of truth is git. Every milestone is committed; `green/*` tags mark
  completed nodes. Restore a node by checking out its prior green tag or
  milestone commit and re-running the boot sequence.
- Generated files are reproducible from `schemas/`; regenerate rather than
  restore stale bindings.

## Upgrade

- Toolchain versions are locked in `mise.toml` / `.tool-versions` and verified
  by `sh scripts/version-verify.sh`. Upgrade only by ADR and VERSIONS.lock.yaml
  change, then re-provision via mise and the devcontainer.

## Disable

- A workspace member can be excluded by removing it from the workspace
  manifests (`Cargo.toml` members, `pnpm-workspace.yaml` packages,
  `pyproject.toml`/`[tool.hatch]`) without touching other members.

## Rollback

- Rollback to the previous milestone commit under `.agent/LOOPS.md`; never
  cross a completed green tag. The gate chain is the rollback gate: a commit
  that fails any gate is rolled back before the next milestone.
