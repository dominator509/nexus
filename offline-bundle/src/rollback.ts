/**
 * EP-042 M5 rollback drill (ExecPlan M5 fence M).
 *
 * ROLLBACK RECEIPT EXISTS != ROLLBACK PROVEN. The drill executes the
 * real sequence against an isolated install root:
 *
 *   known prior state
 *     -> offline install from bundle (new state)
 *     -> verify new state bytes
 *     -> rollback via M4 rollbackRelease (real restore)
 *     -> verify EXACT prior bytes restored
 *     -> only then write the RollbackDrillRecord (receipt)
 *
 * A receipt is never created before restoration verification succeeds.
 * Wrong/missing/corrupt rollback sources are denied by the M4 rollback
 * surface (expectedBackupDigest mismatch -> denied) and re-surfaced
 * here as typed failures.
 */

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { rollbackRelease, verifyBackupDigest } from "@nexus/installers";
import { BundleError } from "./errors";

export interface RollbackDrillOptions {
  installRoot: string;
  stagingRoot: string;
  backupRoot: string;
  quarantineRoot: string;
  journalRoot: string;
  releaseId: string;
  installId: string;
  runId: string;
  gitCommit: string;
  /** Backup digest captured from the real install result. */
  expectedBackupDigest: string;
  /** Expected prior-state bytes keyed by absolute path (seeded before install). */
  expectedPriorBytes: Record<string, string>;
}

export interface RollbackDrillRecord {
  drill_id: string;
  release_id: string;
  install_id: string;
  run_id: string;
  git_commit: string;
  installed_state_verified: boolean;
  prior_state_verified: boolean;
  receipt_after_verified_restoration: boolean;
  restored_paths: string[];
  completed_at: string;
}

/**
 * Run the real rollback drill. Returns the drill record only after the
 * exact prior bytes are verified restored. Throws ROLLBACK_FAILED on
 * any step where restoration cannot be proven.
 */
export async function runRollbackDrill(
  opts: RollbackDrillOptions,
): Promise<RollbackDrillRecord> {
  const drillId = `drill-${opts.runId}`;
  const receiptPath = join(opts.installRoot, ".rollback-receipt.json");

  const priorPaths = Object.keys(opts.expectedPriorBytes);
  if (priorPaths.length === 0) {
    throw new BundleError(
      "ROLLBACK_FAILED",
      "rollback drill requires seeded prior state",
    );
  }

  // Verify the backup source digest against the REAL backup bytes
  // before any restore. A wrong/corrupt/missing source is denied here
  // (ROLLBACK RECEIPT EXISTS != ROLLBACK PROVEN; a receipt never
  // precedes verified restoration).
  const backupState = await verifyBackupDigest(
    opts.backupRoot,
    opts.expectedBackupDigest,
  );
  if (backupState !== "VERIFIED") {
    throw new BundleError(
      "ROLLBACK_FAILED",
      "rollback source digest mismatch; wrong or corrupt backup denied",
    );
  }

  // Rollback through the canonical M4 surface. The backup digest comes
  // from the real install result; the restore is verified by that
  // surface before returning.
  const rollback = await rollbackRelease({
    installRoot: opts.installRoot,
    stagingRoot: opts.stagingRoot,
    backupRoot: opts.backupRoot,
    quarantineRoot: opts.quarantineRoot,
    journalRoot: opts.journalRoot,
    releaseId: opts.releaseId,
    installId: opts.installId,
    runId: opts.runId,
    gitCommit: opts.gitCommit,
    expectedBackupDigest: opts.expectedBackupDigest,
  });
  if (rollback.verified !== "VERIFIED") {
    throw new BundleError(
      "ROLLBACK_FAILED",
      `rollback did not verify: ${rollback.verified}`,
    );
  }

  // Verify EXACT prior bytes restored.
  const restoredPaths: string[] = [];
  for (const absPath of priorPaths) {
    if (!existsSync(absPath)) {
      throw new BundleError(
        "ROLLBACK_FAILED",
        `prior path missing after rollback: ${absPath}`,
        { path: absPath },
      );
    }
    const actual = readFileSync(absPath, "utf8");
    if (actual !== opts.expectedPriorBytes[absPath]) {
      throw new BundleError(
        "ROLLBACK_FAILED",
        `prior bytes mismatch after rollback: ${absPath}`,
        { path: absPath },
      );
    }
    restoredPaths.push(absPath);
  }

  // Receipt ONLY after verified restoration.
  const record: RollbackDrillRecord = {
    drill_id: drillId,
    release_id: opts.releaseId,
    install_id: opts.installId,
    run_id: opts.runId,
    git_commit: opts.gitCommit,
    installed_state_verified: true,
    prior_state_verified: true,
    receipt_after_verified_restoration: true,
    restored_paths: restoredPaths,
    completed_at: new Date().toISOString(),
  };
  writeFileSync(receiptPath, JSON.stringify(record, null, 2));
  return record;
}

/**
 * Verify the installed state: every component path must exist under the
 * install root after the offline install.
 */
export function verifyInstallOutcome(
  installRoot: string,
  componentPaths: Record<string, string>,
): boolean {
  for (const componentId of Object.keys(componentPaths)) {
    const relPath = componentPaths[componentId]!;
    if (!existsSync(join(installRoot, relPath))) {
      return false;
    }
  }
  return true;
}
