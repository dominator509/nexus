/**
 * Pure mapping from the @nexus/workflows RetryPolicy contract to the
 * Temporal SDK RetryPolicy shape (SPEC-006 behavior 7: bounded, classified
 * retries). No I/O; fully deterministic and unit-testable.
 */

import { validateRetryPolicy } from "@nexus/workflows";
import type { RetryPolicy as NexusRetryPolicy } from "@nexus/workflows";
import type { RetryPolicy as TemporalRetryPolicy } from "@temporalio/common";

/**
 * Map a nexus retry contract onto Temporal retry options.
 * Durations are expressed in milliseconds (number) per the Temporal
 * Duration type. The nexus contract enforces bounded attempts; Temporal
 * receives an explicit maximumAttempts and never Infinity. Error-class
 * filtering is enforced in the activity layer via typed failures; an
 * empty retryableErrorClasses list means no retries at all.
 */
export function toTemporalRetry(policy: NexusRetryPolicy): TemporalRetryPolicy {
  validateRetryPolicy(policy);
  return {
    backoffCoefficient: policy.backoffCoefficient,
    initialInterval: policy.initialIntervalMs,
    maximumAttempts:
      policy.retryableErrorClasses.length === 0 ? 1 : policy.maxAttempts,
    maximumInterval: policy.maxIntervalMs,
  };
}
