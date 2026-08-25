/**
 * EP-042 M4 installer CLI (SPEC-016, SPEC-024).
 *
 * Real command surface for the installer scripts:
 *
 *   node src/cli.ts install --install-root <dir> --staging-root <dir>
 *     --backup-root <dir> --quarantine-root <dir> --journal-root <dir>
 *     --release <id> --install <id> --run <id> --git <sha>
 *     --manifest <file> --components <csv of componentId=path>
 *     --artifacts <dir>
 *   node src/cli.ts rollback --install-root <dir> --backup-root <dir>
 *     --staging-root <dir> --quarantine-root <dir> --journal-root <dir>
 *     --release <id> --install <id> --run <id> --git <sha>
 *     --backup-digest <sha256:...>
 *   node src/cli.ts recover --install-root <dir> --backup-root <dir>
 *     --staging-root <dir> --quarantine-root <dir> --journal-root <dir>
 *     --release <id> --install <id> --run <id> --git <sha>
 *   node src/cli.ts status --journal-root <dir> --release <id> --install <id>
 *     --run <id> --git <sha>
 *
 * Every command performs REAL work on the filesystem and exits nonzero
 * on any failure (fail closed). Declared digests are read from the
 * release manifest via the canonical M1/M2 surface.
 */

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { parseReleaseManifest } from "@nexus/setup";
import { InstallerError } from "./errors";
import { journalRead } from "./journal";
import {
  installRelease,
  recoverInstall,
  rollbackRelease,
  type InstallComponent,
} from "./installer";

interface Args {
  [key: string]: string | undefined;
}

function parseArgs(argv: string[]): Args {
  const out: Args = {};
  for (let i = 0; i < argv.length; i += 2) {
    const key = argv[i];
    const value = argv[i + 1];
    if (key !== undefined && key.startsWith("--") && value !== undefined) {
      out[key.slice(2)] = value;
    }
  }
  return out;
}

function requireArg(args: Args, name: string): string {
  const value = args[name];
  if (value === undefined || value.trim() === "") {
    throw new InstallerError(
      "VALIDATION_FAILED",
      `missing required argument --${name}`,
    );
  }
  return value;
}

function env(): { runId: string; gitCommit: string } {
  return {
    runId: process.env["NEXUS_INSTALL_RUN_ID"] ?? "cli",
    gitCommit: process.env["NEXUS_INSTALL_GIT_COMMIT"] ?? "unknown",
  };
}

async function cmdInstall(args: Args): Promise<void> {
  const installRoot = requireArg(args, "install-root");
  const stagingRoot = requireArg(args, "staging-root");
  const backupRoot = requireArg(args, "backup-root");
  const quarantineRoot = requireArg(args, "quarantine-root");
  const journalRoot = requireArg(args, "journal-root");
  const releaseId = requireArg(args, "release");
  const installId = requireArg(args, "install");
  const manifestFile = requireArg(args, "manifest");
  const artifactsDir = requireArg(args, "artifacts");
  const componentsCsv = requireArg(args, "components");
  const { runId, gitCommit } = env();

  const manifestWire = JSON.parse(readFileSync(manifestFile, "utf8")) as Record<
    string,
    unknown
  >;
  const manifest = parseReleaseManifest(manifestWire);
  const declared = new Map<string, string>();
  for (const component of manifest.components) {
    declared.set(component.component_id, component.digest);
  }

  const components: InstallComponent[] = [];
  for (const entry of componentsCsv.split(",")) {
    if (entry.trim() === "") continue;
    const [componentId, relPath] = entry.split("=");
    if (componentId === undefined || relPath === undefined) {
      throw new InstallerError(
        "VALIDATION_FAILED",
        `malformed component entry: ${entry}`,
      );
    }
    const bytes = new Uint8Array(
      readFileSync(resolve(artifactsDir, componentId)),
    );
    const declaredDigest = declared.get(componentId);
    if (declaredDigest === undefined) {
      throw new InstallerError(
        "MANIFEST_INVALID",
        `component ${componentId} not declared in release manifest`,
      );
    }
    components.push({ componentId, declaredDigest, bytes, path: relPath });
  }
  if (components.length === 0) {
    throw new InstallerError("VALIDATION_FAILED", "no components supplied");
  }

  const result = await installRelease({
    installRoot,
    stagingRoot,
    backupRoot,
    quarantineRoot,
    journalRoot,
    releaseId,
    installId,
    runId,
    gitCommit,
    manifestWire,
    components,
  });
  console.log(`installed: ${result.release_id} -> ${result.install_root}`);
  console.log(`installed_components: ${result.installed.join(",")}`);
  console.log(`backup_id: ${result.backup?.backup_id ?? "none"}`);
  console.log(`backup_digest: ${result.backup?.digest ?? "none"}`);
  console.log(`journal: ${result.journal_path}`);
}

async function cmdRollback(args: Args): Promise<void> {
  const installRoot = requireArg(args, "install-root");
  const backupRoot = requireArg(args, "backup-root");
  const stagingRoot = requireArg(args, "staging-root");
  const quarantineRoot = requireArg(args, "quarantine-root");
  const journalRoot = requireArg(args, "journal-root");
  const releaseId = requireArg(args, "release");
  const installId = requireArg(args, "install");
  const expectedBackupDigest = requireArg(args, "backup-digest");
  const { runId, gitCommit } = env();

  const result = await rollbackRelease({
    installRoot,
    backupRoot,
    stagingRoot,
    quarantineRoot,
    journalRoot,
    releaseId,
    installId,
    runId,
    gitCommit,
    expectedBackupDigest,
  });
  console.log(`rolled_back: ${result.release_id}`);
  console.log(`restored: ${result.restored.length} entries`);
  console.log(`rollback_verified: ${result.verified}`);
}

function cmdRecover(args: Args): void {
  const installRoot = requireArg(args, "install-root");
  const backupRoot = requireArg(args, "backup-root");
  const stagingRoot = requireArg(args, "staging-root");
  const quarantineRoot = requireArg(args, "quarantine-root");
  const journalRoot = requireArg(args, "journal-root");
  const releaseId = requireArg(args, "release");
  const installId = requireArg(args, "install");
  const { runId, gitCommit } = env();

  const result = recoverInstall({
    installRoot,
    backupRoot,
    stagingRoot,
    quarantineRoot,
    journalRoot,
    releaseId,
    installId,
    runId,
    gitCommit,
  });
  console.log(`recover_journal_state: ${result.journal_state ?? "none"}`);
  console.log(`recovered: ${result.recovered ? "true" : "false"}`);
  console.log(`detail: ${result.detail}`);
}

function cmdStatus(args: Args): void {
  const journalRoot = requireArg(args, "journal-root");
  const releaseId = requireArg(args, "release");
  const installId = requireArg(args, "install");
  const { runId, gitCommit } = env();
  const journal = journalRead({
    journalPath: resolve(journalRoot, "installer.journal.jsonl"),
    runId,
    gitCommit,
    installId,
    releaseId,
  });
  console.log(`journal_entries: ${journal.entries.length}`);
  console.log(`last_state: ${journal.lastState ?? "none"}`);
  for (const entry of journal.entries.slice(-3)) {
    console.log(`  ${entry.state} ${entry.detail}`);
  }
}

async function main(): Promise<void> {
  const [command, ...rest] = process.argv.slice(2);
  const args = parseArgs(rest);
  switch (command) {
    case "install":
      await cmdInstall(args);
      break;
    case "rollback":
      await cmdRollback(args);
      break;
    case "recover":
      cmdRecover(args);
      break;
    case "status":
      cmdStatus(args);
      break;
    default:
      throw new InstallerError(
        "VALIDATION_FAILED",
        `unknown command: ${command ?? "none"}`,
      );
  }
}

main().catch((error) => {
  if (error instanceof InstallerError) {
    const shape = error.toShape();
    console.error(`installer ${shape.failure_class}: ${shape.message}`);
    process.exitCode = 1;
  } else {
    console.error(`installer internal: ${(error as Error).message}`);
    process.exitCode = 1;
  }
});
