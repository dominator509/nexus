/**
 * RX-002 Evidence Truth Engine (SPEC-008 remediation, AUD-071..075).
 *
 * Replaces filename/string-presence evidence with structured execution
 * evidence. A filename is never a PASS. Every evidence record carries the
 * required fields; validation binds exit code, result, git commit, and
 * freshness. SIGNED/READY/APPROVED/VERIFIED/DATED_EVIDENCE require the
 * corresponding verifier, never a textual marker.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { ShipError } from "./errors.ts";

/** Required structured execution evidence record (RX-002 doctrine). */
export interface ExecutionEvidence {
  schema_version: number;
  proof_id: string;
  producer: string;
  command: string;
  started_at: string;
  completed_at: string;
  exit_code: number;
  result: string;
  git_commit: string;
  run_id: string;
  environment_class: string;
  artifact_digests: Record<string, string>;
  stdout_digest: string;
  stderr_digest: string;
}

const REQUIRED_FIELDS = [
  "schema_version",
  "proof_id",
  "producer",
  "command",
  "started_at",
  "completed_at",
  "exit_code",
  "result",
  "git_commit",
  "run_id",
  "environment_class",
  "artifact_digests",
  "stdout_digest",
  "stderr_digest",
] as const;

/** Parse a structured execution evidence record; fails closed on any gap. */
export function parseExecutionEvidence(raw: string): ExecutionEvidence {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw) as unknown;
  } catch {
    throw new ShipError(
      "VALIDATION_FAILED",
      "execution evidence is not valid JSON",
    );
  }
  if (typeof parsed !== "object" || parsed === null) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "execution evidence must be an object",
    );
  }
  const obj = parsed as Record<string, unknown>;
  for (const field of REQUIRED_FIELDS) {
    if (obj[field] === undefined || obj[field] === null) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `execution evidence missing required field: ${field}`,
      );
    }
  }
  if (obj["schema_version"] !== 1) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "unsupported execution evidence schema_version",
    );
  }
  if (typeof obj["exit_code"] !== "number") {
    throw new ShipError(
      "VALIDATION_FAILED",
      "execution evidence exit_code must be a number",
    );
  }
  if (typeof obj["artifact_digests"] !== "object" || obj["artifact_digests"] === null) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "execution evidence artifact_digests must be an object",
    );
  }
  for (const field of [
    "proof_id",
    "producer",
    "command",
    "started_at",
    "completed_at",
    "result",
    "git_commit",
    "run_id",
    "environment_class",
    "stdout_digest",
    "stderr_digest",
  ] as const) {
    if (typeof obj[field] !== "string" || (obj[field] as string).length === 0) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `execution evidence ${field} must be a non-empty string`,
      );
    }
  }
  return obj as unknown as ExecutionEvidence;
}

export interface ValidateOptions {
  expectedCommit?: string;
  maxAgeMs?: number;
  requiredResult?: string | string[];
}

const DEFAULT_MAX_AGE_MS = 30 * 24 * 3600 * 1000;

/**
 * Validate a parsed execution evidence record. Returns true only when the
 * record is fresh, bound to the expected commit, exited 0, and carries a
 * PASS/VERIFIED-style result. A filename is never a PASS; this is the
 * semantic check the presence-only collectors skipped.
 */
export function validateExecutionEvidence(
  evidence: ExecutionEvidence,
  options: ValidateOptions = {},
): boolean {
  if (evidence.exit_code !== 0) return false;
  const okResults = options.requiredResult
    ? Array.isArray(options.requiredResult)
      ? options.requiredResult
      : [options.requiredResult]
    : ["PASS", "VERIFIED", "APPROVED", "READY", "DATED_EVIDENCE"];
  if (!okResults.includes(evidence.result)) return false;
  if (options.expectedCommit && evidence.git_commit !== options.expectedCommit) {
    return false;
  }
  const maxAge = options.maxAgeMs ?? DEFAULT_MAX_AGE_MS;
  const completed = Date.parse(evidence.completed_at);
  if (Number.isNaN(completed)) return false;
  const age = Date.now() - completed;
  if (age < 0 || age > maxAge) return false;
  return true;
}

/** Resolve the repository HEAD commit (40-hex) or "unknown". */
export function currentGitCommit(root: string): string {
  try {
    const head = readFileSync(join(root, ".git", "HEAD"), "utf8").trim();
    if (head.startsWith("ref:")) {
      const refPath = head.slice(5).trim();
      return readFileSync(join(root, ".git", refPath), "utf8")
        .trim()
        .slice(0, 40);
    }
    return head.slice(0, 40);
  } catch {
    return "unknown";
  }
}

/**
 * Find the evidence file for a proof id under either naming scheme
 * (LF-NNN-* or EP-NNN-M5-LF-NNN-*), read it, and return the parsed
 * record. Returns undefined when missing, unreadable, or not structured.
 */
export function loadEvidenceRecord(
  evidenceDir: string,
  proofId: string,
): ExecutionEvidence | undefined {
  let entries: string[];
  try {
    entries = readdirSync(evidenceDir);
  } catch {
    return undefined;
  }
  const exact = entries.find((entry) => entry.startsWith(`${proofId}-`));
  const embedded = entries.find(
    (entry) =>
      entry.includes(`-${proofId}-`) ||
      entry.includes(`-${proofId}.`) ||
      entry.toLowerCase().includes(proofId.toLowerCase()),
  );
  const match = exact ?? embedded;
  if (!match) return undefined;
  try {
    const raw = readFileSync(join(evidenceDir, match), "utf8");
    return parseExecutionEvidence(raw);
  } catch {
    return undefined;
  }
}

/** Convenience: load + validate in one step. */
export function loadValidatedEvidence(
  evidenceDir: string,
  proofId: string,
  options: ValidateOptions = {},
): ExecutionEvidence | undefined {
  const record = loadEvidenceRecord(evidenceDir, proofId);
  if (!record) return undefined;
  return validateExecutionEvidence(record, options) ? record : undefined;
}
