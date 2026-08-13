/**
 * EP-006 M4: partial-side-effect + cancelled-work failure class on a
 * REAL Temporal server (execplan M4 content 1-3; SPEC-006 behavior 8).
 *
 * Real mechanism: a two-step objective workflow runs step 1's effect
 * (counted), then a cancel signal arrives before step 2. The workflow
 * must land on COMPENSATED and the applyCompensation activity must run
 * for the EXECUTED step 1 effect (reverse order rollback) - proven by
 * counting activities registered in the TESTING.md test zone. Step 2's
 * effect must never run.
 *
 * Uses an INDEPENDENT stack with the shared session's Runtime.
 */

import { describe, expect, it } from "vitest";

import {
  compensationKeyFor,
  TASK_QUEUES,
  WORKFLOW_TYPES,
  signalChannel,
} from "@nexus/temporal";
import type { ApplyCompensationInput } from "@nexus/temporal";
import {
  parseActionDigest,
  parseActivityId,
  parseObjectiveId,
  parseSignalId,
  parseWorkflowId,
  type ObjectiveInput,
} from "@nexus/workflows";
import { Client, Connection } from "@temporalio/client";
import { NativeConnection } from "@temporalio/worker";

import {
  actionIdA,
  AUTH_STEP_UP,
  makeApprovalSignal,
  PRINCIPAL_HUMAN,
} from "./helpers/fixtures.js";
import { getSession } from "./helpers/session.js";
import type { TestSession } from "./helpers/session.js";
import { startTemporalStack } from "./helpers/stack.js";
import { startWorker } from "./helpers/worker.js";

const WID_COMP = "0193a1f2-0000-7000-8000-000000000511";
const STEP_1 = "0193a1f2-0000-7000-8000-000000000521";
const STEP_2 = "0193a1f2-0000-7000-8000-000000000522";
const OBJ_ID = "0193a1f2-0000-7000-8000-000000000531";
const DIGEST_1 = "e".repeat(64);
const DIGEST_2 = "f".repeat(64);

function idempotencyKey(
  workflowId: ReturnType<typeof parseWorkflowId>,
  activityId: string,
): string {
  return `${workflowId}:${activityId}:1`;
}

describe("ep006_failure_stepgate_partial_compensation", () => {
  it("ep006_failure_stepgate_partial_compensation_real_server", async () => {
    const session = await getSession();
    const stack = await startTemporalStack();
    const connection = await NativeConnection.connect({
      address: stack.address,
    });
    const sdkConnection = await Connection.connect({
      address: stack.address,
    });
    const client = new Client({
      connection: sdkConnection,
      namespace: stack.namespace,
    });
    const ownSession = {
      stack,
      address: stack.address,
      namespace: stack.namespace,
      runtime: session.runtime,
      connection,
      client,
    } as unknown as TestSession;

    const workflowId = parseWorkflowId(WID_COMP);
    const effectCalls: string[] = [];
    const compensationCalls: string[] = [];
    const countingActivities = {
      runEffect: async (input: { idempotencyKey: string }) => {
        effectCalls.push(input.idempotencyKey);
        return { receiptId: `receipt-${input.idempotencyKey}` };
      },
      verifyEffect: async () => ({ verified: true }),
      applyCompensation: async (input: ApplyCompensationInput) => {
        compensationCalls.push(input.compensationKey);
        return {
          compensated: true as const,
          compensationKey: input.compensationKey,
        };
      },
    };

    const input: ObjectiveInput = {
      workflowId,
      tenantId: "tenant-m4",
      correlationId: "corr-partial-real",
      principal: PRINCIPAL_HUMAN,
      objectiveId: parseObjectiveId(OBJ_ID),
      title: "partial compensation objective",
      milestones: [
        {
          milestoneId: parseActivityId(STEP_1),
          title: "step one",
          actionId: actionIdA,
          actionDigest: parseActionDigest(DIGEST_1),
        },
        {
          milestoneId: parseActivityId(STEP_2),
          title: "step two",
          actionId: actionIdA,
          actionDigest: parseActionDigest(DIGEST_2),
        },
      ],
    };

    let worker: Awaited<ReturnType<typeof startWorker>> | undefined;
    try {
      worker = await startWorker(
        ownSession,
        [TASK_QUEUES.OBJECTIVE],
        countingActivities,
      );
      const handle = await client.workflow.start(WORKFLOW_TYPES.OBJECTIVE, {
        taskQueue: TASK_QUEUES.OBJECTIVE,
        workflowId,
        args: [input],
      });

      // Approve step 1: its effect runs and verifies (partial success).
      await handle.signal(
        signalChannel(WORKFLOW_TYPES.OBJECTIVE, "approval"),
        makeApprovalSignal({
          signalId: parseSignalId(STEP_1),
          workflowId,
          actionId: actionIdA,
          actionDigest: parseActionDigest(DIGEST_1),
          principal: { ...PRINCIPAL_HUMAN },
          authentication: { ...AUTH_STEP_UP },
        }),
      );

      // Wait until step 2 is awaiting approval (step 1 fully verified).
      await waitForStatus(client, handle, "AWAITING_APPROVAL");

      // Cancel before step 2: compensation must roll back step 1.
      await handle.signal(signalChannel(WORKFLOW_TYPES.OBJECTIVE, "cancel"), {
        signalId: workflowId,
        reason: "m4 partial compensation",
        requestedAt: "2026-08-13T00:05:00Z",
      });

      const result = await handle.result();
      expect(result.state).toBe("COMPENSATED");
      expect(result.outcome).toBe("COMPENSATED");

      // Exactly the executed step 1 effect ran - never step 2.
      expect(effectCalls).toEqual([idempotencyKey(workflowId, STEP_1)]);
      // The executed step 1 effect was compensated (reverse-order
      // rollback through the applyCompensation activity).
      expect(compensationCalls).toEqual([
        compensationKeyFor(idempotencyKey(workflowId, STEP_1)),
      ]);
    } finally {
      if (worker !== undefined) {
        await worker.shutdown();
      }
      await connection.close();
      await sdkConnection.close();
      await stack.dispose();
    }
  }, 120_000);
});

async function waitForStatus(
  client: Client,
  handle: Awaited<ReturnType<Client["workflow"]["start"]>>,
  expected: string,
  timeoutMs = 20_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const status = (await handle.query(
        `${WORKFLOW_TYPES.OBJECTIVE}.query.status`,
      )) as { state: string };
      if (status.state === expected) {
        return;
      }
    } catch {
      /* worker not up yet; retry */
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`workflow status did not reach ${expected} within timeout`);
}
