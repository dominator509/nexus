import { describe, expect, it } from "vitest";

import { NexusWorkflowError } from "@nexus/workflows";

import { applyCompensation, verifyApproval } from "../activities.js";
import {
  actionDigestA,
  actionDigestB,
  makeApprovalSignal,
  workflowIdA,
  workflowIdB,
} from "./helpers/fixtures.js";

describe("ep006_unit_activities", () => {
  it("ep006_unit_activities_verify_approval_passes_binding", async () => {
    const signal = makeApprovalSignal();
    const output = await verifyApproval({
      workflowId: workflowIdA,
      actionId: signal.actionId,
      actionDigest: actionDigestA,
      signal,
      requiredStrength: "STEP_UP",
    });
    expect(output.digestMatch).toBe(true);
    expect(output.strengthOk).toBe(true);
    expect(output.verifiedAt).toBe(signal.decidedAt);
  });

  it("ep006_unit_activities_verify_approval_rejects_bad_binding", async () => {
    const signal = makeApprovalSignal({ actionDigest: actionDigestB });
    await expect(
      verifyApproval({
        workflowId: workflowIdA,
        actionId: signal.actionId,
        actionDigest: actionDigestA,
        signal,
        requiredStrength: "STEP_UP",
      }),
    ).rejects.toThrow(NexusWorkflowError);
  });

  it("ep006_unit_activities_verify_approval_rejects_invalid_workflow_id", async () => {
    const signal = makeApprovalSignal();
    await expect(
      verifyApproval({
        workflowId: "not-a-uuid" as typeof workflowIdA,
        actionId: signal.actionId,
        actionDigest: actionDigestA,
        signal,
        requiredStrength: "STEP_UP",
      }),
    ).rejects.toThrow(/workflowId/);
  });

  it("ep006_unit_activities_apply_compensation_valid", async () => {
    const effectKey = `${workflowIdA}:approval-wait`;
    const output = await applyCompensation({
      workflowId: workflowIdA,
      effectIdempotencyKey: effectKey,
      compensationKey: `comp:${effectKey}`,
      reason: "cancelled",
    });
    expect(output.compensated).toBe(true);
    expect(output.compensationKey).toBe(`comp:${effectKey}`);
  });

  it("ep006_unit_activities_apply_compensation_rejects_bad_key", async () => {
    const effectKey = `${workflowIdA}:approval-wait`;
    await expect(
      applyCompensation({
        workflowId: workflowIdA,
        effectIdempotencyKey: effectKey,
        compensationKey: "comp:WRONG",
        reason: "cancelled",
      }),
    ).rejects.toThrow(NexusWorkflowError);
  });

  it("ep006_unit_activities_apply_compensation_rejects_empty_effect_key", async () => {
    await expect(
      applyCompensation({
        workflowId: workflowIdA,
        effectIdempotencyKey: "",
        compensationKey: "comp:",
        reason: "cancelled",
      }),
    ).rejects.toThrow(/effectIdempotencyKey/);
  });

  it("ep006_unit_activities_apply_compensation_rejects_invalid_workflow_id", async () => {
    await expect(
      applyCompensation({
        workflowId: "not-a-uuid" as typeof workflowIdA,
        effectIdempotencyKey: "x",
        compensationKey: "comp:x",
        reason: "cancelled",
      }),
    ).rejects.toThrow(/workflowId/);
  });
});
