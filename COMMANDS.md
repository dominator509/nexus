# COMMANDS

## Working directory

Run every command from the repository root. Coding agents must not invent commands. If a command is missing or stale, update this file first, cite repository evidence in the active ExecPlan Decision Log, and commit the command change before using it.

## Non-interactive environment

Repository scripts source `scripts/env.sh` to establish the canonical
environment: the exports below plus the mise toolchain shims on PATH, so a
fresh noninteractive shell can run node verification without a manual
preamble (EP-003 M5). Equivalent manual exports:

```sh
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
export PNPM_HOME="${PNPM_HOME:-$HOME/.local/share/pnpm}"
export PATH="$HOME/.local/share/mise/shims:$HOME/.local/bin:$PATH"
```

## Legal commands

| Purpose | Exact command | Success sentinel |
| --- | --- | --- |
| Install tools | `sh scripts/install.sh` | `install: ok` |
| Preflight | `sh scripts/preflight.sh` | `preflight: ok` |
| Scheduler | `sh scripts/graph-next.sh` | one dispatch line |
| Ledger tail | `sh scripts/ledger.sh tail 30` | ledger lines |
| Current stage | `sh scripts/stage.sh current` | `STAGE EP-NNN` |
| Lint | `sh scripts/lint.sh` | `lint: ok` |
| Format check | `sh scripts/format-check.sh` | `format check: ok` |
| Typecheck | `sh scripts/typecheck.sh` | `typecheck: ok` |
| Unit tests | `sh scripts/test-unit.sh` | `unit tests: ok` |
| Failure tests | `sh scripts/test-failure.sh` | `failure tests: ok` |
| Integration tests | `sh scripts/test-integration.sh` | `integration tests: ok` |
| E2E tests | `sh scripts/test-e2e.sh` | `e2e tests: ok` |
| Build | `sh scripts/build.sh` | `build: ok` |
| Security check | `sh scripts/security-check.sh` | `security check: ok` |
| Dependency audit | `sh scripts/dependency-audit.sh` | `dependency audit: ok` |
| License gate | `sh scripts/license-gate.sh` | `license gate: ok` |
| Reality gate | `sh scripts/reality-gate.sh` | `reality gate: ok` |
| Smoke | `sh scripts/smoke-test.sh` | `smoke test: ok` |
| Active live-fire | `sh scripts/live-fire.sh` | `live-fire: ok` |
| Full verify | `sh scripts/verify.sh` | `verify: ok` |
| Production readiness | `sh scripts/production-readiness-check.sh` | `production readiness: ok` |
| Expected-file audit | `sh scripts/expected-files.sh EP-NNN` | `expected files EP-NNN: ok` |
| Node verify | `sh scripts/node-verify.sh EP-NNN` | `node verify EP-NNN: ok` |
| Shell syntax | `sh scripts/check-shell.sh` | `shell syntax: ok` |
| Clean-shell regression | `sh scripts/clean-shell-check.sh` | `clean shell check: ok` |
| Contract generation | `sh scripts/generate-contracts.sh` | `contract generation: ok` |
| Provider certification | `sh scripts/provider-certify.sh PROVIDER PROFILE` | `provider certification: ok` |
| Hardware certification | `sh scripts/hardware-certify.sh TARGET` | `hardware certification: ok` |
| Local core start | `sh scripts/local-start.sh core` | `local start core: ok` |
| Local full start | `sh scripts/local-start.sh full` | `local start full: ok` |
| Local stop | `sh scripts/local-stop.sh` | `local stop: ok` |
| Database migrate | `sh scripts/migrate.sh` | `migrate: ok` |
| Database seed test tenant | `sh scripts/seed-test-tenant.sh` | `seed test tenant: ok` |
| Backup | `sh scripts/backup.sh` | `backup: ok` |
| Restore drill | `sh scripts/restore-drill.sh` | `restore drill: ok` |
| Rollback drill | `sh scripts/rollback-drill.sh` | `rollback drill: ok` |
| Release build | `sh scripts/release-build.sh` | `release build: ok` |
| Deployment dry run | `sh scripts/deploy.sh --dry-run` | `deploy dry run: ok` |

## Targeted diagnostics

These are legal only after the owning workspace file exists:

- `cargo test -p <locked-crate-name> <locked-test-name> -- --nocapture`
- `cargo check -p <locked-crate-name>`
- `pnpm --filter <locked-package-name> test -- <locked-test-name>`
- `uv run pytest <locked-test-path> -q`
- `flutter test <locked-test-path>`
- `docker compose -f <locked-compose-file> logs --tail=100 <locked-service>`
- `docker run --rm --network <locked-network> --entrypoint temporal temporalio/admin-tools:1.31.2 operator cluster health --address temporal:7233` — real-server health gate for the EP-006 M3 integration suite (`tests/workflows/`); command surface verified against the pinned admin-tools image in the EP-006 Decision Log.
- `sh scripts/ep006-orphan-audit.sh` — post-suite orphan audit for the EP-006 M3 integration run: asserts zero `nexus-ep006-*` containers/networks, zero registered stack volumes (registry file `/tmp/nexus-ep006-stack-state.json` written by `tests/workflows/src/helpers/stack.ts`), and zero `temporal-server start` processes; fails the M3 gate if any EP-006 resource survives. Registered in the EP-006 Decision Log.

The exact crate, package, test, and compose names must already appear in an accepted spec, active ExecPlan, or checked-in workspace manifest.

## Agent adapter parity

```sh
for f in AGENTS.md CLAUDE.md GEMINI.md .github/copilot-instructions.md .cursor/rules/6layer.mdc .clinerules/6layer.md HERMES.md OPENCLAW.md; do awk '/PRIME-BLOCK-BEGIN/,/PRIME-BLOCK-END/' "$f" | cksum; done
```

Every checksum must match.

## Forbidden commands

- Interactive editors, REPLs, pagers, foreground watch processes, and prompt-on-conflict tools.
- `git push --force`, history rewrites, or deleting green tags.
- `git clean -fd` outside a plan-scoped recovery step with a recorded path list.
- `rm -rf` outside listed build directories or a disposable test environment.
- Destructive database commands against non-test databases.
- Production deploy commands because auto-deploy is not authorized.
- Installing unpinned dependencies or executing remote `curl | sh` installers.
- Skipping, ignoring, or masking a required test failure.

## Recovery

Use .agent/LOOPS.md. A missing command is fixed in COMMANDS.md before use. A stale generated file is regenerated from its canonical schema. A failed service uses its bounded readiness diagnostics. A provider or hardware integration that cannot be certified remains unavailable; it does not block the core profile unless the active node explicitly requires that profile.
