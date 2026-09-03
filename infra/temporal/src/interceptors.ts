/**
 * Activity-boundary failure classification (SPEC-006 behavior 7).
 *
 * Activities throw `NexusWorkflowError` (typed SPEC-006 failures). The
 * Temporal worker would otherwise wrap them as a generic ApplicationFailure
 * whose type is the class name, losing the retry classification. This
 * inbound interceptor rethrows them as classified `ApplicationFailure`s
 * (type = SPEC-006 code, nonRetryable per the code's class) at the single
 * activity boundary, covering core and provider-registered activities
 * alike.
 */

import { NexusWorkflowError } from "@nexus/workflows";
import type {
  ActivityExecuteInput,
  ActivityInboundCallsInterceptor,
  Next,
} from "@temporalio/worker";

import { toApplicationFailure } from "./failure.js";

/**
 * Rethrow `NexusWorkflowError` as a classified `ApplicationFailure`; all
 * other errors pass through unchanged.
 */
export class NexusFailureInterceptor implements ActivityInboundCallsInterceptor {
  async execute(
    input: ActivityExecuteInput,
    next: Next<ActivityInboundCallsInterceptor, "execute">,
  ): Promise<unknown> {
    try {
      return await next(input);
    } catch (err) {
      if (err instanceof NexusWorkflowError) {
        throw toApplicationFailure(err);
      }
      throw err;
    }
  }
}
