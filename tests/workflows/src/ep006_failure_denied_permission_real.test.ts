/**
 * EP-006 M4: denied-permission failure class on a REAL Temporal server
 * (execplan M4 content 1-2; SPEC-005/SPEC-006 approval binding).
 *
 * Real mechanism: an approval signal whose authentication strength is
 * BELOW the required strength is quarantined - the workflow stays
 * REQUESTED and never advances. A REJECT decision with a valid binding
 * lands on the REJECTED terminal outcome. Both are observed through
 * real signal delivery, queries, and the terminal result.
 */

import { describe, expect, it } from "vitest";

import { TASK_QUEUES, WORKFLOW_TYPES, signalChannel } from "@nexus/temporal";
import type { ApprovalInput } from "@nexus/workflows";
import {
  parseSignalId,
  parseWorkflowId,
  type WorkflowId,
} from "@nexus/workflows";

import {
  actionDigestA,
  actionIdA,
  makeApprovalSignal,
  PRINCIPAL_HUMAN,
  workflowIdH,
  workflowIdI,
} from "./helpers/fixtures.js";
import { getSession } from "./helpers/session.js";
import { startWorker } from "./helpers/worker.js";

function approvalInput(
  workflowId: WorkflowId,
  requiredStrength: "STEP_UP",
): ApprovalInput {
  return {
    workflowId,
    tenantId: "tenant-m4",
    correlationId: "corr-denied-real",
    principal: PRINCIPAL_HUMAN,
    actionId: actionIdA,
    actionDigest: actionDigestA,
    requiredAuthenticationStrength: requiredStrength,
    approvalTimeoutMs: 30_000,
  };
}

function queryChannelName(workflowType: string, kind: string): string {
  return `${workflowType}.query.${kind}`;
}

describe("ep006_failure_denied_permission_real", () => {
  it("ep006_failure_denied_permission_weak_strength_quarantined", async () => {
    const session = await getSession();
    const started = await startWorker(session, [TASK_QUEUES.APPROVAL]);
    try {
      const handle = await session.client.workflow.start(
        WORKFLOW_TYPES.APPROVAL,
        {
          taskQueue: TASK_QUEUES.APPROVAL,
          workflowId: workflowIdH,
          args: [approvalInput(workflowIdH, "STEP_UP")],
        },
      );

      // Approval with SINGLE_FACTOR against a STEP_UP requirement: the
      // binding fails and the gate must NOT advance.
      await handle.signal(
        signalChannel(WORKFLOW_TYPES.APPROVAL, "approval"),
        makeApprovalSignal({
          signalId: parseSignalId(workflowIdH),
          workflowId: workflowIdH,
          authentication: {
            strength: "SINGLE_FACTOR",
            method: "password",
            sessionId: "sess-weak-real",
            verifiedAt: "2026-08-13T00:06:00Z",
          },
        }),
      );

      await new Promise((resolve) => setTimeout(resolve, 2500));
      const status = (await handle.query(
        queryChannelName(WORKFLOW_TYPES.APPROVAL, "status"),
      )) as { state: string };
      expect(status.state).toBe("REQUESTED");
      const pending = (await handle.query(
        queryChannelName(WORKFLOW_TYPES.APPROVAL, "pending"),
      )) as { approvals: readonly unknown[] };
      // The weak signal was observed (quarantined) but never advanced.
      expect(pending.approvals.length).toBe(1);

      // REJECT with a VALID binding: terminal REJECTED.
      await handle.signal(
        signalChannel(WORKFLOW_TYPES.APPROVAL, "approval"),
        makeApprovalSignal({
          signalId: parseSignalId(workflowIdI),
          workflowId: workflowIdH,
          decision: "REJECT",
        }),
      );
      const result = await handle.result();
      expect(result.state).toBe("REJECTED");
      expect(result.outcome).toBe("REJECTED");
    } finally {
      await started.shutdown();
    }
  }, 90_000);

  it("ep006_failure_denied_permission_reject_terminal_real_server", async () => {
    const session = await getSession();
    const started = await startWorker(session, [TASK_QUEUES.APPROVAL]);
    try {
      const handle = await session.client.workflow.start(
        WORKFLOW_TYPES.APPROVAL,
        {
          taskQueue: TASK_QUEUES.APPROVAL,
          workflowId: workflowIdI,
          args: [approvalInput(workflowIdI, "STEP_UP")],
        },
      );

      // REJECT decision with the correct binding: REJECTED terminal.
      await handle.signal(
        signalChannel(WORKFLOW_TYPES.APPROVAL, "approval"),
        makeApprovalSignal({
          signalId: parseSignalId(workflowIdI),
          workflowId: workflowIdI,
          decision: "REJECT",
        }),
      );
      const result = await handle.result();
      expect(result.state).toBe("REJECTED");
      expect(result.outcome).toBe("REJECTED");
    } finally {
      await started.shutdown();
    }
  }, 90_000);
});
