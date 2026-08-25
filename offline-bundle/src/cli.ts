/**
 * EP-042 M5 offline bundle CLI (SPEC-016 behavior 5, SPEC-024).
 *
 * Real commands executed by the POSIX scripts:
 *   produce        build a real bundle from real files
 *   verify         digest-bound verification (fails closed)
 *   install        offline install from a verified bundle (no transport)
 *   rollback-drill real rollback drill (receipt after verified restore)
 *   evidence       write + validate current-run redacted evidence
 *
 * Runs under Node 24 native TS with the resolution-only ESM loader
 * (scripts/ts-resolve-loader.mjs) so it executes the REAL canonical
 * @nexus/setup + @nexus/installers code.
 */

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { produceBundle } from "./produce";
import { verifyBundle } from "./verify";
import { installBundleOffline, type OfflineInstallOptions } from "./install";
import { runRollbackDrill, verifyInstallOutcome } from "./rollback";
import {
  buildBundleEvidence,
  validateEvidence,
  writeEvidenceFile,
} from "./evidence";
import { BundleError } from "./errors";

const [command, ...args] = process.argv.slice(2);

function flag(name: string): string | undefined {
  const index = args.indexOf(`--${name}`);
  if (index === -1 || index + 1 >= args.length) return undefined;
  return args[index + 1];
}
function flagDefault(name: string, fallback: string): string {
  return flag(name) ?? fallback;
}

function requireFlag(name: string): string {
  const value = flag(name);
  if (value === undefined) {
    throw new BundleError("BUNDLE_INVALID", `missing required --${name}`);
  }
  return value;
}

async function cmdProduce(): Promise<void> {
  const bundleDir = requireFlag("bundle-dir");
  const bundleId = requireFlag("bundle-id");
  const releaseId = requireFlag("release-id");
  const manifestPath = requireFlag("manifest");
  const artifactsCsv = requireFlag("artifacts"); // compId=path:kind,compId=path:kind
  const sbomsCsv = flagDefault("sboms", "");
  const licensesCsv = flagDefault("licenses", "");
  const migrationsCsv = flagDefault("migrations", "");
  const recoveryCsv = flagDefault("recovery", "");

  const releaseManifestWire = readFileSync(manifestPath, "utf8");
  const artifacts: Record<
    string,
    { kind: "IMAGE" | "MODEL"; payloadPath: string; name: string }
  > = {};
  for (const entry of artifactsCsv.split(",")) {
    if (entry.length === 0) continue;
    const [componentId, rest] = entry.split("=");
    const [path, kindRaw] = (rest ?? "").split(":");
    const kind = kindRaw === "MODEL" ? "MODEL" : "IMAGE";
    if (!componentId || !path) {
      throw new BundleError("BUNDLE_INVALID", `bad artifact entry ${entry}`);
    }
    artifacts[componentId] = {
      kind,
      payloadPath: path,
      name: path.split("/").pop() ?? path,
    };
  }
  const mapPayloads = (csv: string) =>
    csv
      .split(",")
      .filter((entry) => entry.length > 0)
      .map((entry) => {
        const [name, path] = entry.split("=");
        if (!name || !path) {
          throw new BundleError("BUNDLE_INVALID", `bad payload entry ${entry}`);
        }
        return { name, payloadPath: path };
      });

  const result = await produceBundle({
    bundleDir,
    bundleId,
    releaseId,
    releaseManifestWire,
    artifacts,
    sbomPayloads: mapPayloads(sbomsCsv),
    licensePayloads: mapPayloads(licensesCsv),
    migrationPayloads: mapPayloads(migrationsCsv),
    recoveryToolPayloads: mapPayloads(recoveryCsv),
  });
  console.log(`bundle produced: ${result.bundleId} (${result.bundleDigest})`);
  console.log(`bundle manifest_digest: ${result.manifestDigest}`);
  console.log(`bundle items: ${result.items.length}`);
  console.log(`bundle files: ${result.filesWritten.length}`);
}

async function cmdVerify(): Promise<void> {
  const bundleDir = requireFlag("bundle-dir");
  const result = await verifyBundle({ bundleDir });
  console.log(`bundle verified: ${result.bundleId} (${result.bundleDigest})`);
  console.log(`bundle verification: ${result.state}`);
  console.log(`bundle files_verified: ${result.filesVerified}`);
}

async function cmdInstall(): Promise<void> {
  const opts: OfflineInstallOptions = {
    bundleDir: requireFlag("bundle-dir"),
    installRoot: requireFlag("install-root"),
    stagingRoot: requireFlag("staging-root"),
    backupRoot: requireFlag("backup-root"),
    quarantineRoot: requireFlag("quarantine-root"),
    journalRoot: requireFlag("journal-root"),
    releaseId: requireFlag("release"),
    installId: requireFlag("install"),
    runId: requireFlag("run-id"),
    gitCommit: requireFlag("git-commit"),
    componentPaths: Object.fromEntries(
      requireFlag("components")
        .split(",")
        .filter((entry) => entry.length > 0)
        .map((entry) => {
          const [componentId, path] = entry.split("=");
          if (!componentId || !path) {
            throw new BundleError(
              "BUNDLE_INVALID",
              `bad component entry ${entry}`,
            );
          }
          return [componentId, path];
        }),
    ),
  };
  const result = await installBundleOffline(opts);
  console.log(`offline installed: ${result.install.release_id}`);
  console.log(`offline install_id: ${result.install.install_id}`);
  console.log(
    `offline transport_required: ${result.offline.transport_required}`,
  );
  console.log(`offline source: ${result.offline.source}`);
  console.log(
    `offline components: ${result.offline.componentsResolved.join(",")}`,
  );
  if (result.install.backup !== undefined) {
    console.log(`backup_digest: ${result.install.backup.digest}`);
  }
  console.log(`installed: ${result.install.release_id}`);
}

async function cmdRollbackDrill(): Promise<void> {
  const installRoot = requireFlag("install-root");
  const releaseId = requireFlag("release");
  const installId = requireFlag("install");
  const backupDigest = requireFlag("backup-digest");
  const expectedPrior = requireFlag("expected-prior");
  const runId = requireFlag("run-id");
  const gitCommit = requireFlag("git-commit");

  const expectedPriorBytes: Record<string, string> = {};
  for (const entry of expectedPrior.split(",")) {
    if (entry.length === 0) continue;
    const [absPath, bytes] = entry.split("=");
    if (!absPath)
      throw new BundleError("BUNDLE_INVALID", `bad prior entry ${entry}`);
    expectedPriorBytes[absPath] = bytes ?? "";
  }

  const record = await runRollbackDrill({
    installRoot,
    stagingRoot: flagDefault("staging-root", `${installRoot}.staging`),
    backupRoot: flagDefault("backup-root", `${installRoot}.backup`),
    quarantineRoot: flagDefault("quarantine-root", `${installRoot}.quarantine`),
    journalRoot: flagDefault("journal-root", `${installRoot}.journal`),
    releaseId,
    installId,
    runId,
    gitCommit,
    expectedBackupDigest: backupDigest,
    expectedPriorBytes,
  });
  console.log(`rollback drill: ${record.drill_id}`);
  console.log(`rollback_verified: ${record.prior_state_verified}`);
  console.log(
    `rollback_receipt_after_verified_restoration: ${record.receipt_after_verified_restoration}`,
  );
  console.log(`rollback restored_paths: ${record.restored_paths.join(",")}`);
}

async function cmdEvidence(): Promise<void> {
  const outPath = requireFlag("out");
  const runId = requireFlag("run-id");
  const gitCommit = requireFlag("git-commit");
  const releaseId = requireFlag("release-id");
  const installId = requireFlag("install-id");
  const bundleId = requireFlag("bundle-id");
  const manifestDigest = requireFlag("manifest-digest");
  const bundleDigest = requireFlag("bundle-digest");
  const componentDigestsCsv = flagDefault("component-digests", "");
  const verificationState = flagDefault("verification-state", "VERIFIED");
  const installState = flagDefault("install-state", "INSTALLED");
  const rollbackState = flagDefault("rollback-state", "VERIFIED");
  const offlineInstallState = flagDefault(
    "offline-install-state",
    "OFFLINE_INSTALL_VERIFIED",
  );
  const signatureState = flagDefault(
    "signature-state",
    "SIGNATURE_PRESENT_NOT_VERIFIED",
  );
  const boundaryPath = flagDefault("boundary", "");
  const canariesCsv = flagDefault("canaries", "");

  const boundary =
    boundaryPath.length > 0
      ? readFileSync(boundaryPath, "utf8")
          .split("\n")
          .filter((line) => line.length > 0)
      : ["INTERNAL BEHAVIOR CERTIFIED for exact exercised local surface"];

  const evidence = await buildBundleEvidence({
    runId,
    gitCommit,
    releaseId,
    installId,
    bundleId,
    manifestDigest,
    bundleDigest,
    componentDigests: componentDigestsCsv
      .split(",")
      .filter((entry) => entry.length > 0),
    bundleVerificationState: verificationState,
    installState,
    rollbackState,
    offlineInstallState,
    signatureState,
    certificationBoundary: boundary,
    timestamp: new Date().toISOString(),
    secretCanaries: canariesCsv.split(",").filter((entry) => entry.length > 0),
  });
  await validateEvidence(evidence, { runId, gitCommit });
  const written = writeEvidenceFile(evidence, outPath);
  console.log(`evidence written: ${written}`);
  console.log(`evidence digest: ${evidence.evidence_digest}`);
  console.log(`redaction_result: ${evidence.redaction_result}`);
}

async function main(): Promise<void> {
  switch (command) {
    case "produce":
      await cmdProduce();
      break;
    case "verify":
      await cmdVerify();
      break;
    case "install":
      await cmdInstall();
      break;
    case "rollback-drill":
      await cmdRollbackDrill();
      break;
    case "evidence":
      await cmdEvidence();
      break;
    default:
      console.error("offline-bundle CLI: unknown command");
      console.error(
        "usage: cli.ts produce|verify|install|rollback-drill|evidence ...",
      );
      process.exit(2);
  }
}

main().catch((error) => {
  if (error instanceof BundleError) {
    console.error(`offline bundle ${error.code}: ${error.message}`);
  } else {
    console.error(`offline bundle FAILED: ${(error as Error).message}`);
  }
  process.exit(1);
});
