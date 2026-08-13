import { describe, expect, it } from "vitest";

import {
  workflowError,
  NexusWorkflowError,
  WorkflowContractError,
} from "../errors.js";
import { validateQuery, validateQueryResponse } from "../queries.js";
import { validateSignal } from "../signals.js";
import {
  makeApprovalSignal,
  signalIdA,
  workflowIdA,
  actionIdA,
} from "./helpers/fixtures.js";

describe("ep006_unit_serialization", () => {
  it("ep006_unit_serialization_signal_json_roundtrip", () => {
    const signal = makeApprovalSignal();
    const wire = JSON.stringify(signal);
    const parsed = validateSignal(JSON.parse(wire));
    expect(parsed).toEqual(signal);
  });

  it("ep006_unit_serialization_query_json_roundtrip", () => {
    const query = { queryType: "WORKFLOW_STATUS", workflowId: workflowIdA };
    const parsed = validateQuery(JSON.parse(JSON.stringify(query)));
    expect(parsed).toEqual(query);
  });

  it("ep006_unit_serialization_query_response_roundtrip", () => {
    const response = {
      queryType: "WORKFLOW_STATUS",
      workflowId: workflowIdA,
      state: "AWAITING_APPROVAL",
      updatedAt: "2026-08-13T00:00:00Z",
    };
    const parsed = validateQueryResponse(JSON.parse(JSON.stringify(response)));
    expect(parsed).toEqual(response);
  });

  it("ep006_unit_serialization_pending_approval_response_roundtrip", () => {
    const response = {
      queryType: "PENDING_APPROVAL",
      workflowId: workflowIdA,
      approvals: [makeApprovalSignal()],
    };
    const parsed = validateQueryResponse(JSON.parse(JSON.stringify(response)));
    expect(parsed).toEqual(response);
  });

  it("ep006_unit_serialization_rejects_unknown_signal_type", () => {
    const raw = { ...makeApprovalSignal() } as Record<string, unknown>;
    raw.signalType = "TELEPORT";
    expect(() => validateSignal(raw)).toThrow(WorkflowContractError);
  });

  it("ep006_unit_serialization_rejects_unknown_query_type", () => {
    expect(() =>
      validateQuery({ queryType: "MIND_READ", workflowId: workflowIdA }),
    ).toThrow(WorkflowContractError);
  });

  it("ep006_unit_serialization_error_problem_details", () => {
    const error = workflowError("TIMEOUT", "approval wait exceeded", {
      correlationId: "corr-1",
      workflowId: workflowIdA,
    });
    expect(error).toBeInstanceOf(NexusWorkflowError);
    expect(error.isRetryable()).toBe(true);
    const details = error.toProblemDetails();
    expect(details.code).toBe("TIMEOUT");
    expect(details.correlation_id).toBe("corr-1");
    expect(details.workflow_id).toBe(workflowIdA);
    expect(details.type).toBe("urn:nexus:error:timeout");
  });

  it("ep006_unit_serialization_validation_error_not_retryable", () => {
    const error = new WorkflowContractError("bad vocabulary value");
    expect(error.code).toBe("VALIDATION");
    expect(error.isRetryable()).toBe(false);
  });

  it("ep006_unit_serialization_retryable_classification", () => {
    expect(workflowError("UNAVAILABLE", "x").isRetryable()).toBe(true);
    expect(workflowError("RATE_LIMIT", "x").isRetryable()).toBe(true);
    expect(workflowError("CONFLICT", "x").isRetryable()).toBe(true);
    expect(workflowError("AUTHORIZATION", "x").isRetryable()).toBe(false);
    expect(workflowError("INTERNAL_INVARIANT", "x").isRetryable()).toBe(false);
  });
});
