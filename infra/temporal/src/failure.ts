/**
 * Classify a NexusWorkflowError for the Temporal boundary (SPEC-006
 * behavior 7).
 *
 * The failure `type` becomes the SPEC-006 machine code and `nonRetryable`
 * follows the code's intrinsic class, so Temporal's retry engine can
 * distinguish permanent from transient failures without inspecting
 * messages. A VALIDATION/POLICY/AUTH failure is never retried five times.
 */

import { NexusWorkflowError } from "@nexus/workflows";
import { ApplicationFailure } from "@temporalio/common";

/**
 * Convert a typed nexus failure into an `ApplicationFailure` carrying the
 * SPEC-006 code as its type. Permanent codes are marked non-retryable so
 * both the failure itself and the policy's `nonRetryableErrorTypes` agree.
 */
export function toApplicationFailure(
  err: NexusWorkflowError,
): ApplicationFailure {
  return ApplicationFailure.fromError(err, {
    type: err.code,
    nonRetryable: !err.isRetryable(),
  });
}
