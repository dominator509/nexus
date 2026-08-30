/**
 * Pure mapping from the @nexus/workflows RetryPolicy contract to the
 * Temporal SDK RetryPolicy shape (SPEC-006 behavior 7: bounded, classified
 * retries). No I/O; fully deterministic and unit-testable.
 *
 * Error-class filtering is enforced in BOTH directions: the activity
 * boundary rethrows `NexusWorkflowError` as an `ApplicationFailure` whose
 * `type` is the SPEC-006 code and whose `nonRetryable` flag matches the
 * code's class (see failure.ts), and the policy below also declares the
 * non-retryable SPEC-006 types explicitly so a permanent failure
 * (VALIDATION/POLICY/AUTH/...) can never consume the five attempts.
 */

import {
  ERROR_CODE_CLASS,
  NEXUS_ERROR_CODES,
  validateRetryPolicy,
} from "@nexus/workflows";
import type { RetryPolicy as NexusRetryPolicy } from "@nexus/workflows";
import type { RetryPolicy as TemporalRetryPolicy } from "@temporalio/common";

/**
 * Map a nexus retry contract onto Temporal retry options.
 * Durations are expressed in milliseconds (number) per the Temporal
 * Duration type. The nexus contract enforces bounded attempts; Temporal
 * receives an explicit maximumAttempts and never Infinity. `nonRetryableErrorTypes`
 * is derived from the policy: every SPEC-006 code whose retry class is
 * NOT in `retryableErrorClasses` is declared non-retryable.
 */
export function toTemporalRetry(policy: NexusRetryPolicy): TemporalRetryPolicy {
  validateRetryPolicy(policy);
  const retryableClasses = new Set<string>(policy.retryableErrorClasses);
  const nonRetryableErrorTypes = NEXUS_ERROR_CODES.filter(
    (code) => !retryableClasses.has(ERROR_CODE_CLASS[code]),
  );
  return {
    backoffCoefficient: policy.backoffCoefficient,
    initialInterval: policy.initialIntervalMs,
    maximumAttempts:
      policy.retryableErrorClasses.length === 0 ? 1 : policy.maxAttempts,
    maximumInterval: policy.maxIntervalMs,
    nonRetryableErrorTypes,
  };
}
