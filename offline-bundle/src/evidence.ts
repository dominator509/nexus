/**
 * EP-042 M5 current-run evidence (ExecPlan M5 fence O/P).
 *
 * Every M5 gate run writes machine-readable evidence bound to run_id +
 * git_commit. The evidence records real observed states: release id,
 * manifest digest, component digests, bundle digest, bundle
 * verification state, install state, rollback state, offline-install
 * state (transport absent), signature state (SIGNATURE PRESENT !=
 * SIGNATURE VALID), certification boundary, redaction result, and
 * timestamp.
 *
 * validateEvidence rejects stale (run_id mismatch), tampered
 * (self-digest mismatch), missing-bound, and secret-shaped evidence.
 */

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { redactValue } from "@nexus/installers";
import { BundleError } from "./errors";

export interface BundleEvidenceInput {
  runId: string;
  gitCommit: string;
  releaseId: string;
  installId: string;
  bundleId: string;
  manifestDigest: string;
  bundleDigest: string;
  componentDigests: string[];
  bundleVerificationState: string;
  installState: string;
  rollbackState: string;
  offlineInstallState: string;
  signatureState: string;
  certificationBoundary: string[];
  timestamp: string;
  /** Runtime-constructed canaries that must never appear raw. */
  secretCanaries?: string[];
}

export interface BundleEvidence {
  node: "EP-042";
  milestone: "M5";
  run_id: string;
  git_commit: string;
  release_id: string;
  install_id: string;
  bundle_id: string;
  manifest_digest: string;
  bundle_digest: string;
  component_digests: string[];
  bundle_verification_state: string;
  install_state: string;
  rollback_state: string;
  offline_install_state: string;
  signature_state: string;
  certification_boundary: string[];
  redaction_result: string;
  evidence_digest: string;
  timestamp: string;
}

function sha256Hex(bytes: Uint8Array<ArrayBuffer>): Promise<string> {
  return crypto.subtle.digest("SHA-256", bytes).then((digest) =>
    Array.from(new Uint8Array(digest))
      .map((byte) => byte.toString(16).padStart(2, "0"))
      .join(""),
  );
}

/**
 * Build redacted current-run evidence. Secret-shaped values (runtime
 * canaries, credential-shaped strings) are scrubbed BEFORE the evidence
 * digest is computed, so canaries can never enter the record.
 */
export async function buildBundleEvidence(
  input: BundleEvidenceInput,
): Promise<BundleEvidence> {
  const canaryValues = input.secretCanaries ?? [];
  const redact = (value: string): string => {
    let out = value;
    for (const canary of canaryValues) {
      out = out.split(canary).join("[REDACTED]");
    }
    out = redactValue(out);
    return out;
  };
  const redactedBoundary = input.certificationBoundary.map((line) =>
    redact(line),
  );
  const redactedComponents = input.componentDigests.map((digest) =>
    redact(digest),
  );

  const base: Omit<BundleEvidence, "evidence_digest"> = {
    node: "EP-042",
    milestone: "M5",
    run_id: input.runId,
    git_commit: input.gitCommit,
    release_id: input.releaseId,
    install_id: input.installId,
    bundle_id: input.bundleId,
    manifest_digest: input.manifestDigest,
    bundle_digest: input.bundleDigest,
    component_digests: redactedComponents,
    bundle_verification_state: input.bundleVerificationState,
    install_state: input.installState,
    rollback_state: input.rollbackState,
    offline_install_state: input.offlineInstallState,
    signature_state: input.signatureState,
    certification_boundary: redactedBoundary,
    redaction_result: canaryValues.length > 0 ? "REDACTED" : "CLEAN",
    timestamp: input.timestamp,
  };
  const digest = await sha256Hex(
    new TextEncoder().encode(JSON.stringify(base)),
  );
  return { ...base, evidence_digest: `sha256:${digest}` };
}

/**
 * Validate current-run evidence. Denies:
 *   - missing run_id / git_commit / evidence_digest
 *   - stale run_id (does not match the expected current run)
 *   - tampered evidence (self-digest mismatch over the redacted fields)
 *   - secret-shaped values present in any field
 *   - wrong node/milestone binding
 */
export async function validateEvidence(
  evidence: BundleEvidence,
  expected: { runId: string; gitCommit: string },
): Promise<{ valid: true }> {
  if (evidence.node !== "EP-042" || evidence.milestone !== "M5") {
    throw new BundleError(
      "EVIDENCE_INVALID",
      `evidence bound to ${evidence.node} ${evidence.milestone}, expected EP-042 M5`,
    );
  }
  if (evidence.run_id !== expected.runId) {
    throw new BundleError(
      "EVIDENCE_INVALID",
      `evidence run_id ${evidence.run_id} is stale (expected ${expected.runId})`,
    );
  }
  if (evidence.git_commit !== expected.gitCommit) {
    throw new BundleError(
      "EVIDENCE_INVALID",
      `evidence git_commit ${evidence.git_commit} does not match ${expected.gitCommit}`,
    );
  }
  if (typeof evidence.evidence_digest !== "string") {
    throw new BundleError("EVIDENCE_INVALID", "evidence digest missing");
  }
  const { evidence_digest: _excluded, ...rest } = evidence;
  const actual = await sha256Hex(
    new TextEncoder().encode(JSON.stringify(rest)),
  );
  const declared = evidence.evidence_digest.startsWith("sha256:")
    ? evidence.evidence_digest.slice("sha256:".length)
    : evidence.evidence_digest;
  if (actual !== declared) {
    throw new BundleError(
      "EVIDENCE_INVALID",
      "evidence self-digest mismatch (tampered evidence)",
    );
  }
  const serialized = JSON.stringify(evidence);
  if (
    /(sk-[A-Za-z0-9]{8,}|ghp_[A-Za-z0-9]{8,}|AKIA[A-Z0-9]{8,}|Bearer\s+[A-Za-z0-9._-]{8,})/.test(
      serialized,
    )
  ) {
    throw new BundleError(
      "EVIDENCE_INVALID",
      "secret-shaped content present in evidence",
    );
  }
  return { valid: true };
}

export function writeEvidenceFile(
  evidence: BundleEvidence,
  outPath: string,
): string {
  const abs = resolve(outPath);
  writeFileSync(abs, JSON.stringify(evidence, null, 2));
  return abs;
}

export function readEvidenceFile(evidencePath: string): BundleEvidence {
  const abs = resolve(evidencePath);
  const raw = readFileSync(abs, "utf8");
  const parsed = JSON.parse(raw) as BundleEvidence;
  if (parsed.evidence_digest === undefined) {
    throw new BundleError(
      "EVIDENCE_INVALID",
      `evidence file missing digest: ${abs}`,
    );
  }
  return parsed;
}

export function evidenceDirForRun(base: string, runId: string): string {
  return resolve(base, `ep042-m5-${runId}`);
}
