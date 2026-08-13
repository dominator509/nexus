/**
 * Workflow timeout, cancellation, and retry policies (EP-006 acceptance
 * obligation 3; SPEC-006 behavior 7; SPEC-023 behavior 5).
 *
 * Timeout and cancel paths are explicit contracts, not implementation
 * accidents: every workflow declares its execution/run/task timeouts, the
 * human-approval wait bound, and what cancellation does (CANCEL fails
 * closed, COMPENSATE runs registered compensation in reverse order).
 */

import { WorkflowContractError } from "./errors.js";
import { retryErrorClass, cancelAction } from "./vocabulary.js";
import type { RetryErrorClass, CancelAction } from "./vocabulary.js";

/** Bounded, classified retry policy (SPEC-006 behavior 7). */
export interface RetryPolicy {
  /** Maximum attempts, inclusive. Must be >= 1. Never unbounded. */
  readonly maxAttempts: number;
  /** Initial backoff interval in milliseconds. Must be > 0. */
  readonly initialIntervalMs: number;
  /** Backoff multiplier. Must be >= 1. */
  readonly backoffCoefficient: number;
  /** Backoff ceiling in milliseconds. Must be >= initialIntervalMs. */
  readonly maxIntervalMs: number;
  /** Error classes eligible for retry. PERMANENT is never retried. */
  readonly retryableErrorClasses: readonly RetryErrorClass[];
}

export interface TimeoutPolicy {
  /** Whole-workflow bound in milliseconds. */
  readonly executionTimeoutMs: number;
  /** Single-run bound in milliseconds. */
  readonly runTimeoutMs: number;
  /** Per-task bound in milliseconds. */
  readonly taskTimeoutMs: number;
  /** Bound on a human approval wait. Required for approval workflows. */
  readonly approvalTimeoutMs?: number;
}

export interface WorkflowPolicy {
  readonly timeouts: TimeoutPolicy;
  /** What cancellation does. CANCEL fails closed; COMPENSATE rolls back. */
  readonly cancelAction: CancelAction;
  readonly defaultActivityRetry: RetryPolicy;
}

export const DEFAULT_RETRY_POLICY: RetryPolicy = {
  maxAttempts: 5,
  initialIntervalMs: 1_000,
  backoffCoefficient: 2,
  maxIntervalMs: 60_000,
  retryableErrorClasses: ["TRANSIENT", "RATE_LIMIT", "UNAVAILABLE", "TIMEOUT"],
};

/** Validate a retry policy. Throws WorkflowContractError on violation. */
export function validateRetryPolicy(policy: RetryPolicy): void {
  if (!Number.isInteger(policy.maxAttempts) || policy.maxAttempts < 1) {
    throw new WorkflowContractError(
      `retry maxAttempts must be an integer >= 1, got ${policy.maxAttempts}`,
    );
  }
  if (
    !Number.isFinite(policy.initialIntervalMs) ||
    policy.initialIntervalMs <= 0
  ) {
    throw new WorkflowContractError(
      `retry initialIntervalMs must be > 0, got ${policy.initialIntervalMs}`,
    );
  }
  if (policy.backoffCoefficient < 1) {
    throw new WorkflowContractError(
      `retry backoffCoefficient must be >= 1, got ${policy.backoffCoefficient}`,
    );
  }
  if (policy.maxIntervalMs < policy.initialIntervalMs) {
    throw new WorkflowContractError(
      `retry maxIntervalMs (${policy.maxIntervalMs}) must be >= initialIntervalMs (${policy.initialIntervalMs})`,
    );
  }
  for (const cls of policy.retryableErrorClasses) {
    retryErrorClass.parse(cls, "retryableErrorClasses");
    if (cls === "PERMANENT") {
      throw new WorkflowContractError(
        "PERMANENT errors must never be listed as retryable",
      );
    }
  }
}

/** Validate a timeout policy. Throws WorkflowContractError on violation. */
export function validateTimeoutPolicy(timeouts: TimeoutPolicy): void {
  if (
    !Number.isFinite(timeouts.executionTimeoutMs) ||
    timeouts.executionTimeoutMs <= 0
  ) {
    throw new WorkflowContractError(
      `executionTimeoutMs must be > 0, got ${timeouts.executionTimeoutMs}`,
    );
  }
  if (
    !Number.isFinite(timeouts.runTimeoutMs) ||
    timeouts.runTimeoutMs <= 0 ||
    timeouts.runTimeoutMs > timeouts.executionTimeoutMs
  ) {
    throw new WorkflowContractError(
      `runTimeoutMs must be > 0 and <= executionTimeoutMs (${timeouts.executionTimeoutMs}), got ${timeouts.runTimeoutMs}`,
    );
  }
  if (
    !Number.isFinite(timeouts.taskTimeoutMs) ||
    timeouts.taskTimeoutMs <= 0 ||
    timeouts.taskTimeoutMs > timeouts.runTimeoutMs
  ) {
    throw new WorkflowContractError(
      `taskTimeoutMs must be > 0 and <= runTimeoutMs (${timeouts.runTimeoutMs}), got ${timeouts.taskTimeoutMs}`,
    );
  }
  if (
    timeouts.approvalTimeoutMs !== undefined &&
    (!Number.isFinite(timeouts.approvalTimeoutMs) ||
      timeouts.approvalTimeoutMs <= 0)
  ) {
    throw new WorkflowContractError(
      `approvalTimeoutMs must be > 0 when present, got ${timeouts.approvalTimeoutMs}`,
    );
  }
}

/** Validate a full workflow policy. Throws WorkflowContractError. */
export function validateWorkflowPolicy(policy: WorkflowPolicy): void {
  validateTimeoutPolicy(policy.timeouts);
  cancelAction.parse(policy.cancelAction, "cancelAction");
  validateRetryPolicy(policy.defaultActivityRetry);
}
