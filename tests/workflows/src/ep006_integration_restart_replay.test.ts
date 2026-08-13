/**
 * EP-006 M3: worker restart + delayed approval exactly-once, and real
 * history replay (EP-006 hard invariants 1 and 6; SPEC-023 behaviors 4
 * and 8).
 *
 * Worker restart test: a step-gated workflow runs its effect activity on
 * a REAL server, the worker is killed, a NEW worker resumes from the
 * recorded history, and the delayed approval completes the workflow. A
 * counting test activity (registered in the TESTING.md test zone) proves
 * each effect executes exactly once per idempotency key across the
 * restart - no duplicated side effects.
 *
 * Replay test: a workflow is run to completion against the real server,
 * its full history is fetched, and the SDK replays that recorded history
 * through the worker bundle. Any Date.now()/Math.random()/I/O in the
 * workflow code would raise DeterminismViolationError here.
 */

import { describe, expect, it } from "vitest";

import { TASK_QUEUES, WORKFLOW_TYPES, signalChannel } from "@nexus/temporal";
import type { ApprovalSignal } from "@nexus/workflows";
import {
  parseActionDigest,
  parseActivityId,
  parseObjectiveId,
  parseSignalId,
  parseWorkflowId,
  type ObjectiveInput,
} from "@nexus/workflows";

import {
  actionIdA,
  AUTH_STEP_UP,
  makeApprovalSignal,
  PRINCIPAL_HUMAN,
  signalIdD,
  signalIdE,
  workflowIdA,
} from "./helpers/fixtures.js";
import { getSession } from "./helpers/session.js";
import type { TestSession } from "./helpers/session.js";
import { startWorker, workflowBundle } from "./helpers/worker.js";

const WID_OBJ = "0193a1f2-0000-7000-8000-000000000101";
const WID_REPLAY = "0193a1f2-0000-7000-8000-000000000102";
const STEP_1 = "0193a1f2-0000-7000-8000-000000000201";
const STEP_2 = "0193a1f2-0000-7000-8000-000000000202";
const OBJ_ID = "0193a1f2-0000-7000-8000-000000000301";
const DIGEST_STEP_1 = "c".repeat(64);
const DIGEST_STEP_2 = "d".repeat(64);

describe("ep006_integration_worker_restart", () => {
  it("ep006_integration_worker_restart_delayed_approval_exactly_once", async () => {
    const session = await getSession();
    const workflowId = parseWorkflowId(WID_OBJ);

    // Shared counter: effect invocations keyed by idempotency key.
    const effectCalls: string[] = [];
    const countingActivities = {
      runEffect: async (input: { idempotencyKey: string }) => {
        effectCalls.push(input.idempotencyKey);
        return { receiptId: `receipt-${input.idempotencyKey}` };
      },
      verifyEffect: async () => ({ verified: true }),
    };

    const input: ObjectiveInput = {
      workflowId,
      tenantId: "tenant-1",
      correlationId: "corr-restart",
      principal: PRINCIPAL_HUMAN,
      objectiveId: parseObjectiveId(OBJ_ID),
      title: "restart objective",
      milestones: [
        {
          milestoneId: parseActivityId(STEP_1),
          title: "step one",
          actionId: actionIdA,
          actionDigest: parseActionDigest(DIGEST_STEP_1),
        },
        {
          milestoneId: parseActivityId(STEP_2),
          title: "step two",
          actionId: actionIdA,
          actionDigest: parseActionDigest(DIGEST_STEP_2),
        },
      ],
    };

    // Worker A: starts the workflow, approves step 1, then is killed.
    // Declared outside so a failure mid-body still shuts it down; a
    // leaked worker keeps its task-queue slots registered and would
    // poison every later worker in this process.
    let workerA: Awaited<ReturnType<typeof startWorker>> | undefined;
    let workerB: Awaited<ReturnType<typeof startWorker>> | undefined;
    try {
      workerA = await startWorker(
        session,
        [TASK_QUEUES.OBJECTIVE],
        countingActivities,
      );
      const handle = await session.client.workflow.start(
        WORKFLOW_TYPES.OBJECTIVE,
        {
          taskQueue: TASK_QUEUES.OBJECTIVE,
          workflowId,
          args: [input],
        },
      );
      await handle.signal(
        signalChannel(WORKFLOW_TYPES.OBJECTIVE, "approval"),
        stepApproval(workflowId, actionIdA, DIGEST_STEP_1, signalIdD),
      );

      // Wait until step 1 is FULLY verified (its effect ran AND the
      // verify activity completed) and step 2 is awaiting approval.
      // effectCalls alone is insufficient: the step-2 approval signal
      // would be ignored if it arrived while step 1 was still being
      // verified (applyStepGateSignal requires the CURRENT step to be
      // AWAITING_APPROVAL). Only the status query proves the gate has
      // advanced to step 2.
      await waitForStatus(session, handle, "AWAITING_APPROVAL");
      await workerA.shutdown();
      workerA = undefined;

      // Delayed approval for step 2 arrives while no worker is polling.
      await handle.signal(
        signalChannel(WORKFLOW_TYPES.OBJECTIVE, "approval"),
        stepApproval(workflowId, actionIdA, DIGEST_STEP_2, signalIdE),
      );

      // Worker B (restart) resumes from the recorded history.
      workerB = await startWorker(
        session,
        [TASK_QUEUES.OBJECTIVE],
        countingActivities,
      );
      const result = await handle.result();
      expect(result.state).toBe("SUCCEEDED");
      expect(result.outcome).toBe("SUCCEEDED");
      expect(result.output?.completedMilestones).toEqual([STEP_1, STEP_2]);

      // Exactly-once: step 1 effect ran once before the restart, and
      // replay after restart must NOT re-execute it. Step 2 ran once.
      expect(effectCalls).toEqual([
        idempotencyKey(workflowId, STEP_1),
        idempotencyKey(workflowId, STEP_2),
      ]);
    } finally {
      if (workerA !== undefined) {
        await workerA.shutdown();
      }
      if (workerB !== undefined) {
        await workerB.shutdown();
      }
    }
  }, 120_000);
});

describe("ep006_integration_replay", () => {
  it("ep006_integration_replay_recorded_history_succeeds", async () => {
    const session = await getSession();
    const workflowId = parseWorkflowId(WID_REPLAY);

    const workerA = await startWorker(session, [TASK_QUEUES.APPROVAL]);
    try {
      const handle = await session.client.workflow.start(
        WORKFLOW_TYPES.APPROVAL,
        {
          taskQueue: TASK_QUEUES.APPROVAL,
          workflowId,
          args: [
            {
              workflowId,
              tenantId: "tenant-1",
              correlationId: "corr-replay",
              principal: PRINCIPAL_HUMAN,
              actionId: actionIdA,
              actionDigest: parseActionDigest(DIGEST_STEP_1),
              requiredAuthenticationStrength: "STEP_UP",
              approvalTimeoutMs: 30_000,
            },
          ],
        },
      );
      await handle.signal(
        signalChannel(WORKFLOW_TYPES.APPROVAL, "approval"),
        makeApprovalSignal({
          workflowId,
          actionDigest: parseActionDigest(DIGEST_STEP_1),
        }),
      );
      const result = await handle.result();
      expect(result.state).toBe("APPROVED");

      // Fetch the recorded history and replay it through the bundle.
      // Success = the workflow code is deterministic and replay-safe
      // against its own recorded history.
      const history = await handle.fetchHistory();
      expect(history.events?.length ?? 0).toBeGreaterThan(0);

      const bundle = await workflowBundle();
      const { Worker } = await import("@temporalio/worker");
      await Worker.runReplayHistory(
        { workflowBundle: bundle },
        history,
        workflowId,
      );
      // No throw = replay succeeded against recorded history.
    } finally {
      await workerA.shutdown();
    }
  }, 120_000);
});

function stepApproval(
  workflowId: ReturnType<typeof parseWorkflowId>,
  actionId: typeof actionIdA,
  digest: string,
  signalId: ReturnType<typeof parseSignalId>,
): ApprovalSignal {
  return makeApprovalSignal({
    signalId,
    workflowId,
    actionId,
    actionDigest: parseActionDigest(digest),
    principal: { ...PRINCIPAL_HUMAN },
    authentication: { ...AUTH_STEP_UP },
  });
}

function queryChannelName(workflowType: string, kind: string): string {
  return `${workflowType}.query.${kind}`;
}

function idempotencyKey(
  workflowId: ReturnType<typeof parseWorkflowId>,
  activityId: string,
): string {
  return `${workflowId}:${activityId}:1`;
}

async function waitFor(
  predicate: () => boolean,
  timeoutMs = 15_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (predicate()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error("condition not met within timeout");
}

/**
 * Poll the workflow status query until the state matches. The query is
 * served from real workflow state on the running worker, so this proves
 * the gate advanced (e.g. step 2 is AWAITING_APPROVAL) rather than
 * racing side-effect counters.
 */
async function waitForStatus(
  session: TestSession,
  handle: Awaited<
    ReturnType<typeof session.client.workflow.start>
  >,
  expected: string,
  timeoutMs = 15_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const status = (await handle.query(
        queryChannelName(WORKFLOW_TYPES.OBJECTIVE, "status"),
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
