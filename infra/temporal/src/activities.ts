/**
 * EP-006-owned activity implementations (SPEC-023 behavior 6; SPEC-006
 * behaviors 5 and 8).
 *
 * Activities run OUTSIDE the workflow isolate and are the only surface
 * allowed to touch the world. The approval-owned activities here are
 * real, idempotent, and fail closed; provider-bound effect activities are
 * registered by their owning nodes (see src/activity-types.ts).
 */

import {
  assertApprovalBinding,
  isUuidV7,
  workflowError,
} from "@nexus/workflows";
import type {
  ApplyCompensationInput,
  ApplyCompensationOutput,
  VerifyApprovalInput,
  VerifyApprovalOutput,
} from "./activity-types.js";

const ISO_NOW = "1970-01-01T00:00:00.000Z";

/**
 * Verify an approval assertion against the exact action digest and the
 * required authentication strength. This is the verification step of the
 * approval gate (SPEC-006 behavior 5): external success is not accepted
 * until the assertion verifies. Throws a typed VALIDATION failure when the
 * binding does not hold, so the workflow treats the approval as invalid.
 */
export async function verifyApproval(
  input: VerifyApprovalInput,
): Promise<VerifyApprovalOutput> {
  if (!isUuidV7(input.workflowId)) {
    throw workflowError("VALIDATION", `invalid workflowId ${input.workflowId}`);
  }
  const digestMatch = input.signal.actionDigest === input.actionDigest;
  const strengthOk =
    input.signal.authentication.strength === input.requiredStrength;
  try {
    assertApprovalBinding(
      input.signal,
      input.actionId,
      input.actionDigest,
      input.requiredStrength,
    );
  } catch {
    throw workflowError(
      "VALIDATION",
      "approval assertion does not bind to the action digest or strength",
      { workflowId: input.workflowId },
    );
  }
  return {
    digestMatch,
    strengthOk,
    // Deterministic timestamp: the engine clock is the source of truth in
    // the workflow; here we return the assertion's own decided time.
    verifiedAt: input.signal.decidedAt ?? ISO_NOW,
  };
}

/**
 * Apply a compensation step. For EP-006-owned effects (approval waits,
 * orchestration bookkeeping) the compensation is the durable, idempotent
 * execution point: it validates the compensation key derivation and
 * returns the result the workflow state machine commits. Provider-specific
 * rollback bodies are registered by the owning nodes and compose with
 * this entry point. Throws typed errors on malformed input; re-delivery
 * with the same compensationKey is idempotent by contract.
 */
export async function applyCompensation(
  input: ApplyCompensationInput,
): Promise<ApplyCompensationOutput> {
  if (!isUuidV7(input.workflowId)) {
    throw workflowError("VALIDATION", `invalid workflowId ${input.workflowId}`);
  }
  if (input.effectIdempotencyKey.length === 0) {
    throw workflowError(
      "VALIDATION",
      "effectIdempotencyKey must not be empty",
      {
        workflowId: input.workflowId,
      },
    );
  }
  const expected = `comp:${input.effectIdempotencyKey}`;
  if (input.compensationKey !== expected) {
    throw workflowError(
      "VALIDATION",
      `compensationKey must equal comp:<effectIdempotencyKey>, got ${input.compensationKey}`,
      { workflowId: input.workflowId },
    );
  }
  return { compensated: true, compensationKey: input.compensationKey };
}
