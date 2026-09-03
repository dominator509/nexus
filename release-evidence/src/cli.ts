/**
 * EP-043 M2/M3 production readiness CLI (SPEC-008).
 *
 * Real commands:
 *   readiness        collect real repository state, evaluate acceptance
 *                    obligations, write PRODUCTION_READINESS.md
 *   manifest         build dist/release/RELEASE_MANIFEST.json from real
 *                    component bytes with real sha256 digests
 *   ship-gate-status inspect the ship gate: obligations, verdict, and
 *                    exact blocking reasons from real repository state
 *   certification-rows
 *                    list certification rows read from the real
 *                    provider and hardware RESULTS.md files
 *   verify-manifest  verify dist/release/RELEASE_MANIFEST.json against
 *                    the real artifact bytes and the manifest digest;
 *                    fails closed on tamper or missing dependency
 *
 * Runs under Node 24 native TS with the resolution-only ESM loader.
 */

import {
  mkdirSync,
  readFileSync,
  renameSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import {
  collectCertifications,
  collectReadinessInputs,
  defaultRepoPaths,
} from "./repo-state.ts";
import {
  evaluateReadiness,
  validateReadinessInputs,
  liveFireProofsToGateProofs,
} from "./readiness.ts";
import { renderProductionReadinessReport } from "./report.ts";
import {
  buildReleaseManifest,
  digestBytes,
  parseReleaseManifestWire,
  verifyManifestDigest,
  type ManifestComponentInput,
  type ReleaseManifestWire,
} from "./manifest.ts";
import {
  createManualDeployHandoff,
  createProductionReadinessDecision,
  createReleaseEvidence,
  createShipGate,
} from "./model.ts";
import { redactShipMessage, ShipError } from "./errors.ts";

const [command, ...args] = process.argv.slice(2);

function flag(name: string): string | undefined {
  // Support both --name value and --name=value forms; --name=value
  // silently falling back to a default would misdirect evidence.
  const eq = `--${name}=`;
  for (const arg of args) {
    if (arg.startsWith(eq)) return arg.slice(eq.length);
  }
  const index = args.indexOf(`--${name}`);
  if (index === -1 || index + 1 >= args.length) return undefined;
  return args[index + 1];
}

function requireFlag(name: string): string {
  const value = flag(name);
  if (!value) {
    throw new ShipError("VALIDATION_FAILED", `missing required flag --${name}`);
  }
  return value;
}

/**
 * Reject unknown flags for a command (M4 operator-bypass protection).
 * A command must declare every flag it accepts; anything else fails
 * closed so a forged --force/--override can never be silently ignored.
 */
function rejectUnknownFlags(allowed: readonly string[]): void {
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index]!;
    if (arg.startsWith("--")) {
      const name = arg.slice(2).split("=")[0]!;
      if (!allowed.includes(name)) {
        throw new ShipError("VALIDATION_FAILED", `unknown flag --${name}`);
      }
      // Consume the separate value form (--name value) so the value is
      // not misread as a positional argument.
      if (
        !arg.includes("=") &&
        index + 1 < args.length &&
        !args[index + 1]!.startsWith("--")
      ) {
        index += 1;
      }
    } else {
      throw new ShipError("VALIDATION_FAILED", `unexpected argument: ${arg}`);
    }
  }
}

/** Reject an output path that exists but is a directory (EISDIR class). */
function rejectDirectoryTarget(target: string): void {
  try {
    if (statSync(target).isDirectory()) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `output path is a directory: ${target}`,
      );
    }
  } catch (error) {
    if (error instanceof ShipError) throw error;
    // statSync errors (ENOENT etc.) mean the target does not exist yet,
    // which is the normal case for an output path.
  }
}

/** Reject an output directory that exists but is not a directory. */
function rejectFileTarget(target: string): void {
  try {
    if (!statSync(target).isDirectory()) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `output path is not a directory: ${target}`,
      );
    }
  } catch (error) {
    if (error instanceof ShipError) throw error;
    // ENOENT is the normal case: the directory will be created.
  }
}

/**
 * Write a file atomically (temp file + rename) so cancelled or failed
 * work never leaves a partial target file (M4 partial-side-effect
 * guarantee). The temp path is owned by this process and removed by
 * the rename; a kill mid-write strands a .tmp-<pid> file, so the path
 * is tracked and unlinked by the SIGTERM/SIGINT cleanup handler below.
 */
let pendingTmp: string | null = null;

function writeAtomic(target: string, content: string): void {
  const tmp = `${target}.tmp-${process.pid}`;
  pendingTmp = tmp;
  try {
    writeFileSync(tmp, content, "utf8");
    renameSync(tmp, target);
  } finally {
    pendingTmp = null;
  }
}

/**
 * Install SIGTERM/SIGINT cleanup so cancelled work never strands the
 * atomic-write temp file (M4 no-partial-output guarantee, EP-043
 * ep043_failure_cancelled_work_no_partial_output). With a handler
 * installed, the process is not killed mid-syscall by the default
 * signal behavior: a signal that arrives during the synchronous
 * write+rename pair is delivered after the pair completes (target
 * complete, no residue), and one that arrives before or between
 * operations unlinks any tracked temp path. Either way no .tmp-*
 * residue remains. The conventional exit codes are preserved (143 for
 * SIGTERM, 130 for SIGINT).
 */
function installSignalCleanup(): void {
  const cleanup = (code: number): void => {
    if (pendingTmp !== null) {
      try {
        unlinkSync(pendingTmp);
      } catch {
        // Best effort: the temp file may already be renamed or absent.
      }
    }
    process.exit(code);
  };
  process.on("SIGTERM", () => cleanup(143));
  process.on("SIGINT", () => cleanup(130));
}

function runId(prefix: string): string {
  return `${prefix}-${Date.now()}`;
}

function gitCommit(): string {
  try {
    const head = readFileSync(
      join(process.cwd(), ".git", "HEAD"),
      "utf8",
    ).trim();
    if (head.startsWith("ref:")) {
      const refPath = head.slice(5).trim();
      return readFileSync(join(process.cwd(), ".git", refPath), "utf8")
        .trim()
        .slice(0, 40);
    }
    return head.slice(0, 40);
  } catch {
    return "unknown";
  }
}

async function commandReadiness(): Promise<void> {
  rejectUnknownFlags(["output"]);
  const output = flag("output") ?? "PRODUCTION_READINESS.md";
  const root = process.cwd();
  const paths = defaultRepoPaths(root);
  const inputs = collectReadinessInputs(paths);
  validateReadinessInputs(inputs);
  const evaluation = evaluateReadiness(inputs);
  // AUD-087: the production readiness decision must be bound through the
  // authoritative M1 decision constructor (gate verdict, fresh-clone
  // rerun, every drill DATED_EVIDENCE, exact manual command), not only
  // derived from the six acceptance obligations.
  const runIdValue = runId("ep043-readiness");
  const gitCommitValue = gitCommit();
  const gate = createShipGate({
    gateId: "ep043-ship-gate",
    releaseKind: "CORE_RELEASE",
    phase: "SHIP_DECISION",
    requiredProofs: liveFireProofsToGateProofs(inputs.liveFireProofs),
    freshCloneRerun: inputs.freshCloneRerun,
  });
  const evidence = createReleaseEvidence({
    node: "EP-043",
    runId: runIdValue,
    gitCommit: gitCommitValue,
    releaseId: "nexus-1.0.0-rc1",
    certifications: [
      ...inputs.certifications.providerRows,
      ...inputs.certifications.hardwareRows,
    ],
    drills: inputs.drills,
    reviews: inputs.reviews,
  });
  const handoff = createManualDeployHandoff({
    handoffId: "ep043-deploy-handoff",
    releaseId: "nexus-1.0.0-rc1",
    profile: "core",
    exactCommand: inputs.manualDeployCommand,
  });
  const decision = createProductionReadinessDecision({
    decisionId: "ep043-production-readiness",
    releaseId: "nexus-1.0.0-rc1",
    gate,
    evidence,
    handoff,
  });
  const report = renderProductionReadinessReport(evaluation, {
    node: "EP-043",
    runId: runIdValue,
    gitCommit: gitCommitValue,
    generatedAt: new Date().toISOString(),
    decision: decision.decision,
  });
  const safe = redactShipMessage(report);
  const target = resolve(root, output);
  rejectDirectoryTarget(target);
  writeAtomic(target, safe);
  // eslint-disable-next-line no-console
  console.log(
    `readiness: ${decision.decision} (${evaluation.blockingReasons.length} blocking reasons)`,
  );
  // eslint-disable-next-line no-console
  console.log(`wrote ${output} (${safe.length} bytes)`);
}

/**
 * AUD-082: real release artifacts. The manifest MUST be built from the
 * REAL product artifacts that ship - the actual model code, provider
 * config, router policy, and container definition committed in the
 * repository - never from fixture strings. Each entry resolves to a
 * committed, clone-portable path so a fresh checkout can rebuild and
 * reverify the exact same manifest.
 */
const REAL_RELEASE_ARTIFACTS: ReadonlyArray<{
  componentId: string;
  name: string;
  version: string;
  /** Repository-relative artifact path (committed, clone-portable). */
  relPath: string;
}> = [
  {
    componentId: "nexus-wake-model",
    name: "nexus-wake-model",
    version: "1.0.0",
    relPath: "models/wake/nexus_wake/decision.py",
  },
  {
    componentId: "nexus-wake-manifest",
    name: "nexus-wake-manifest",
    version: "1.0.0",
    relPath: "models/wake/nexus_wake/manifest.py",
  },
  {
    componentId: "nexus-providers-config",
    name: "nexus-providers-config",
    version: "1.0.0",
    relPath: "config/models/providers/providers.json",
  },
  {
    componentId: "nexus-router-policy",
    name: "nexus-router-policy",
    version: "1.0.0",
    relPath: "config/models/router/policy.json",
  },
  {
    componentId: "nexus-container-seaweedfs",
    name: "nexus-container-seaweedfs",
    version: "1.0.0",
    relPath: "infra/release/containers/seaweedfs.yaml",
  },
];

function readRealArtifact(
  root: string,
  relPath: string,
  componentId: string,
): Uint8Array {
  const fullPath = join(root, relPath);
  try {
    const bytes = readFileSync(fullPath);
    return new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  } catch {
    throw new ShipError(
      "NOT_FOUND",
      `real release artifact missing for ${componentId}: ${relPath}`,
    );
  }
}

function realArtifactInputs(
  root: string,
  releaseId: string,
): ManifestComponentInput[] {
  return REAL_RELEASE_ARTIFACTS.map((artifact) => ({
    componentId: artifact.componentId,
    name: artifact.name,
    version: artifact.version,
    artifactBytes: readRealArtifact(
      root,
      artifact.relPath,
      artifact.componentId,
    ),
    artifactKey: `releases/${releaseId}/components/${artifact.componentId}`,
  }));
}

async function commandManifest(): Promise<void> {
  rejectUnknownFlags(["output-dir", "release-id"]);
  const outputDir = flag("output-dir") ?? "dist/release";
  const releaseId = flag("release-id") ?? "nexus-1.0.0-rc1";
  const root = process.cwd();
  const outPath = resolve(root, outputDir);
  rejectFileTarget(outPath);
  mkdirSync(outPath, { recursive: true });

  // AUD-082: bind the manifest to the REAL committed product artifacts,
  // never fixture strings.
  const componentInputs = realArtifactInputs(root, releaseId);

  const manifest = buildReleaseManifest({
    releaseId,
    version: "1.0.0",
    channel: "STABLE",
    profile: "FULLY_LOCAL",
    createdAt: new Date().toISOString(),
    components: componentInputs,
  });

  writeAtomic(
    join(outPath, "RELEASE_MANIFEST.json"),
    JSON.stringify(manifest, null, 2),
  );
  // eslint-disable-next-line no-console
  console.log(
    `manifest: wrote ${join(outputDir, "RELEASE_MANIFEST.json")} digest=${manifest.manifest_digest}`,
  );
  // eslint-disable-next-line no-console
  console.log(
    `manifest: ${manifest.components.length} real product artifacts, signatures PRESENT_NOT_VERIFIED`,
  );
}

/** Inspect the ship gate: obligations, verdict, exact blocking reasons. */
async function commandShipGateStatus(): Promise<void> {
  rejectUnknownFlags([]);
  const root = process.cwd();
  const paths = defaultRepoPaths(root);
  const inputs = collectReadinessInputs(paths);
  validateReadinessInputs(inputs);
  const evaluation = evaluateReadiness(inputs);
  // eslint-disable-next-line no-console
  console.log(`ship-gate verdict: ${evaluation.shipGateVerdict}`);
  // eslint-disable-next-line no-console
  console.log(`readiness decision: ${evaluation.decision}`);
  for (const obligation of evaluation.obligations) {
    // eslint-disable-next-line no-console
    console.log(
      `obligation: ${obligation.obligation}: ${obligation.met ? "MET" : "NOT MET"}`,
    );
  }
  // eslint-disable-next-line no-console
  console.log(`blocking reasons (${evaluation.blockingReasons.length}):`);
  for (const reason of evaluation.blockingReasons) {
    // eslint-disable-next-line no-console
    console.log(`  - ${redactShipMessage(reason)}`);
  }
}

/** List certification rows read from the real RESULTS.md files. */
async function commandCertificationRows(): Promise<void> {
  rejectUnknownFlags([]);
  const root = process.cwd();
  const paths = defaultRepoPaths(root);
  const certifications = collectCertifications(paths);
  const rows = [...certifications.providerRows, ...certifications.hardwareRows];
  if (rows.length === 0) {
    // eslint-disable-next-line no-console
    console.log("certification rows: none");
    return;
  }
  for (const row of rows) {
    // eslint-disable-next-line no-console
    console.log(
      `${row.domain} ${row.rowId} ${row.state}${
        row.evidenceRef ? ` ${row.evidenceRef}` : ""
      }`,
    );
  }
  // eslint-disable-next-line no-console
  console.log(`certification rows: ${rows.length}`);
}

/**
 * Verify the release manifest against real artifact bytes and the
 * manifest digest. Fails closed on tamper or missing dependency.
 */
async function commandVerifyManifest(): Promise<void> {
  rejectUnknownFlags(["manifest"]);
  const manifestPath = flag("manifest") ?? "dist/release/RELEASE_MANIFEST.json";
  const root = process.cwd();
  const fullPath = resolve(root, manifestPath);
  let raw: string;
  try {
    const st = statSync(fullPath);
    if (!st.isFile()) {
      // A FIFO (or socket/device) blocks reads forever; the manifest
      // must be a regular file. Fail closed instead of hanging the
      // bounded budget (ep043_failure_timeout_blocked_dependency).
      throw new ShipError(
        "VALIDATION_FAILED",
        `release manifest is not a regular file: ${manifestPath}`,
      );
    }
    raw = readFileSync(fullPath, "utf8");
  } catch (error) {
    if (error instanceof ShipError) throw error;
    throw new ShipError(
      "NOT_FOUND",
      `release manifest not found or unreadable: ${manifestPath}`,
    );
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw) as unknown;
  } catch {
    throw new ShipError(
      "VALIDATION_FAILED",
      `release manifest is not valid JSON: ${manifestPath}`,
    );
  }
  const manifest: ReleaseManifestWire = parseReleaseManifestWire(parsed);
  if (!verifyManifestDigest(manifest)) {
    throw new ShipError(
      "VERIFICATION_FAILED",
      "manifest digest mismatch (tampered manifest)",
    );
  }
  // AUD-082: verify against the REAL committed product artifacts. A
  // component that is not one of the real release artifacts is not a
  // shippable product component and fails closed.
  const realByComponentId = new Map(
    REAL_RELEASE_ARTIFACTS.map((artifact) => [
      artifact.componentId,
      artifact.relPath,
    ]),
  );
  let verified = 0;
  for (const component of manifest.components) {
    const relPath = realByComponentId.get(component.component_id);
    if (relPath === undefined) {
      throw new ShipError(
        "NOT_FOUND",
        `component ${component.component_id} is not a real release artifact`,
      );
    }
    const bytes = readRealArtifact(root, relPath, component.component_id);
    const actual = digestBytes(bytes);
    if (actual !== component.digest) {
      throw new ShipError(
        "VERIFICATION_FAILED",
        `component ${component.component_id} digest mismatch (tampered artifact)`,
      );
    }
    verified += 1;
  }
  // eslint-disable-next-line no-console
  console.log(
    `verify-manifest: ok (${verified} components verified against real artifact bytes)`,
  );
  // eslint-disable-next-line no-console
  console.log(
    `verify-manifest: manifest digest ${manifest.manifest_digest} valid`,
  );
}

async function main(): Promise<void> {
  // M4 no-partial-output guarantee: cancelled work must never strand
  // an atomic-write temp file.
  installSignalCleanup();
  switch (command) {
    case "readiness":
      await commandReadiness();
      break;
    case "manifest":
      await commandManifest();
      break;
    case "ship-gate-status":
      await commandShipGateStatus();
      break;
    case "certification-rows":
      await commandCertificationRows();
      break;
    case "verify-manifest":
      await commandVerifyManifest();
      break;
    default:
      // eslint-disable-next-line no-console
      console.error(
        "usage: release-evidence-cli <readiness|manifest|ship-gate-status|certification-rows|verify-manifest> [--output FILE] [--output-dir DIR] [--manifest FILE]",
      );
      process.exitCode = 2;
  }
}

void main().catch((error: unknown) => {
  // Structured fail-closed error surface (M4): every failure emits one
  // redacted JSON line with SPEC-006 code, class, message, and the
  // redaction flag so operators and incident tooling can correlate.
  const shape =
    error instanceof ShipError
      ? error.toShape()
      : {
          code: "INTERNAL_INVARIANT" as const,
          class: "ShipError",
          message: redactShipMessage(
            error instanceof Error ? error.message : String(error),
          ),
          redacted: true,
        };
  process.stderr.write(`${JSON.stringify(shape)}\n`);
  process.exitCode = 1;
});
