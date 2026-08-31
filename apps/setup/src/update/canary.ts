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
 * - APPROVAL REF STRING EXISTS != APPROVAL RECORD VERIFIED (AUD-070)
 * - APPROVAL RECORD VERIFIED == AUTHENTICATED APPROVER + RECORD BINDING
 *   + EXPIRY + REAL SIGNATURE + POLICY LOOKUP + REQUESTER/APPROVER
 *   SEPARATION
 * - MANUAL PROMOTION OBJECT EXISTS != AUTOMATIC DEPLOYMENT
 *
 * AUD-070: promotion authority is NEVER reducible to a nonempty string.
 * A promotion is approved only when a full ManualPromotion record:
 *   1. is cryptographically signed by a pinned Ed25519 key (the
 *      signature is verified over the canonical approval payload);
 *   2. was approved by an approver listed in the authorized approver
 *      policy (policy lookup, not self-assertion);
 *   3. was approved within the validity window (expiry; a stale or
 *      future-dated approval is denied);
 *   4. was approved by an identity different from the requester
 *      (requester/approver separation);
 *   5. binds to the exact canary ring and release under evaluation
 *      (record binding);
 *   6. is in APPROVED_MANUAL_ONLY state and carries the exact manual
 *      command.
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
import { canonicalJsonBytes } from "./digest";

export interface PromotionVerdict {
  decision: "LOCKED" | "AWAITING_HUMAN_APPROVAL" | "APPROVED_MANUAL_ONLY";
  requiresHuman: boolean;
  reasons: ReadonlyArray<string>;
}

/**
 * Policy governing a manual promotion approval (AUD-070). Authority is
 * derived from this policy, never from a bare string in the request.
 */
export interface PromotionApprovalPolicy {
  /** Identities authorized to approve promotions (policy lookup). */
  authorizedApprovers: ReadonlySet<string>;
  /** Pinned Ed25519 public key (raw 32-byte) that must verify the signature. */
  approverPublicKey: Uint8Array<ArrayBuffer>;
  /** Approval validity window in minutes; approved_at must be within it. */
  validityMinutes: number;
  /** Identity of the requester; must differ from the approver. */
  requesterId: string;
  /** Reference time (ms since epoch) for expiry evaluation. */
  nowMs?: number;
}

/**
 * Canonical approval payload: the exact bytes over which the approval
 * signature is computed. Every authority-relevant field is included;
 * the signature envelope itself is excluded. Field order is fixed.
 */
export function canonicalApprovalPayload(
  promotion: ManualPromotion,
): Uint8Array<ArrayBuffer> {
  return canonicalJsonBytes({
    schema_version: promotion.schema_version,
    promotion_id: promotion.promotion_id,
    release_id: promotion.release_id,
    update_plan_ref: promotion.update_plan_ref,
    canary_ring_ref: promotion.canary_ring_ref,
    approval_ref: promotion.approval_ref,
    approver: promotion.approver,
    approved_at: promotion.approved_at,
    state: promotion.state,
    exact_manual_command: promotion.exact_manual_command,
  });
}

function base64ToBytes(valueB64: string): Uint8Array<ArrayBuffer> {
  // Node >= 16 and browsers both provide atob; the update core stays
  // framework-neutral by using the platform global.
  const binary = globalThis.atob(valueB64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

/**
 * Verify the real Ed25519 approval signature against the pinned public
 * key over the canonical approval payload (AUD-070). Returns false for
 * a missing, malformed, or cryptographically invalid signature.
 */
export async function verifyApprovalSignature(
  promotion: ManualPromotion,
  policy: PromotionApprovalPolicy,
): Promise<boolean> {
  if (promotion.signature.algorithm !== "ED25519") {
    return false;
  }
  if (promotion.signature.value_b64.trim() === "") {
    return false;
  }
  try {
    const payload = canonicalApprovalPayload(promotion);
    const signature = base64ToBytes(promotion.signature.value_b64);
    const key = await globalThis.crypto.subtle.importKey(
      "raw",
      policy.approverPublicKey,
      { name: "Ed25519" },
      false,
      ["verify"],
    );
    return globalThis.crypto.subtle.verify(
      "Ed25519",
      key,
      signature,
      payload,
    );
  } catch {
    return false;
  }
}

/**
 * Evaluate the promotion gate for a canary ring and a full approval
 * record (AUD-070). A bare approval reference string is NOT accepted:
 * approval authority requires a signed ManualPromotion record that
 * passes every policy check. This function never returns an automatic
 * deployment decision.
 */
export async function evaluatePromotionGate(
  ring: CanaryRing,
  promotion: ManualPromotion | undefined,
  policy: PromotionApprovalPolicy,
): Promise<PromotionVerdict> {
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
  if (promotion === undefined) {
    return {
      decision: "AWAITING_HUMAN_APPROVAL",
      requiresHuman: true,
      reasons: ["signed manual approval record required"],
    };
  }
  if (promotion.state !== "APPROVED_MANUAL_ONLY") {
    return {
      decision: "AWAITING_HUMAN_APPROVAL",
      requiresHuman: true,
      reasons: ["approval record is not in APPROVED_MANUAL_ONLY state"],
    };
  }
  if (promotion.exact_manual_command.trim() === "") {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: ["approval record carries no exact manual command"],
    };
  }
  // Record binding: the approval must reference this exact ring.
  if (promotion.canary_ring_ref !== ring.ring_id) {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: [
        `approval record references ring ${promotion.canary_ring_ref}, expected ${ring.ring_id}`,
      ],
    };
  }
  if (promotion.release_id !== ring.release_id) {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: [
        `approval record references release ${promotion.release_id}, expected ${ring.release_id}`,
      ],
    };
  }
  // Policy lookup: the approver must be authorized.
  if (!policy.authorizedApprovers.has(promotion.approver)) {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: [`approver ${promotion.approver} is not authorized to approve`],
    };
  }
  // Requester/approver separation: the requester cannot approve itself.
  if (promotion.approver === policy.requesterId) {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: ["requester and approver must be different identities"],
    };
  }
  // Expiry: approved_at must be within the validity window.
  const nowMs = policy.nowMs ?? Date.now();
  const approvedMs = Date.parse(promotion.approved_at);
  if (Number.isNaN(approvedMs)) {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: ["approval timestamp is invalid"],
    };
  }
  const validityMs = policy.validityMinutes * 60_000;
  if (approvedMs < nowMs - validityMs) {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: ["approval record has expired"],
    };
  }
  if (approvedMs > nowMs + 5 * 60_000) {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: ["approval record is dated in the future"],
    };
  }
  // Real signature over the canonical payload (AUD-070).
  const signatureValid = await verifyApprovalSignature(promotion, policy);
  if (!signatureValid) {
    return {
      decision: "LOCKED",
      requiresHuman: true,
      reasons: ["approval signature does not verify against the pinned key"],
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
 * and the manual approval must be a verified record (AUD-070).
 * Promotion never bypasses compatibility, backup, or rollback
 * preconditions.
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
  promotion: ManualPromotion | undefined;
  policy: PromotionApprovalPolicy;
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
  return evaluatePromotionGate(input.ring, input.promotion, input.policy);
}

/**
 * A ManualPromotion record is a decision record, never an executor: it
 * carries the exact manual command and requires a verified approval
 * signature and state. It can never perform deployment itself.
 */
export function promotionNeverDeploys(promotion: ManualPromotion): boolean {
  return (
    promotion.approval_ref.trim() !== "" &&
    promotion.state === "APPROVED_MANUAL_ONLY" &&
    promotion.exact_manual_command.trim() !== "" &&
    promotion.signature.value_b64.trim() !== ""
  );
}
