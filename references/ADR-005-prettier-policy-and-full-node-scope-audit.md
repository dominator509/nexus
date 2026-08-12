# ADR-005: Prettier Policy and Full-Node Scope Audit

## Status

Accepted (2026-08-12, during EP-001 M5 corrective closure).

## Context

Two gate defects surfaced during EP-001 closure:

1. **Prettier false-fail on immutable pack documents.** `pnpm exec prettier --check .`
   flagged 130 files, including S0/S1 GraphLock control documents shipped by the
   blueprint pack (`.agent/**`, `.clinerules/**`, `CLAUDE.md`, `GEMINI.md`,
   `OPENCLAW.md`, `.github/copilot-instructions.md`, `ASSUMPTIONS.md`,
   `ENVIRONMENT.md`, `OPERATIONS.md`, `LIVE_FIRE_PROOFS.md`). These documents are
   governed by `scripts/blueprint_validate.py` (schema/ASCII/placeholder checks),
   not by Prettier. Rewriting them with `prettier --write` would be a scope
   violation and would churn immutable L1/L2 content. Separately, `pnpm-lock.yaml`
   is package-manager-owned and must never be hand-formatted.

2. **Scope audit only examined `HEAD~1`.** The original
   `scripts/scope-audit.sh` ran `git diff --name-only HEAD~1`, so an out-of-fence
   path introduced in an earlier milestone commit and untouched by the last commit
   was invisible to the audit. This is a real leak class: during EP-001,
   `references/ADR-004-blueprint-validator-dependency-aware-scanning.md` was
   committed inside EP-001's window but was never added to the EP-001 fence; only
   the strengthened baseline-from-first-milestone audit caught it.

## Decision

### A. Prettier policy

1. `.prettierignore` is an explicit EP-001 M5 file (added to Expected Changed
   Files, the M5 CHANGE list, and `.agent/expected-files/EP-001.txt`).
2. The ignore list covers: vendored/build/cache paths already justified by
   ADR-004 doctrine, pack-controlled documentation (`.agent/**`,
   `.clinerules/**`, `CLAUDE.md`, `GEMINI.md`, `OPENCLAW.md`,
   `.github/copilot-instructions.md`, `ASSUMPTIONS.md`, `ENVIRONMENT.md`,
   `OPERATIONS.md`, `LIVE_FIRE_PROOFS.md`, and the other root control docs), and
   `pnpm-lock.yaml` (package-manager-owned).
3. First-party code remains covered: the generated TypeScript binding, `apps/`,
   `packages/`, `tests/`, `infra/devcontainer/devcontainer.json`, and
   `docs/` are NOT ignored.
4. The TypeScript generation pipeline (`packages/contracts/scripts/generate.py`)
   now emits deterministic Prettier-compliant output: enum unions lose redundant
   parens and long unions break at the 80-column boundary in Prettier's chain
   style. Regeneration followed by formatting is idempotent (verified: generator
   output is byte-identical to `prettier` output; running the generator twice
   produces no diff).
5. `infra/devcontainer/devcontainer.json` was formatted with `prettier --write`
   as first-party configuration.

### B. Full-node scope audit

`scripts/scope-audit.sh` was strengthened without weakening any rule:

1. Find the earliest commit whose subject starts with `[<NODE>][M`.
2. Use that commit's parent as the node baseline.
3. Audit all committed paths from baseline through HEAD.
4. Audit staged and unstaged paths too (`git diff --cached`, `git diff`,
   `git ls-files --others --exclude-standard`).
5. Treat `.agent/state/LEDGER.md` and `.agent/state/evidence/**` as governed
   always-writable L6 state.
6. Reject every other path outside `.agent/expected-files/<NODE>.txt`.
7. Print every unauthorized path and exit nonzero.
8. A regression test (`tests/scope-audit-regression.sh`) proves a path introduced
   in an earlier milestone commit is detected even when the last commit is clean.
9. The public command and sentinel are preserved:
   `sh scripts/scope-audit.sh EP-001` -> `scope audit EP-001: ok`.

The strengthened audit immediately caught `references/ADR-004-...` as an
out-of-fence path committed inside EP-001's window; it is retained via this ADR
and an exact-path fence entry rather than a broad directory wildcard.

## Consequences

- Prettier gate validates first-party code only; immutable pack documents remain
  governed by `blueprint_validate.py`.
- Scope audit is now a full-node audit; no out-of-fence path can hide in an
  earlier milestone commit.
- Compatibility: no gate was weakened; the audit is strictly stronger. Security:
  full-path visibility reduces scope-drift risk. Rollback: revert `.prettierignore`
  entries or the scope-audit rewrite, re-run gates. Scope impact: exact-path fence
  additions for `.prettierignore`, `scripts/scope-audit.sh`,
  `tests/scope-audit-regression.sh`, `COMMANDS.md`, and the EP-001 ADR files.
