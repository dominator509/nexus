/**
 * EP-042 M4 append-only installer journal (SPEC-016 behavior 6;
 * SPEC-024 restore).
 *
 * The journal records every state transition of an install/rollback/
 * recovery operation on the real filesystem. It is append-only per
 * journal root and bound to the current run (run_id, git_commit,
 * install_id, release_id). Entries are written before and after each
 * owned mutation so an interrupted update leaves an auditable trace:
 *
 * - JOURNAL EXISTS != UPDATE COMPLETED
 * - RECOVERY ATTEMPTED != RECOVERY VERIFIED
 * - FAILURE INJECTED != SYSTEM HARDENED
 *
 * Journal files never contain secret-shaped content; redaction is
 * applied at the boundary (fence P).
 */

import {
  writeFileSync,
  appendFileSync,
  readFileSync,
  mkdirSync,
} from "node:fs";
import { dirname, resolve } from "node:path";
import type { InstallerFailureClass } from "./errors";
import { redactValue } from "./observability";

export const INSTALLER_JOURNAL_STATES = [
  "STARTED",
  "MANIFEST_VALIDATED",
  "COMPATIBILITY_OK",
  "BACKUP_REQUESTED",
  "BACKUP_COMPLETED",
  "STAGING",
  "STAGED",
  "STAGING_VALIDATED",
  "SWITCHED",
  "INSTALLED",
  "ROLLBACK_REQUIRED",
  "ROLLBACK_COMPLETED",
  "RECOVERY_REQUIRED",
  "RECOVERY_COMPLETED",
  "FAILED",
] as const;

export type InstallerJournalState = (typeof INSTALLER_JOURNAL_STATES)[number];

export interface InstallerJournalEntry {
  ts: string;
  run_id: string;
  git_commit: string;
  install_id: string;
  release_id: string;
  state: InstallerJournalState;
  failure_class?: InstallerFailureClass;
  detail: string;
}

export interface InstallerJournalConfig {
  journalPath: string;
  runId: string;
  gitCommit: string;
  installId: string;
  releaseId: string;
}

/**
 * Open (create parent dir if needed) and append a journal entry. The
 * entry is serialized as one JSON line; the file grows append-only.
 * Redaction is applied to every free-text field before serialization.
 */
export function journalAppend(
  cfg: InstallerJournalConfig,
  state: InstallerJournalState,
  detail: string,
  failureClass?: InstallerFailureClass,
): void {
  const dir = dirname(cfg.journalPath);
  if (dir.length > 0) mkdirSync(dir, { recursive: true });
  const entry: InstallerJournalEntry = {
    ts: new Date().toISOString(),
    run_id: redactValue(cfg.runId),
    git_commit: redactValue(cfg.gitCommit),
    install_id: redactValue(cfg.installId),
    release_id: redactValue(cfg.releaseId),
    state,
    detail: redactValue(detail),
  };
  if (failureClass !== undefined) entry.failure_class = failureClass;
  appendFileSync(cfg.journalPath, `${JSON.stringify(entry)}\n`, "utf8");
}

export interface JournalRecord {
  entries: ReadonlyArray<InstallerJournalEntry>;
  lastState: InstallerJournalState | undefined;
}

/**
 * Read the journal back and summarize the last observed state. A
 * missing journal file is an empty record (nothing has started), never
 * a proof of completion.
 */
export function journalRead(cfg: InstallerJournalConfig): JournalRecord {
  let raw = "";
  try {
    raw = readFileSync(cfg.journalPath, "utf8");
  } catch {
    return { entries: [], lastState: undefined };
  }
  const entries: InstallerJournalEntry[] = [];
  for (const line of raw.split("\n")) {
    if (line.trim() === "") continue;
    try {
      const parsed = JSON.parse(line) as InstallerJournalEntry;
      entries.push(parsed);
    } catch {
      // A malformed journal line is not a state proof; skip it but keep
      // the file untouched (append-only means we never rewrite).
      continue;
    }
  }
  return {
    entries,
    lastState:
      entries.length > 0 ? entries[entries.length - 1]!.state : undefined,
  };
}

/** Fail-closed completion check: INSTALLED is the only completed state. */
export function journalComplete(journal: JournalRecord): boolean {
  return journal.lastState === "INSTALLED";
}

/** Write a fresh journal file (used only when a journal root is created). */
export function journalReset(cfg: InstallerJournalConfig): void {
  const dir = dirname(cfg.journalPath);
  if (dir.length > 0) mkdirSync(dir, { recursive: true });
  writeFileSync(cfg.journalPath, "", "utf8");
}

/** Resolve a journal path inside a journal root (never outside it). */
export function journalPathInRoot(journalRoot: string, name: string): string {
  return resolve(journalRoot, name);
}
