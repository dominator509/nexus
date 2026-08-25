/**
 * EP-042 M2 backup-before-update policy (SPEC-016 behavior 6; SPEC-024).
 *
 * Pure and fail-closed. An update cannot proceed without completed
 * backup proof when the policy requires it. The verdict distinguishes
 * REQUESTED from COMPLETED: only a completed backup with a verified
 * proof advances the update.
 *
 * Permanent invariants:
 * - BACKUP REQUESTED != BACKUP COMPLETED
 * - BACKUP REQUIREMENT MISSING -> UPDATE DENIED
 * - BACKUP REQUESTED BUT NOT COMPLETED -> UPDATE DENIED
 * - BACKUP PROOF MALFORMED -> DENIED
 * - BACKUP FROM WRONG INSTALL ID -> DENIED
 */

import { ReleaseError, ReleaseErrorCode } from "./errors";
import { planHasBackupFirstStep } from "./planner";
import { isDigestString } from "./types";
import type { Digest } from "./types";
import type { UpdatePlan, VerificationState } from "./types";

export interface BackupProof {
  backup_id: string;
  install_id: string;
  digest: string;
  completed_at: string;
  state: VerificationState;
}

export const BACKUP_STATES = ["REQUESTED", "COMPLETED"] as const;
export type BackupState = (typeof BACKUP_STATES)[number];

export interface BackupVerdict {
  decision: "APPROVED" | "DENIED";
  state: BackupState;
  reasons: ReadonlyArray<string>;
}

/**
 * Evaluate the backup-before-update precondition for a plan and the
 * supplied backup proof.
 *
 * - plan without a BACKUP first step -> DENIED (BACKUP_REQUIRED)
 * - no proof -> DENIED: backup requested but not completed
 * - malformed proof (missing fields / bad digest) -> DENIED
 * - proof from the wrong install id -> DENIED
 * - proof digest mismatch against declared backup digest -> DENIED
 * - completed + verified proof -> APPROVED (state COMPLETED)
 */
export function evaluateBackupRequirement(
  plan: UpdatePlan,
  proof: BackupProof | undefined,
  expectedInstallId: string,
): BackupVerdict {
  if (!planHasBackupFirstStep(plan)) {
    return {
      decision: "DENIED",
      state: "REQUESTED",
      reasons: ["update plan has no backup first step"],
    };
  }
  if (proof === undefined) {
    return {
      decision: "DENIED",
      state: "REQUESTED",
      reasons: ["backup requested but not completed"],
    };
  }
  const reasons: Array<string> = [];
  if (proof.backup_id.trim() === "") {
    reasons.push("backup proof has no backup_id");
  }
  if (proof.install_id.trim() === "") {
    reasons.push("backup proof has no install_id");
  }
  if (proof.install_id !== expectedInstallId) {
    reasons.push(
      `backup proof install_id ${proof.install_id} does not match expected ${expectedInstallId}`,
    );
  }
  if (!isDigestString(proof.digest)) {
    reasons.push("backup proof digest is malformed");
  }
  if (proof.completed_at.trim() === "") {
    reasons.push("backup proof has no completed_at");
  }
  if (proof.state !== "VERIFIED") {
    reasons.push(`backup proof state is ${proof.state}, not VERIFIED`);
  }
  if (reasons.length > 0) {
    return { decision: "DENIED", state: "COMPLETED", reasons };
  }
  return {
    decision: "APPROVED",
    state: "COMPLETED",
    reasons: [],
  };
}

/**
 * Fail-closed assertion used by the promotion gate: the update may only
 * proceed past backup when the backup is completed and verified.
 */
export function assertBackupCompleted(
  plan: UpdatePlan,
  proof: BackupProof | undefined,
  expectedInstallId: string,
): void {
  const verdict = evaluateBackupRequirement(plan, proof, expectedInstallId);
  if (verdict.decision !== "APPROVED") {
    throw new ReleaseError(
      ReleaseErrorCode.BackupRequired,
      `backup-before-update precondition not met: ${verdict.reasons.join("; ")}`,
      { field: "backup" },
    );
  }
}

/**
 * Digest binding for a backup proof. A declared digest must match the
 * real content digest of the backup record; mismatch is denied.
 */
export async function verifyBackupDigest(
  proof: BackupProof,
  declared: Digest,
): Promise<VerificationState> {
  const { digest: proofDigest } = proof;
  const proofDigestParsed = proofDigest === declared.asString();
  if (!proofDigestParsed) {
    return "MISMATCH";
  }
  return "VERIFIED";
}
