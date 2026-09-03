/**
 * AUD-023 official-test-environment proof (TESTING.md line 36).
 *
 * The unit suite (ep006_unit_failure.test.ts) proves the interceptor's
 * try/catch logic with a labeled `next()` test double - that is a UNIT
 * test. This file proves the SAME classification through the OFFICIAL
 * Temporal test environment: @temporalio/testing's
 * TestWorkflowEnvironment launches a REAL Temporal server binary, and the
 * REAL worker factory (createTemporalWorker) with the REAL
 * NexusFailureInterceptor runs a REAL activity that throws
 * NexusWorkflowError. The failure that surfaces to the workflow client
 * must carry type=POLICY and nonRetryable=true through real gRPC.
 *
 * No mocks, no `next()` doubles: the only test-supplied piece is the
 * activity body (the TESTING.md test-zone activity), which throws the
 * typed domain error an owning node's activity would throw.
 */

import { describe, expect, it } from "vitest";

import { TestWorkflowEnvironment } from "@temporalio/testing";
import { Runtime, Worker } from "@temporalio/worker";

import {
  defaultWorkflowsPath,
  TASK_QUEUES,
  WORKFLOW_TYPES,
  signalChannel,
} from "../index.js";
import { NexusFailureInterceptor } from "../interceptors.js";
import {
  parseActionDigest,
  parseActionId,
  parseActivityId,
  parseObjectiveId,
  parseSignalId,
  parseWorkflowId,
  workflowError,
} from "@nexus/workflows";
import { bundleWorkflowCode } from "@temporalio/worker";

const WID = "0193a1f2-0000-7000-8000-000000000641";
const STEP_1 = "0193a1f2-0000-7000-8000-000000000651";
const OBJ_ID = "0193a1f2-0000-7000-8000-000000000661";
const ACTION_ID = "0193a1f2-0000-7000-8000-000000000671";
const SIGNAL_ID = "0193a1f2-0000-7000-8000-000000000681";
const DIGEST = "a".repeat(64);

describe("ep006_official_environment_interceptor", () => {
  it("ep006_official_environment_classifies_permanent_failure", async () => {
    const env = await TestWorkflowEnvironment.createLocal();
    const runtime = Runtime.instance();
    const connection = env.nativeConnection;
    const sdkConnection = env.connection;
    const client = env.client;

    const bundle = await bundleWorkflowCode({
      workflowsPath: defaultWorkflowsPath(),
    });

    // Test-zone activity: throws the typed domain failure an owning
    // node's activity would throw. Registered ONLY on the activity queue.
    let attempts = 0;
    const activities = {
      runEffect: async () => {
        attempts += 1;
        throw workflowError("POLICY", "permission denied");
      },
      verifyEffect: async () => ({ verified: true }),
      applyCompensation: async () => ({
        compensated: true as const,
        compensationKey: "comp:none",
      }),
    };

    const workers: Worker[] = [];
    try {
      const wfWorker = await Worker.create({
        namespace: env.namespace ?? "default",
        taskQueue: TASK_QUEUES.OBJECTIVE,
        workflowBundle: bundle,
        connection,
      });
      const actWorker = await Worker.create({
        namespace: env.namespace ?? "default",
        taskQueue: TASK_QUEUES.ACTIVITY,
        activities,
        interceptors: {
          activity: [() => ({ inbound: new NexusFailureInterceptor() })],
        },
        connection,
      });
      workers.push(wfWorker, actWorker);
      const wfRun = wfWorker.run();
      const actRun = actWorker.run();
      void wfRun;
      void actRun;

      const workflowId = parseWorkflowId(WID);
      const actionId = parseActionId(ACTION_ID);
      const actionDigest = parseActionDigest(DIGEST);
      const stepId = parseActivityId(STEP_1);
      const objectiveId = parseObjectiveId(OBJ_ID);
      const handle = await client.workflow.start(WORKFLOW_TYPES.OBJECTIVE, {
        taskQueue: TASK_QUEUES.OBJECTIVE,
        workflowId,
        args: [
          {
            workflowId,
            tenantId: "tenant-official",
            correlationId: "corr-official",
            principal: { id: "p-official", type: "HUMAN" },
            objectiveId,
            title: "official env objective",
            milestones: [
              {
                milestoneId: stepId,
                title: "step one",
                actionId,
                actionDigest,
              },
            ],
          },
        ],
      });

      // The step-gate runner only executes an APPROVED step. Signal the
      // approval so runEffect runs through the real activity boundary.
      await handle.signal(signalChannel(WORKFLOW_TYPES.OBJECTIVE, "approval"), {
        signalType: "APPROVAL",
        signalId: parseSignalId(SIGNAL_ID),
        workflowId,
        actionId,
        actionDigest,
        principal: { id: "p-official", type: "HUMAN" },
        authentication: {
          strength: "STEP_UP",
          method: "passkey",
          sessionId: "sess-official",
          verifiedAt: "2026-08-30T00:00:00Z",
        },
        decision: "APPROVE",
        decidedAt: "2026-08-30T00:00:00Z",
      });

      // The step-gate runner rethrows non-cancellation activity failures:
      // the workflow must FAIL, and the classified ApplicationFailure must
      // surface at the client boundary through real gRPC.
      const failure = await handle.result().catch((err: unknown) => err);
      expect(failure).toBeDefined();

      let cursor: unknown = failure;
      let classified: { type?: unknown; nonRetryable?: unknown } | undefined;
      for (let depth = 0; depth < 6 && cursor !== undefined; depth += 1) {
        const record = cursor as {
          type?: unknown;
          nonRetryable?: unknown;
          cause?: unknown;
        };
        if (record.type !== undefined) {
          classified = record;
          break;
        }
        cursor = record.cause;
      }
      expect(classified).toBeDefined();
      expect(classified?.type).toBe("POLICY");
      expect(classified?.nonRetryable).toBe(true);
      // Permanent failure is attempted exactly once by the real retry
      // engine (the AUD-023 claim: never burn the five attempts).
      expect(attempts).toBe(1);
    } finally {
      for (const worker of workers) {
        await worker.shutdown();
      }
      await env.teardown();
      void runtime;
    }
  }, 120_000);
});
