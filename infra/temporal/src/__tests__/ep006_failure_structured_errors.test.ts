/**
 * EP-006 M4: structured-errors + observability failure class (execplan M4
 * content 4).
 *
 * Boundary failures carry the typed NexusWorkflowError with the SPEC-006
 * code, a stable name, and correlation context - never a bare string.
 * Activities fail closed with a structured VALIDATION code when the
 * assertion does not bind.
 */

import { describe, expect, it } from "vitest";

import { NexusWorkflowError, workflowError } from "@nexus/workflows";
import type { VerifyApprovalInput } from "../activity-types.js";

import { verifyApproval } from "../activities.js";
import {
  actionDigestA,
  actionDigestB,
  actionIdA,
  makeApprovalSignal,
  signalIdA,
  workflowIdA,
} from "./helpers/fixtures.js";

const VALID_INPUT: VerifyApprovalInput = {
  workflowId: workflowIdA,
  actionId: actionIdA,
  actionDigest: actionDigestA,
  requiredStrength: "STEP_UP",
  signal: makeApprovalSignal({ signalId: signalIdA }),
};

describe("ep006_failure_structured_errors", () => {
  it("ep006_failure_structured_error_unbound_assertion_code", async () => {
    try {
      await verifyApproval({
        ...VALID_INPUT,
        signal: makeApprovalSignal({
          signalId: signalIdA,
          actionDigest: actionDigestB,
        }),
      });
      expect.unreachable("unbound assertion must fail closed");
    } catch (error) {
      expect(error).toBeInstanceOf(NexusWorkflowError);
      expect((error as NexusWorkflowError).name).toBe("NexusWorkflowError");
      expect((error as NexusWorkflowError).code).toBe("VALIDATION");
    }
  });

  it("ep006_failure_structured_error_invalid_workflow_id_code", async () => {
    try {
      await verifyApproval({ ...VALID_INPUT, workflowId: "nope" as never });
      expect.unreachable("invalid workflowId must fail closed");
    } catch (error) {
      expect(error).toBeInstanceOf(NexusWorkflowError);
      expect((error as NexusWorkflowError).code).toBe("VALIDATION");
    }
  });

  it("ep006_failure_structured_error_carries_correlation", () => {
    const error = workflowError("UNAVAILABLE", "dependency unreachable", {
      correlationId: "corr-obs-1",
      workflowId: workflowIdA,
    });
    expect(error.name).toBe("NexusWorkflowError");
    expect(error.code).toBe("UNAVAILABLE");
    expect(error.correlationId).toBe("corr-obs-1");
    expect(error.workflowId).toBe(workflowIdA);
  });

  it("ep006_failure_structured_error_codes_are_vocabulary_locked", () => {
    // Codes are stable machine values; a new code requires an ADR.
    for (const code of [
      "VALIDATION",
      "AUTHENTICATION",
      "AUTHORIZATION",
      "POLICY",
      "UNAVAILABLE",
      "TIMEOUT",
      "CONFLICT",
      "RATE_LIMIT",
      "EXTERNAL_PROVIDER",
      "VERIFICATION",
      "COMPENSATION",
      "INTERNAL_INVARIANT",
    ] as const) {
      const error = workflowError(code, `message for ${code}`);
      expect(error.code).toBe(code);
    }
  });
});
