/**
 * EP-006 M3: approval signal digest binding and duplicate-signal
 * idempotency on a REAL Temporal server (EP-006 hard invariants 4 and 5;
 * SPEC-023 behaviors 4 and 7).
 *
 * Wrong-digest signals must NOT advance the approval; duplicate signals
 * (same signalId) must be no-ops. Everything asserted here is the actual
 * workflow state observed through real signal delivery, queries, and the
 * terminal workflow result.
 */

import { describe, expect, it } from "vitest";

import { TASK_QUEUES, WORKFLOW_TYPES, signalChannel } from "@nexus/temporal";
import type { ApprovalInput, ApprovalSignal } from "@nexus/workflows";
import {
  parseActionDigest,
  parseWorkflowId,
  type ActionDigest,
  type WorkflowId,
} from "@nexus/workflows";

import {
  actionDigestA,
  actionDigestB,
  actionIdA,
  AUTH_STEP_UP,
  makeApprovalSignal,
  PRINCIPAL_HUMAN,
  signalIdD,
  signalIdE,
  workflowIdD,
  workflowIdE,
} from "./helpers/fixtures.js";
import { getSession } from "./helpers/session.js";
import { startWorker } from "./helpers/worker.js";

function approvalInput(
  workflowId: WorkflowId,
  digest: ActionDigest,
): ApprovalInput {
  return {
    workflowId,
    tenantId: "tenant-1",
    correlationId: "corr-digest",
    principal: PRINCIPAL_HUMAN,
    actionId: actionIdA,
    actionDigest: digest,
    requiredAuthenticationStrength: "STEP_UP",
    approvalTimeoutMs: 30_000,
  };
}

describe("ep006_integration_digest_binding", () => {
  it("ep006_integration_wrong_digest_does_not_advance_approval", async () => {
    const session = await getSession();
    const started = await startWorker(session, [TASK_QUEUES.APPROVAL]);
    try {
      const handle = await session.client.workflow.start(
        WORKFLOW_TYPES.APPROVAL,
        {
          taskQueue: TASK_QUEUES.APPROVAL,
          workflowId: workflowIdD,
          args: [approvalInput(workflowIdD, actionDigestA)],
        },
      );

      // Signal bound to a DIFFERENT action digest: must not approve.
      // Each logical approval needs a UNIQUE signalId - signalKey is
      // (workflowId, type, signalId), so a later signal reusing this
      // signalId would be deduplicated (correct contract behavior).
      const wrongDigest: ApprovalSignal = makeApprovalSignal({
        signalId: signalIdD,
        actionDigest: actionDigestB,
      });
      await handle.signal(
        signalChannel(WORKFLOW_TYPES.APPROVAL, "approval"),
        wrongDigest,
      );

      // Give the real engine a moment to deliver; then the state must
      // still be the initial REQUESTED (the mismatched assertion is
      // quarantined: the signal is recorded as observed but the gate
      // does not advance).
      await new Promise((resolve) => setTimeout(resolve, 2500));
      const status = (await handle.query(
        queryChannelName(WORKFLOW_TYPES.APPROVAL, "status"),
      )) as { state: string };
      expect(status.state).toBe("REQUESTED");
      // The pending query records the observed (invalid) signal but the
      // approval gate must not have advanced.
      const pending = (await handle.query(
        queryChannelName(WORKFLOW_TYPES.APPROVAL, "pending"),
      )) as { approvals: readonly ApprovalSignal[] };
      expect(pending.approvals.length).toBe(1);

      // Now the CORRECT digest with a fresh signalId: approval completes.
      await handle.signal(
        signalChannel(WORKFLOW_TYPES.APPROVAL, "approval"),
        makeApprovalSignal({
          signalId: signalIdE,
          actionDigest: actionDigestA,
        }),
      );
      const result = await handle.result();
      expect(result.state).toBe("APPROVED");
      // WorkflowOutcome vocabulary: SUCCEEDED (vocabulary.ts).
      expect(result.outcome).toBe("SUCCEEDED");
    } finally {
      await started.shutdown();
    }
  }, 90_000);
});

describe("ep006_integration_signal_idempotency", () => {
  it("ep006_integration_duplicate_signal_is_noop", async () => {
    const session = await getSession();
    const started = await startWorker(session, [TASK_QUEUES.APPROVAL]);
    try {
      const handle = await session.client.workflow.start(
        WORKFLOW_TYPES.APPROVAL,
        {
          taskQueue: TASK_QUEUES.APPROVAL,
          workflowId: workflowIdE,
          args: [approvalInput(workflowIdE, actionDigestA)],
        },
      );

      const approval: ApprovalSignal = makeApprovalSignal({
        signalId: signalIdE,
        workflowId: workflowIdE,
      });
      // Deliver the SAME signal twice (a redelivery / duplicate).
      await handle.signal(
        signalChannel(WORKFLOW_TYPES.APPROVAL, "approval"),
        approval,
      );
      await handle.signal(
        signalChannel(WORKFLOW_TYPES.APPROVAL, "approval"),
        approval,
      );

      const result = await handle.result();
      expect(result.state).toBe("APPROVED");
      // WorkflowOutcome vocabulary: SUCCEEDED (vocabulary.ts).
      expect(result.outcome).toBe("SUCCEEDED");
      // The pending query must show exactly ONE observed signal: the
      // duplicate was deduplicated by signalKey.
      const pending = (await handle.query(
        queryChannelName(WORKFLOW_TYPES.APPROVAL, "pending"),
      )) as { approvals: readonly ApprovalSignal[] };
      expect(pending.approvals.length).toBe(1);
    } finally {
      await started.shutdown();
    }
  }, 90_000);
});

function queryChannelName(workflowType: string, kind: string): string {
  return `${workflowType}.query.${kind}`;
}

void parseActionDigest;
void parseWorkflowId;
