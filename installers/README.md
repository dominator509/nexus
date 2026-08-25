# EP-042 M4 Local Installer (SPEC-016, SPEC-024)

## Component record

| Field                | Value                                                                                                                |
| -------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Component            | Nexus local installer (`@nexus/installers`)                                                                          |
| License              | MIT (repository license)                                                                                             |
| Replacement contract | SPEC-016 UpdateTransaction; canonical truth remains in `crates/nexus-release` (M1) and `apps/setup/src/update/` (M2) |
| Transport boundary   | `infra/release/` (M3) - digest-bound publish/fetch over a real S3 gateway                                            |
| Registry             | workspace package (pnpm `installers`)                                                                                |

## What this package owns

`installers/` is the EP-042 M4 local execution boundary:

- `src/installer.ts` - transactional install (manifest validation, backup-before-update, staged replacement, digest validation, atomic switch, verification), real rollback, bounded recovery, foreign-root cleanup guard
- `src/backup.ts` - real backup execution (bytes copied, real sha256 digest, verified)
- `src/paths.ts` - abuse-case guards: path traversal, symlink escape, duplicate overwrite, foreign-root cleanup
- `src/journal.ts` - append-only installer journal with typed state transitions
- `src/observability.ts` - current-run redacted installer evidence
- `src/errors.ts` - typed failure classification (17 classes)
- `src/cli.ts` - real CLI (install / rollback / recover / status)
- `scripts/` - real POSIX installer scripts (`installer-install.sh`, `installer-rollback.sh`, `installer-recover.sh`, `installer-status.sh`)

## Installer invariants

- `INSTALLER EXISTS != INSTALLER EXECUTED != INSTALLATION VERIFIED`
- `BACKUP REQUESTED != BACKUP COMPLETED != RESTORE VERIFIED`
- `ROLLBACK PLAN EXISTS != ROLLBACK EXECUTED != ROLLBACK PROVEN`
- `JOURNAL EXISTS != UPDATE COMPLETED`
- `RECOVERY ATTEMPTED != RECOVERY VERIFIED`
- `FAILURE INJECTED != SYSTEM HARDENED`
- `OBSERVABILITY EVENT EXISTS != RECOVERY PROVEN`
- Interruption before commit -> old state remains valid; interruption during staging -> staged state quarantined/removed.

## Certification boundary (honest)

- REAL local installer behavior on isolated temp roots: exercised by the M4 failure suite (real bytes, real chattr permission denial, real abort signals, real rollback).
- Production installer execution, production host upgrade, real release signing verification, canary rollout, production backup/restore, production rollback, offline bundle production, release build, deployment, remote synchronization: NOT ASSERTED.
- AWS/R2/B2 transport: NOT ASSERTED (only the local ephemeral SeaweedFS gateway is exercised in M3).
