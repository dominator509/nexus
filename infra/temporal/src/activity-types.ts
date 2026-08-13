/**
 * Activity registry types for the five nexus workflows (ADR-010).
 *
 * SPEC-023 behavior 6: external effects happen ONLY in activities. This
 * interface is the full surface the workflow bodies may invoke. EP-006
 * implements the approval-owned activities (verifyApproval,
 * applyCompensation) in src/activities.ts; provider-bound effect and
 * verification activities (runEffect, verifyEffect) are registered by the
 * nodes that own those providers (EP-017 agents, EP-019 remediation,
 * EP-029 social, EP-042 deployment) through the worker factory. A worker
 * that is asked to run an unregistered activity fails closed with a typed
 * error, never silently.
 */

import type {
  ActionDigest,
  ActionId,
  ApprovalSignal,
  WorkflowId,
} from "@nexus/workflows";
import type { AuthenticationStrength } from "@nexus/workflows";

export interface VerifyApprovalInput {
  readonly workflowId: WorkflowId;
  readonly actionId: ActionId;
  readonly actionDigest: ActionDigest;
  readonly signal: ApprovalSignal;
  readonly requiredStrength: AuthenticationStrength;
}

export interface VerifyApprovalOutput {
  readonly digestMatch: boolean;
  readonly strengthOk: boolean;
  readonly verifiedAt: string;
}

export interface ApplyCompensationInput {
  readonly workflowId: WorkflowId;
  readonly effectIdempotencyKey: string;
  /** Derived as comp:<effectIdempotencyKey> by the ledger contract. */
  readonly compensationKey: string;
  readonly reason: string;
}

export interface ApplyCompensationOutput {
  readonly compensated: true;
  readonly compensationKey: string;
}

/** Provider-bound effect execution (registered by owning nodes). */
export interface RunEffectInput {
  readonly workflowId: WorkflowId;
  readonly idempotencyKey: string;
  readonly actionDigest: ActionDigest;
  readonly payload: unknown;
}

export interface RunEffectOutput {
  readonly receiptId?: string;
}

/** Provider-bound effect verification (registered by owning nodes). */
export interface VerifyEffectInput {
  readonly workflowId: WorkflowId;
  readonly idempotencyKey: string;
  readonly receiptId?: string;
  readonly expectedState: unknown;
}

export interface VerifyEffectOutput {
  readonly verified: boolean;
}

export interface NexusActivityRegistry {
  verifyApproval(input: VerifyApprovalInput): Promise<VerifyApprovalOutput>;
  applyCompensation(
    input: ApplyCompensationInput,
  ): Promise<ApplyCompensationOutput>;
  runEffect(input: RunEffectInput): Promise<RunEffectOutput>;
  verifyEffect(input: VerifyEffectInput): Promise<VerifyEffectOutput>;
}
