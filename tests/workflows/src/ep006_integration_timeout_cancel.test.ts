/**
 * EP-006 M3: explicit timeout and cancel paths on a REAL Temporal server
 * (EP-006 hard invariant 7; SPEC-023 behavior 5).
 *
 * The approval workflow carries an explicit per-workflow deadline
 * (approvalTimeoutMs) and an explicit cancel/compensation path. Both are
 * proven by observing real workflow terminal states.
 */

import { describe, expect, it } from "vitest";

import { TASK_QUEUES, WORKFLOW_TYPES, signalChannel } from "@nexus/temporal";
import type { ApprovalInput } from "@nexus/workflows";

import {
  actionDigestA,
  actionIdA,
  AUTH_STEP_UP,
  PRINCIPAL_HUMAN,
  workflowIdF,
  workflowIdG,
} from "./helpers/fixtures.js";
import { getSession } from "./helpers/session.js";
import { startWorker } from "./helpers/worker.js";
import type { WorkflowId } from "@nexus/workflows";

function approvalInput(
  workflowId: WorkflowId,
  approvalTimeoutMs: number,
): ApprovalInput {
  return {
    workflowId,
    tenantId: "tenant-1",
    correlationId: "corr-timeout",
    principal: PRINCIPAL_HUMAN,
    actionId: actionIdA,
    actionDigest: actionDigestA,
    requiredAuthenticationStrength: "STEP_UP",
    approvalTimeoutMs,
  };
}

describe("ep006_integration_timeout", () => {
  it("ep006_integration_approval_timeout_explicit_terminal", async () => {
    const session = await getSession();
    const started = await startWorker(session, [TASK_QUEUES.APPROVAL]);
    try {
      const handle = await session.client.workflow.start(
        WORKFLOW_TYPES.APPROVAL,
        {
          taskQueue: TASK_QUEUES.APPROVAL,
          workflowId: workflowIdF,
          args: [approvalInput(workflowIdF, 3000)],
        },
      );

      // No approval signal is sent: the real timer must fire and the
      // workflow must land on the explicit TIMED_OUT outcome.
      const result = await handle.result();
      expect(result.state).toBe("TIMED_OUT");
      expect(result.outcome).toBe("TIMED_OUT");
    } finally {
      await started.shutdown();
    }
  }, 90_000);
});

describe("ep006_integration_cancel", () => {
  it("ep006_integration_cancel_signal_compensates", async () => {
    const session = await getSession();
    const started = await startWorker(session, [TASK_QUEUES.APPROVAL]);
    try {
      const handle = await session.client.workflow.start(
        WORKFLOW_TYPES.APPROVAL,
        {
          taskQueue: TASK_QUEUES.APPROVAL,
          workflowId: workflowIdG,
          args: [approvalInput(workflowIdG, 30_000)],
        },
      );

      // Cancel before the deadline: explicit cancel path runs
      // compensation (policy cancelAction = COMPENSATE) and lands on
      // the COMPENSATED terminal state.
      await handle.signal(signalChannel(WORKFLOW_TYPES.APPROVAL, "cancel"), {
        signalId: workflowIdG,
        reason: "operator cancelled",
        requestedAt: "2026-08-13T00:01:00Z",
      });

      const result = await handle.result();
      expect(result.state).toBe("COMPENSATED");
      expect(result.outcome).toBe("COMPENSATED");
    } finally {
      await started.shutdown();
    }
  }, 90_000);
});
