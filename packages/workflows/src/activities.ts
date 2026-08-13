/**
 * Workflow activity contracts (SPEC-023 behavior 6; SPEC-006 behaviors 2,
 * 5, 7, 8; EP-006 acceptance obligations 1 and 4).
 *
 * Hard invariant: external effects happen ONLY in activities. Workflow code
 * never performs network, database, filesystem, or wall-clock operations;
 * it schedules activities through the deterministic WorkflowContext and
 * receives typed results. Every activity carries an idempotency key and a
 * bounded, error-classified retry policy, so a worker restart resumes
 * without duplicating side effects.
 */

import type { NexusWorkflowError } from "./errors.js";
import type { ActivityId, WorkflowId } from "./ids.js";
import type { RetryPolicy } from "./policies.js";
import type { ActivityKind, PrincipalType } from "./vocabulary.js";

export interface PrincipalRef {
  readonly id: string;
  readonly type: PrincipalType;
}

/** Compensation for a prior effect (SPEC-006 behavior 8). */
export interface CompensationStep {
  /** The COMPENSATE activity that rolls back the effect. */
  readonly activityId: ActivityId;
  /** Prefix for the compensation activity idempotency key. */
  readonly idempotencyKeyPrefix: string;
  /** Execution order; compensations run in reverse order. */
  readonly order: number;
}

export interface ActivityContract {
  readonly activityId: ActivityId;
  readonly kind: ActivityKind;
  /** SPEC-006 behavior 2: commands require idempotency keys. */
  readonly idempotencyRequired: boolean;
  readonly retry: RetryPolicy;
  readonly timeoutMs: number;
  /** Present when this effect has a registered rollback. */
  readonly compensation?: CompensationStep;
}

/** Immutable context handed to an activity by the workflow engine. */
export interface ActivityContext {
  readonly correlationId: string;
  readonly tenantId: string;
  readonly principal: PrincipalRef;
  readonly activityId: ActivityId;
  /** Canonical idempotency key; stable across retries and restarts. */
  readonly idempotencyKey: string;
  /** 1-based attempt number. */
  readonly attempt: number;
}

/**
 * Canonical activity idempotency key. The key is a pure function of the
 * workflow, the activity, and the logical attempt, so a retry or a worker
 * restart reuses the same key and the provider deduplicates the effect.
 */
export function idempotencyKeyFor(
  workflowId: WorkflowId,
  activityId: ActivityId,
  attempt: number,
): string {
  return `${workflowId}:${activityId}:${attempt}`;
}

export interface ActivitySuccess<T = unknown> {
  readonly ok: true;
  readonly activityId: ActivityId;
  readonly idempotencyKey: string;
  readonly value: T;
  /**
   * SPEC-006 behavior 5: external success is not accepted until the
   * verifier reads actual state or an authoritative receipt.
   */
  readonly verified: boolean;
  readonly receiptId?: string;
}

export interface ActivityFailure {
  readonly ok: false;
  readonly activityId: ActivityId;
  readonly idempotencyKey: string;
  readonly error: NexusWorkflowError;
}

export type WorkflowActivityResult<T = unknown> =
  | ActivitySuccess<T>
  | ActivityFailure;

export function activitySuccess<T>(
  activityId: ActivityId,
  idempotencyKey: string,
  value: T,
  verified: boolean,
  receiptId?: string,
): ActivitySuccess<T> {
  return receiptId === undefined
    ? { ok: true, activityId, idempotencyKey, value, verified }
    : { ok: true, activityId, idempotencyKey, value, verified, receiptId };
}

export function activityFailure(
  activityId: ActivityId,
  idempotencyKey: string,
  error: NexusWorkflowError,
): ActivityFailure {
  return { ok: false, activityId, idempotencyKey, error };
}
