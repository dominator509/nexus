/**
 * EP-043 M2 repository state adapter (SPEC-008).
 *
 * Real I/O: reads the actual repository state (GRAPH.md node table,
 * append-only LEDGER, live-fire registry, certification RESULTS.md
 * files, evidence dir) into typed readiness inputs. This adapter may
 * import the pure domain (readiness.ts) but never the reverse.
 *
 * All reads are defensive: missing files yield typed PENDING/NOT_RUN
 * states so the evaluation fails closed honestly.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import type {
  CertificationMatrixInput,
  DrillInput,
  GraphNodeStatus,
  LiveFireProofResult,
  ReadinessInputs,
  ReviewInput,
} from "./readiness.ts";
import { REVIEW_DOMAINS } from "./model.ts";
import { ShipError } from "./errors.ts";
import {
  currentGitCommit,
  loadValidatedEvidence,
  loadEvidenceRecord,
} from "./evidence.ts";

export interface RepoPaths {
  root: string;
  graphPath: string;
  ledgerPath: string;
  registryPath: string;
  providerCertPath: string;
  hardwareCertPath: string;
  evidenceDir: string;
}

export function defaultRepoPaths(root = process.cwd()): RepoPaths {
  return {
    root,
    graphPath: join(root, ".agent", "GRAPH.md"),
    ledgerPath: join(root, ".agent", "state", "LEDGER.md"),
    registryPath: join(root, "live-fire", "REGISTRY.tsv"),
    providerCertPath: join(root, "provider-certification", "RESULTS.md"),
    hardwareCertPath: join(root, "hardware", "CERTIFICATION_RESULTS.md"),
    evidenceDir: join(root, ".agent", "state", "evidence"),
  };
}

function readIfExists(path: string): string | undefined {
  try {
    return readFileSync(path, "utf8");
  } catch {
    return undefined;
  }
}

/** Parse the GRAPH.md node table rows (| EP-NNN | DEP | ... |). */
export function collectGraphNodes(paths: RepoPaths): GraphNodeStatus[] {
  const graph = readIfExists(paths.graphPath);
  const ledger = readIfExists(paths.ledgerPath);
  const ledgerText = ledger ?? "";
  if (!graph) {
    return [];
  }
  const nodes: GraphNodeStatus[] = [];
  const re = /^\|\s*(EP-\d+)\s*\|/gm;
  let match: RegExpExecArray | null;
  while ((match = re.exec(graph)) !== null) {
    const nodeId = match[1]!;
    nodes.push({
      nodeId,
      done: new RegExp(`\\| ${nodeId} \\| NODE_DONE \\|`).test(ledgerText),
    });
  }
  return nodes;
}

/** Parse the live-fire registry rows (LF-NNN|owner|script|slug|desc). */
export function collectLiveFireProofs(paths: RepoPaths): LiveFireProofResult[] {
  const registry = readIfExists(paths.registryPath);
  const ledger = readIfExists(paths.ledgerPath);
  const ledgerText = ledger ?? "";
  if (!registry) {
    return [];
  }
  const commit = currentGitCommit(paths.root);
  const results: LiveFireProofResult[] = [];
  for (const line of registry.split("\n")) {
    if (line.trim().length === 0 || line.startsWith("#")) continue;
    const parts = line.split("|").map((part) => part.trim());
    const lfId = parts[0] ?? "";
    const owner = parts[1] ?? "";
    const slug = parts[3] ?? "";
    if (!/^LF-\d+$/.test(lfId)) continue;
    const evidenceFile = findEvidenceFile(paths.evidenceDir, lfId);
    // RX-002: evidence is PASS only when the structured record validates
    // (exit 0, PASS result, bound to the current commit, fresh). A bare
    // filename without a valid structured record is NOT a pass.
    const validatedRecord = evidenceFile
      ? loadValidatedEvidence(paths.evidenceDir, lfId, {
          expectedCommit: commit,
        })
      : undefined;
    results.push({
      lfId,
      ownerNode: owner,
      slug,
      ownerDone: new RegExp(`\\| ${owner} \\| NODE_DONE \\|`).test(ledgerText),
      evidenceRef: evidenceFile,
      validated: validatedRecord !== undefined,
    });
  }
  return results;
}

/** Find the committed evidence file for an LF id (exact prefix match). */
export function findEvidenceFile(evidenceDir: string, lfId: string): string {
  try {
    const entries = readdirSync(evidenceDir);
    const match = entries.find((entry) => entry.startsWith(`${lfId}-`));
    return match ? join(".agent", "state", "evidence", match) : "";
  } catch {
    return "";
  }
}

/** Parse certification RESULTS.md files into typed rows. */
export function collectCertifications(
  paths: RepoPaths,
): CertificationMatrixInput {
  const providerText = readIfExists(paths.providerCertPath) ?? "";
  const hardwareText = readIfExists(paths.hardwareCertPath) ?? "";
  return {
    providerRows: parseCertificationText(providerText, "PROVIDER").map((row) =>
      verifyCertificationRow(row, paths),
    ),
    hardwareRows: parseCertificationText(hardwareText, "HARDWARE").map((row) =>
      verifyCertificationRow(row, paths),
    ),
  };
}

/**
 * AUD-074: a SIGNED textual marker is never verification. A row is
 * verified only when a structured execution evidence record with a
 * VERIFIED/SIGNED/PASS result validates for that row's proof id. The
 * evidenceRef is then the structured record path; otherwise the row
 * stays SIGNED with verified=false so the obligation fails closed.
 */
function verifyCertificationRow(
  row: CertificationMatrixInput["providerRows"][number],
  paths: RepoPaths,
): CertificationMatrixInput["providerRows"][number] {
  if (row.state !== "SIGNED") return row;
  const proofId = `ep043-cert-${row.rowId}`;
  const record = loadValidatedEvidence(paths.evidenceDir, proofId, {
    expectedCommit: currentGitCommit(paths.root),
    requiredResult: ["VERIFIED", "SIGNED", "PASS"],
  });
  if (!record) {
    return { ...row, verified: false };
  }
  const file = findEvidenceFile(paths.evidenceDir, proofId);
  return { ...row, verified: true, evidenceRef: file };
}

function parseCertificationText(
  text: string,
  domain: "PROVIDER" | "HARDWARE",
): CertificationMatrixInput["providerRows"] {
  const rows: CertificationMatrixInput["providerRows"] = [];
  // RELEASE-BLOCKING-PENDING marker on a line names a pending row.
  const re = /^\s*(RELEASE-BLOCKING-PENDING|PENDING|SIGNED)\s*:?\s*(.+)$/gm;
  // M4 fail-closed conflict detection: a certification row label that
  // appears more than once with different states is a contradiction the
  // operator must resolve; identical duplicates are collapsed to one.
  const seenByLabel = new Map<string, string>();
  let match: RegExpExecArray | null;
  while ((match = re.exec(text)) !== null) {
    const state = match[1]! as
      | "RELEASE-BLOCKING-PENDING"
      | "PENDING"
      | "SIGNED";
    const label = match[2]!.trim();
    const labelKey = label.toLowerCase().replace(/\s+/g, " ").trim();
    const prior = seenByLabel.get(labelKey);
    if (prior !== undefined && prior !== state) {
      throw new ShipError(
        "CONFLICT",
        `conflicting certification rows for "${label.slice(0, 60)}" (${prior} vs ${state})`,
      );
    }
    if (prior !== undefined) {
      continue;
    }
    seenByLabel.set(labelKey, state);
    rows.push({
      rowId: `${domain.toLowerCase()}-${rows.length + 1}-${label.slice(0, 24).replace(/[^A-Za-z0-9]+/g, "-")}`,
      domain,
      state,
      ...(state === "SIGNED"
        ? { evidenceRef: "provider-certification/RESULTS.md" }
        : {}),
    });
  }
  // A file with no parseable row but present content is a PENDING row.
  if (rows.length === 0 && text.trim().length > 0) {
    rows.push({
      rowId: `${domain.toLowerCase()}-unparsed`,
      domain,
      state: "PENDING",
    });
  }
  return rows;
}

/** Collect review results from structured gate evidence files. */
export function collectReviews(paths: RepoPaths): ReviewInput[] {
  const reviews: ReviewInput[] = [];
  const commit = currentGitCommit(paths.root);
  for (const domain of REVIEW_DOMAINS) {
    const prefix = `ep043-review-${domain.toLowerCase()}`;
    const found =
      findEvidenceFile(paths.evidenceDir, prefix) ||
      (exists(join(paths.root, ".agent", "state", "evidence", prefix))
        ? join(".agent", "state", "evidence", prefix)
        : "");
    // RX-002: a review passes only when the evidence file is a validated
    // structured record with a PASS result. Filename presence alone is NOT_RUN.
    let status: ReviewInput["status"] = "NOT_RUN";
    if (found) {
      const record = loadValidatedEvidence(paths.evidenceDir, prefix, {
        expectedCommit: commit,
        requiredResult: ["PASS", "APPROVED"],
      });
      status = record ? "PASS" : "NOT_RUN";
    }
    reviews.push({
      domain,
      status,
      evidenceRef: found,
    });
  }
  return reviews;
}

function exists(path: string): boolean {
  try {
    readFileSync(path);
    return true;
  } catch {
    return false;
  }
}

/** Collect drill evidence from validated structured evidence records. */
export function collectDrills(paths: RepoPaths): DrillInput[] {
  const drillKinds = [
    "RESTORE",
    "ROLLBACK",
    "PROVIDER_FAILOVER",
    "IDENTITY_RECOVERY",
    "SENTINEL_CONTAINMENT",
    "UPDATE_FAILURE",
  ] as const;
  const commit = currentGitCommit(paths.root);
  const drills: DrillInput[] = [];
  for (const kind of drillKinds) {
    const prefix = `ep043-drill-${kind.toLowerCase()}`;
    const found = findEvidenceFile(paths.evidenceDir, prefix);
    // RX-002: DATED_EVIDENCE only when the structured record validates
    // (exit 0, DATED_EVIDENCE result, current commit, fresh). A bare
    // filename is NOT_RUN.
    let status: DrillInput["status"] = "NOT_RUN";
    if (found) {
      const record = loadValidatedEvidence(paths.evidenceDir, prefix, {
        expectedCommit: commit,
        requiredResult: ["DATED_EVIDENCE", "VERIFIED", "PASS"],
      });
      status = record ? "DATED_EVIDENCE" : "NOT_RUN";
    }
    drills.push({
      kind,
      status,
      ...(status === "DATED_EVIDENCE"
        ? { datedAt: new Date().toISOString(), evidenceRef: found }
        : {}),
    });
  }
  return drills;
}

/** Collect the release tag from the git refs (green/EP-043 or HEAD). */
export function collectReleaseTag(root: string): string {
  try {
    const head = readFileSync(join(root, ".git", "HEAD"), "utf8").trim();
    return head;
  } catch {
    return "";
  }
}

/** Assemble all readiness inputs from real repository state. */
export function collectReadinessInputs(paths: RepoPaths): ReadinessInputs {
  const graphNodes = collectGraphNodes(paths);
  if (graphNodes.length === 0) {
    throw new ShipError(
      "UNAVAILABLE",
      "cannot read graph node table from GRAPH.md",
    );
  }
  const liveFireProofs = collectLiveFireProofs(paths);
  if (liveFireProofs.length === 0) {
    throw new ShipError("UNAVAILABLE", "cannot read live-fire registry");
  }
  const certifications = collectCertifications(paths);
  const reviews = collectReviews(paths);
  const drills = collectDrills(paths);
  const releaseTag = collectReleaseTag(paths.root);
  const manualDeployCommand = "sh scripts/deploy.sh --dry-run";
  return {
    graphNodes,
    liveFireProofs,
    certifications,
    reviews,
    drills,
    releaseTag,
    manualDeployCommand,
    freshCloneRerun: collectFreshCloneEvidence(paths),
  };
}

/**
 * RX-002 fresh-clone acceptance read. True only when a structured
 * fresh-clone evidence record validates: result VERIFIED (or PASS),
 * exit 0, bound to the current commit, and fresh. A bare filename is
 * never proof (AUD-075).
 */
export function collectFreshCloneEvidence(paths: RepoPaths): boolean {
  const commit = currentGitCommit(paths.root);
  const record = loadValidatedEvidence(paths.evidenceDir, "ep043-freshclone", {
    expectedCommit: commit,
    requiredResult: ["VERIFIED", "PASS"],
  });
  return record !== undefined;
}
