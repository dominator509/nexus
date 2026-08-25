/**
 * EP-042 M2 canary / manual promotion gate (SPEC-016 behavior 6, 7).
 *
 * Pure and fail-closed. A canary ring exists != rollout approved. The
 * gate returns only LOCKED, AWAITING_HUMAN_APPROVAL, or
 * APPROVED_MANUAL_ONLY - never an automatic deployment decision. It
 * enforces that promotion does not bypass compatibility, backup, or
 * rollback preconditions.
 *
 * Permanent invariants:
 * - CANARY RING EXISTS != ROLLOUT APPROVED
 * - RING HEALTH MISSING -> PROMOTION DENIED
 * - RING FAILURE -> PROMOTION DENIED
 * - UNKNOWN RING -> DENIED
 * - MANUAL APPROVAL MISSING -> DENIED
 * - MANUAL PROMOTION OBJECT EXISTS != AUTOMATIC DEPLOYMENT
 */

import { ReleaseError, ReleaseErrorCode } from "./errors";
import type {
  CanaryRing,
  CompatibilityMatrix,
  ManualPromotion,
  SignedComponent,
  UpdatePlan,
} from "./types";
import { assertBackupCompleted, type BackupProof } from "./backup";
import { assertRollbackProven, type RollbackDrillEvidence } from "./rollback";
import type { RollbackReceipt } from "./types";

export interface PromotionVerdict {
  decision: "LOCKED" | "AWAITING_HUMAN_APPROVAL" | "APPROVED_MANUAL_ONLY";
  requiresHuman: boolean;
  reasons: ReadonlyArray<string>;
}

/**
 * Evaluate the promotion gate for a canary ring and approval reference.
 * The ring must be READY_TO_PROMOTE with evidence; anything short of an
 * exact human approval reference is not approved. This function never
 * returns an automatic deployment decision.
 */
export function evaluatePromotionGate(
  ring: CanaryRing,
  approvalRef: string | undefined,
): PromotionVerdict {
  if (ring.verdict === "ROLLBACK") {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: ["canary verdict is ROLLBACK"],
    };
  }
  if (ring.verdict !== "READY_TO_PROMOTE") {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: ["canary verdict is not READY_TO_PROMOTE"],
    };
  }
  if (ring.health_criterion.trim() === "") {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: ["canary health criterion is missing"],
    };
  }
  if (ring.evidence_ref === undefined || ring.evidence_ref.trim() === "") {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: ["canary evidence is missing"],
    };
  }
  if (approvalRef === undefined || approvalRef.trim() === "") {
    return {
      decision: "AWAITING_HUMAN_APPROVAL",
      requiresHuman: true,
      reasons: ["exact manual approval required"],
    };
  }
  return {
    decision: "APPROVED_MANUAL_ONLY",
    requiresHuman: true,
    reasons: ["promotion authorized as exact manual action"],
  };
}

/**
 * Full promotion gate with preconditions: the ring must reference the
 * release, the update must have completed backup and proven rollback,
 * and the manual approval must exist. Promotion never bypasses
 * compatibility, backup, or rollback preconditions.
 */
export async function evaluateFullPromotionGate(input: {
  ring: CanaryRing;
  releaseId: string;
  plan: UpdatePlan;
  matrix: CompatibilityMatrix;
  components: ReadonlyArray<SignedComponent>;
  backupProof: BackupProof | undefined;
  rollbackReceipt: RollbackReceipt | undefined;
  rollbackDrill: RollbackDrillEvidence | undefined;
  installId: string;
  approvalRef: string | undefined;
}): Promise<PromotionVerdict> {
  if (input.ring.release_id !== input.releaseId) {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: [
        `canary ring ${input.ring.ring_id} references release ${input.ring.release_id}, expected ${input.releaseId}`,
      ],
    };
  }
  try {
    assertBackupCompleted(input.plan, input.backupProof, input.installId);
  } catch (error) {
    if (error instanceof ReleaseError) {
      return {
        decision: "LOCKED",
        requiresHuman: true,
        reasons: [`backup precondition denied: ${error.message}`],
      };
    }
    throw error;
  }
  try {
    assertRollbackProven(
      input.plan,
      input.matrix,
      input.components,
      input.rollbackReceipt,
      input.rollbackDrill,
    );
  } catch (error) {
    if (error instanceof ReleaseError) {
      return {
        decision: "LOCKED",
        requiresHuman: true,
        reasons: [`rollback precondition denied: ${error.message}`],
      };
    }
    throw error;
  }
  return evaluatePromotionGate(input.ring, input.approvalRef);
}

/**
 * A ManualPromotion record is a decision record, never an executor: it
 * carries the exact manual command and requires a human approval
 * reference. It can never perform deployment itself.
 */
export function promotionNeverDeploys(promotion: ManualPromotion): boolean {
  return (
    promotion.approval_ref.trim() !== "" &&
    promotion.state === "APPROVED_MANUAL_ONLY" &&
    promotion.exact_manual_command.trim() !== ""
  );
}
