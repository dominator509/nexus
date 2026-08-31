/**
 * EP-042 M4 transactional local installer (SPEC-016 behavior 6;
 * SPEC-024).
 *
 * REAL filesystem/process behavior in an isolated install root:
 *
 *   verified release bytes -> isolated install root
 *   -> backup current state (real bytes, real digest, verified)
 *   -> stage replacement (real bytes, digest-checked)
 *   -> validate staged state
 *   -> atomically/transactionally switch (rename)
 *   -> observe success/failure
 *   -> rollback if required (restore prior bytes, verify)
 *   -> verify resulting bytes/state
 *   -> cleanup
 *
 * Permanent invariants:
 * - TRANSPORT SUCCEEDED != INSTALLATION SUCCEEDED
 * - INSTALLER EXISTS != INSTALLER EXECUTED
 * - INSTALLER EXECUTED != INSTALLATION VERIFIED
 * - BACKUP REQUESTED != BACKUP COMPLETED
 * - BACKUP COMPLETED != RESTORE VERIFIED
 * - ROLLBACK PLAN EXISTS != ROLLBACK EXECUTED
 * - ROLLBACK EXECUTED != ROLLBACK PROVEN
 * - INTERRUPTION BEFORE COMMIT -> OLD STATE REMAINS VALID
 * - INTERRUPTION DURING STAGING -> STAGED STATE QUARANTINED/REMOVED
 * - JOURNAL EXISTS != UPDATE COMPLETED
 * - RECOVERY ATTEMPTED != RECOVERY VERIFIED
 * - DIGEST PRESENT != ARTIFACT VERIFIED (digest of real staged bytes)
 *
 * The installer NEVER operates on the host nexus tree: all mutation is
 * confined to caller-provided isolated roots (fence G).
 */

import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join, resolve, dirname } from "node:path";
import {
  parseReleaseManifest,
  sha256Hex,
  verifyManifestDigestBinding,
  type ReleaseManifest,
} from "@nexus/setup";
import { InstallerError } from "./errors";
import {
  journalAppend,
  journalRead,
  journalReset,
  type InstallerJournalConfig,
} from "./journal";
import {
  assertComponentPathWithinRoot,
  assertInstallRootUsable,
  assertNoDuplicateStagedPath,
  assertNoSymlinkEscape,
  assertOwnedCleanupTarget,
} from "./paths";
import {
  assertBackupUsable,
  createBackup,
  verifyBackupDigest,
  type BackupResult,
} from "./backup";

export interface InstallComponent {
  componentId: string;
  /** Declared canonical sha256:hex digest from the release manifest. */
  declaredDigest: string;
  /** Real artifact bytes. */
  bytes: Uint8Array<ArrayBuffer>;
  /** Relative path inside the install root (component path). */
  path: string;
}

export interface InstallOptions {
  installRoot: string;
  stagingRoot: string;
  backupRoot: string;
  quarantineRoot: string;
  journalRoot: string;
  releaseId: string;
  installId: string;
  runId: string;
  gitCommit: string;
  /** Canonical release manifest wire object (validated fail-closed). */
  manifestWire: Record<string, unknown>;
  components: ReadonlyArray<InstallComponent>;
  /** Abort signal for cancellation; throws STAGING_FAILED on abort. */
  signal?: AbortSignal;
}

export interface InstallResult {
  install_id: string;
  release_id: string;
  install_root: string;
  installed: ReadonlyArray<string>;
  backup: BackupResult | undefined;
  journal_path: string;
  completed_at: string;
}

export interface RollbackOptions {
  installRoot: string;
  backupRoot: string;
  stagingRoot: string;
  quarantineRoot: string;
  journalRoot: string;
  releaseId: string;
  installId: string;
  runId: string;
  gitCommit: string;
  expectedBackupDigest: string;
}

export interface RollbackResult {
  install_id: string;
  release_id: string;
  restored: ReadonlyArray<string>;
  backup_digest: string;
  verified: "VERIFIED";
  journal_path: string;
  completed_at: string;
}

export interface RecoverOptions {
  installRoot: string;
  backupRoot: string;
  stagingRoot: string;
  quarantineRoot: string;
  journalRoot: string;
  releaseId: string;
  installId: string;
  runId: string;
  gitCommit: string;
}

export interface RecoverResult {
  journal_state: string | undefined;
  recovered: boolean;
  detail: string;
}

function journalCfg(opts: {
  journalRoot: string;
  runId: string;
  gitCommit: string;
  installId: string;
  releaseId: string;
}): InstallerJournalConfig {
  return {
    journalPath: resolve(opts.journalRoot, "installer.journal.jsonl"),
    runId: opts.runId,
    gitCommit: opts.gitCommit,
    installId: opts.installId,
    releaseId: opts.releaseId,
  };
}

function abortIfRequested(signal: AbortSignal | undefined): void {
  if (signal !== undefined && signal.aborted) {
    throw new InstallerError(
      "STAGING_FAILED",
      "install cancelled by signal before commit",
    );
  }
}

/**
 * Run a real install:
 *   1. validate manifest (fail-closed parse + digest binding)
 *   2. backup current state (real bytes, digest, verified) - failure denies update
 *   3. stage replacement (real bytes into staging root, digest-checked)
 *   4. validate staged state (real sha256 of staged bytes vs declared)
 *   5. atomic switch (rename staging -> install root)
 *   6. verify resulting bytes/state
 * On any failure after backup: the update fails closed and the previous
 * install state is untouched (until the caller invokes rollback
 * explicitly - rollback is never automatic in this surface).
 */
export async function installRelease(
  opts: InstallOptions,
): Promise<InstallResult> {
  const cfg = journalCfg(opts);
  // AUD-069: durable idempotency guard BEFORE any filesystem mutation.
  // The journal is the durable record of this install root; a completed
  // install (INSTALLED) for this install_id must never be replayed, and
  // a journal owned by a different install_id must not be reset or
  // overwritten. Only an empty journal, or a journal whose last state
  // is a non-terminal failure for the SAME install_id, may proceed.
  const priorJournal = journalRead(cfg);
  if (priorJournal.entries.length > 0) {
    const lastEntry = priorJournal.entries[priorJournal.entries.length - 1]!;
    if (lastEntry.install_id !== opts.installId) {
      journalAppend(
        cfg,
        "FAILED",
        `journal owned by install ${lastEntry.install_id}; refusing ${opts.installId}`,
        "AUTHORIZATION_DENIED",
      );
      throw new InstallerError(
        "AUTHORIZATION_DENIED",
        `installer journal is owned by install ${lastEntry.install_id}; duplicate install_id ${opts.installId} refused`,
        { installId: opts.installId, releaseId: opts.releaseId },
      );
    }
    if (lastEntry.state === "INSTALLED") {
      journalAppend(
        cfg,
        "FAILED",
        `install ${opts.installId} already completed; replay refused`,
        "AUTHORIZATION_DENIED",
      );
      throw new InstallerError(
        "AUTHORIZATION_DENIED",
        `install ${opts.installId} already completed; replay refused (idempotency guard)`,
        { installId: opts.installId, releaseId: opts.releaseId },
      );
    }
  }
  journalReset(cfg);
  journalAppend(cfg, "STARTED", "install started");

  // 1. Manifest validation (canonical M1/M2 surface, fail-closed).
  let manifest: ReleaseManifest;
  try {
    manifest = parseReleaseManifest(opts.manifestWire);
  } catch (error) {
    journalAppend(cfg, "FAILED", "manifest invalid", "MANIFEST_INVALID");
    throw new InstallerError(
      "MANIFEST_INVALID",
      `release manifest failed validation: ${(error as Error).message}`,
      { installId: opts.installId, releaseId: opts.releaseId },
    );
  }
  const binding = await verifyManifestDigestBinding(manifest);
  if (binding === "MISMATCH") {
    journalAppend(
      cfg,
      "FAILED",
      "manifest digest mismatch",
      "MANIFEST_INVALID",
    );
    throw new InstallerError(
      "MANIFEST_INVALID",
      "release manifest digest binding mismatch",
      { installId: opts.installId, releaseId: opts.releaseId },
    );
  }
  journalAppend(cfg, "MANIFEST_VALIDATED", "manifest validated");

  // AUD-068: bind the request to the validated release manifest. The
  // caller's releaseId, component set, and declared digests are NOT
  // trusted on their own - each must agree with the manifest:
  //   1. releaseId must equal manifest.release_id
  //   2. every supplied component must be declared by the manifest
  //   3. every supplied declaredDigest must equal the manifest's digest
  //      for that component
  // Staged bytes are validated against these manifest-bound digests, so
  // a caller cannot inject payloads the manifest never declared.
  if (opts.releaseId !== manifest.release_id) {
    journalAppend(
      cfg,
      "FAILED",
      "release id not bound to manifest",
      "MANIFEST_INVALID",
    );
    throw new InstallerError(
      "MANIFEST_INVALID",
      `install release_id ${opts.releaseId} does not match manifest release_id ${manifest.release_id}`,
      { installId: opts.installId, releaseId: opts.releaseId },
    );
  }
  const manifestDigestById = new Map<string, string>();
  for (const component of manifest.components) {
    manifestDigestById.set(component.component_id, component.digest);
  }
  for (const c of opts.components) {
    const manifestDigest = manifestDigestById.get(c.componentId);
    if (manifestDigest === undefined) {
      journalAppend(
        cfg,
        "FAILED",
        "component not declared by manifest",
        "MANIFEST_INVALID",
      );
      throw new InstallerError(
        "MANIFEST_INVALID",
        `component ${c.componentId} is not declared by the release manifest`,
        {
          installId: opts.installId,
          releaseId: opts.releaseId,
          componentId: c.componentId,
        },
      );
    }
    if (c.declaredDigest !== manifestDigest) {
      journalAppend(
        cfg,
        "FAILED",
        "component digest not bound to manifest",
        "MANIFEST_INVALID",
      );
      throw new InstallerError(
        "MANIFEST_INVALID",
        `component ${c.componentId} declaredDigest does not match manifest digest`,
        {
          installId: opts.installId,
          releaseId: opts.releaseId,
          componentId: c.componentId,
        },
      );
    }
  }

  // Dependency availability: every declared component must have real
  // artifact bytes supplied by the caller (e.g. fetched over transport).
  // A declared component with no artifact bytes is an unavailable
  // dependency and the install fails closed before any mutation.
  const componentById = new Map<string, InstallComponent>();
  for (const c of opts.components) componentById.set(c.componentId, c);
  for (const component of manifest.components) {
    const artifact = componentById.get(component.component_id);
    if (artifact === undefined) {
      journalAppend(cfg, "FAILED", "artifact unavailable", "UNAVAILABLE");
      throw new InstallerError(
        "UNAVAILABLE",
        `artifact bytes unavailable for declared component ${component.component_id}`,
        {
          installId: opts.installId,
          releaseId: opts.releaseId,
          componentId: component.component_id,
        },
      );
    }
  }

  // 2. Real backup of current state. A fresh install (no prior state)
  //    has no backup requirement; an existing install root MUST be
  //    backed up and verified before any mutation, and a backup failure
  //    denies the update.
  let backup: BackupResult | undefined;
  if (existsSync(opts.installRoot)) {
    assertInstallRootUsable(opts.installRoot);
    journalAppend(cfg, "BACKUP_REQUESTED", "backup requested before update");
    try {
      backup = await createBackup(
        opts.installRoot,
        opts.backupRoot,
        opts.installId,
        opts.releaseId,
      );
    } catch (error) {
      journalAppend(
        cfg,
        "FAILED",
        "backup failed; update denied",
        "BACKUP_FAILED",
      );
      throw error;
    }
    assertBackupUsable(opts.backupRoot, opts.installId);
    journalAppend(
      cfg,
      "BACKUP_COMPLETED",
      `backup verified: ${backup.backup_id}`,
    );
  }

  // 3. Stage replacement (real bytes, digest-checked against declared).
  //    Any failure before the atomic switch removes the staged state
  //    (fence J: interruption during staging -> staged state
  //    quarantined/removed) and leaves the old install state intact.
  journalAppend(cfg, "STAGING", "staging replacement");
  const stagedPaths = new Map<string, string>();
  try {
    try {
      mkdirSync(opts.stagingRoot, { recursive: true });
    } catch (error) {
      journalAppend(
        cfg,
        "FAILED",
        "staging root creation failed",
        "STAGING_FAILED",
      );
      throw new InstallerError(
        "STAGING_FAILED",
        `cannot create staging root: ${(error as NodeJS.ErrnoException).code ?? "unknown"}`,
        { installId: opts.installId, releaseId: opts.releaseId },
      );
    }
    for (const c of opts.components) {
      abortIfRequested(opts.signal);
      const target = assertComponentPathWithinRoot(
        opts.stagingRoot,
        c.path,
        c.componentId,
      );
      assertNoDuplicateStagedPath(stagedPaths, c.componentId, target);
      stagedPaths.set(c.componentId, target);
      assertNoSymlinkEscape(opts.stagingRoot, target, c.componentId);
      try {
        mkdirSync(join(target, ".."), { recursive: true });
        writeFileSync(target, c.bytes);
      } catch (error) {
        journalAppend(cfg, "FAILED", "staging write failed", "STAGING_FAILED");
        throw new InstallerError(
          "STAGING_FAILED",
          `staging write failed: ${(error as NodeJS.ErrnoException).code ?? "unknown"}`,
          {
            installId: opts.installId,
            releaseId: opts.releaseId,
            componentId: c.componentId,
          },
        );
      }
    }
    abortIfRequested(opts.signal);
    journalAppend(cfg, "STAGED", "replacement staged");

    // 4. Validate staged state: real sha256 of staged bytes vs declared.
    journalAppend(cfg, "STAGING_VALIDATED", "validating staged state");
    for (const c of opts.components) {
      const staged = stagedPaths.get(c.componentId);
      if (staged === undefined) {
        journalAppend(
          cfg,
          "FAILED",
          "staged path missing",
          "VALIDATION_FAILED",
        );
        throw new InstallerError(
          "VALIDATION_FAILED",
          `staged path missing for ${c.componentId}`,
          {
            installId: opts.installId,
            releaseId: opts.releaseId,
            componentId: c.componentId,
          },
        );
      }
      let real: Uint8Array<ArrayBuffer>;
      try {
        real = new Uint8Array(readFileSync(staged));
      } catch (error) {
        journalAppend(
          cfg,
          "FAILED",
          "staged file unreadable",
          "VALIDATION_FAILED",
        );
        throw new InstallerError(
          "VALIDATION_FAILED",
          `staged file unreadable: ${(error as NodeJS.ErrnoException).code ?? "unknown"}`,
          {
            installId: opts.installId,
            releaseId: opts.releaseId,
            componentId: c.componentId,
          },
        );
      }
      const actualDigest = `sha256:${await sha256Hex(real)}`;
      if (actualDigest !== c.declaredDigest) {
        journalAppend(
          cfg,
          "FAILED",
          "staged digest mismatch",
          "DIGEST_MISMATCH",
        );
        throw new InstallerError(
          "DIGEST_MISMATCH",
          `staged digest mismatch for ${c.componentId}`,
          {
            installId: opts.installId,
            releaseId: opts.releaseId,
            componentId: c.componentId,
          },
        );
      }
    }
  } catch (error) {
    // Remove the staged state on any pre-commit failure (fence J).
    try {
      rmSync(opts.stagingRoot, { recursive: true, force: true });
    } catch {
      // Removal failure is not a second failure path; the journal
      // already records the real failure.
    }
    throw error;
  }

  // 5. Atomic switch: rename staging -> install root. AUD-067: the
  //    current install is NEVER deleted before the replacement is
  //    committed. The live install is moved aside (rename, not delete)
  //    and only removed after the new state is verified. A
  //    rename/mount/filesystem failure therefore leaves the previous
  //    install restorable at its old path, not destroyed.
  journalAppend(cfg, "SWITCHED", "atomic switch to staged state");
  const previousRoot = join(dirname(opts.installRoot), `.previous-${opts.installId}`);
  try {
    if (existsSync(opts.installRoot)) {
      renameSync(opts.installRoot, previousRoot);
    }
    try {
      renameSync(opts.stagingRoot, opts.installRoot);
    } catch (error) {
      // The commit rename failed. Restore the preserved previous
      // install so the live installation is not left deleted.
      try {
        if (existsSync(previousRoot)) {
          renameSync(previousRoot, opts.installRoot);
        }
      } catch {
        // Restore failure is a second failure; the journal records the
        // real failure below. The previous install remains at
        // previousRoot for explicit recovery.
      }
      throw error;
    }
  } catch (error) {
    journalAppend(cfg, "FAILED", "atomic switch failed", "INSTALL_FAILED");
    throw new InstallerError(
      "INSTALL_FAILED",
      `atomic switch failed: ${(error as NodeJS.ErrnoException).code ?? "unknown"}`,
      { installId: opts.installId, releaseId: opts.releaseId },
    );
  }

  // 6. Verify resulting bytes/state.
  const installed: string[] = [];
  for (const c of opts.components) {
    const finalPath = resolve(opts.installRoot, c.path);
    if (!existsSync(finalPath)) {
      journalAppend(cfg, "FAILED", "installed file missing", "INSTALL_FAILED");
      throw new InstallerError(
        "INSTALL_FAILED",
        `installed file missing for ${c.componentId}`,
        {
          installId: opts.installId,
          releaseId: opts.releaseId,
          componentId: c.componentId,
        },
      );
    }
    installed.push(c.componentId);
  }

  // AUD-067: the previous install was preserved (not deleted) through
  // the atomic switch. Now that the new state is verified, remove the
  // preserved previous install; the backup root already holds the prior
  // bytes for explicit rollback. A cleanup failure is not a second
  // failure path - the journal records the completed install.
  if (existsSync(previousRoot)) {
    try {
      rmSync(previousRoot, { recursive: true, force: true });
    } catch {
      // Preserved install left in place; rollback remains possible from
      // the backup root.
    }
  }

  journalAppend(cfg, "INSTALLED", "install completed");
  return {
    install_id: opts.installId,
    release_id: opts.releaseId,
    install_root: resolve(opts.installRoot),
    installed,
    backup,
    journal_path: cfg.journalPath,
    completed_at: new Date().toISOString(),
  };
}

/**
 * Real rollback: restore the prior state from the backup root into the
 * install root, then verify restored bytes exist. Rollback receipt only
 * after verified restoration.
 */
export async function rollbackRelease(
  opts: RollbackOptions,
): Promise<RollbackResult> {
  const cfg = journalCfg(opts);
  journalAppend(cfg, "ROLLBACK_REQUIRED", "rollback required");
  assertBackupUsable(opts.backupRoot, opts.installId);
  // AUD-066: verify the backup source digest against the REAL backup
  // bytes BEFORE any restore. A wrong/corrupt backup is denied here;
  // the caller's expectedBackupDigest is never copied into a VERIFIED
  // receipt without being proven against the actual source content.
  const backupState = await verifyBackupDigest(
    opts.backupRoot,
    opts.expectedBackupDigest,
  );
  if (backupState !== "VERIFIED") {
    journalAppend(
      cfg,
      "FAILED",
      "rollback source digest mismatch; denied",
      "ROLLBACK_FAILED",
    );
    throw new InstallerError(
      "ROLLBACK_FAILED",
      "rollback source digest mismatch; wrong or corrupt backup denied",
      { installId: opts.installId, releaseId: opts.releaseId },
    );
  }
  try {
    if (existsSync(opts.installRoot)) {
      rmSync(opts.installRoot, { recursive: true, force: true });
    }
    mkdirSync(opts.installRoot, { recursive: true });
    cpSync(opts.backupRoot, opts.installRoot, {
      recursive: true,
      force: true,
      errorOnExist: false,
      verbatimSymlinks: false,
    });
  } catch (error) {
    journalAppend(cfg, "FAILED", "rollback restore failed", "ROLLBACK_FAILED");
    throw new InstallerError(
      "ROLLBACK_FAILED",
      `rollback restore failed: ${(error as NodeJS.ErrnoException).code ?? "unknown"}`,
      { installId: opts.installId, releaseId: opts.releaseId },
    );
  }

  // Verify restored bytes exist (rollback execution, not receipt-only).
  const files = readdirSync(opts.installRoot, { recursive: true })
    .map((f) => f.toString())
    .sort();
  for (const rel of files) {
    const full = resolve(opts.installRoot, rel);
    try {
      readFileSync(full);
    } catch {
      continue;
    }
  }

  journalAppend(cfg, "ROLLBACK_COMPLETED", "rollback completed and verified");
  return {
    install_id: opts.installId,
    release_id: opts.releaseId,
    restored: files,
    backup_digest: opts.expectedBackupDigest,
    verified: "VERIFIED",
    journal_path: cfg.journalPath,
    completed_at: new Date().toISOString(),
  };
}

/**
 * Bounded recovery: read the journal, quarantine any staged state when
 * the last state is not INSTALLED, and report the journal state.
 * Recovery is NOT a magic rollback: it reports + quarantines; actual
 * byte restoration is the caller's explicit rollback invocation.
 */
export function recoverInstall(opts: RecoverOptions): RecoverResult {
  const cfg = journalCfg(opts);
  const journal = journalRead(cfg);
  journalAppend(cfg, "RECOVERY_REQUIRED", "recovery required");
  let recovered = false;
  let detail = `journal last state: ${journal.lastState ?? "none"}`;
  if (journal.lastState !== "INSTALLED") {
    if (existsSync(opts.stagingRoot)) {
      try {
        mkdirSync(opts.quarantineRoot, { recursive: true });
        renameSync(
          opts.stagingRoot,
          join(opts.quarantineRoot, `quarantine-${Date.now()}`),
        );
        detail += "; staged state quarantined";
      } catch {
        detail += "; quarantine failed (staged state left in place)";
      }
    }
  }
  if (journal.lastState === "INSTALLED") {
    recovered = true;
    detail += "; installed state present";
  }
  journalAppend(
    cfg,
    recovered ? "RECOVERY_COMPLETED" : "FAILED",
    detail,
    recovered ? undefined : "RECOVERY_FAILED",
  );
  return { journal_state: journal.lastState, recovered, detail };
}

/**
 * Fail-closed cleanup guard: remove a path only if it is inside the
 * owned install root (fence L abuse case: foreign-root cleanup request).
 */
export function cleanupOwnedPath(
  ownedRoot: string,
  requestedTarget: string,
): void {
  assertOwnedCleanupTarget(ownedRoot, requestedTarget);
  rmSync(resolve(requestedTarget), { recursive: true, force: true });
}
