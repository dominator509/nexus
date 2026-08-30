/**
 * EP-006 M4 / AUD-023: permanent/transient retry classification at the
 * REAL Temporal activity boundary (TESTING.md line 36; SPEC-006 behavior
 * 7).
 *
 * AUD-023's original finding: "nonRetryableErrorTypes and non-retryable
 * ApplicationFailure are never supplied, so permanent failures get up to
 * five attempts." The register marked this VERIFIED_FIXED on 74 unit
 * tests whose interceptor proof used a hand-rolled `next()` double - a
 * shell. This test replaces that claim with a REAL boundary proof:
 *
 *   1. A real activity (runEffect) throws NexusWorkflowError through the
 *      real NexusFailureInterceptor wired into the real worker, against
 *      a real Temporal server (temporalio/server:1.31.2 container).
 *   2. A PERMANENT failure (POLICY) must be attempted EXACTLY ONCE - the
 *      retry engine must never give it a second attempt - and the
 *      failure surfaced to the client must carry type=POLICY with
 *      nonRetryable=true.
 *   3. A TRANSIENT failure (UNAVAILABLE) must be retried by the real
 *      engine (attempt count > 1) and the workflow must then complete.
 *
 * No mocks, no `next()` doubles: every assertion crosses the real
 * activity boundary through real gRPC.
 */

import { describe, expect, it } from "vitest";

import {
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
  workflowError,
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

const WID_PERM = "0193a1f2-0000-7000-8000-000000000611";
const WID_TRAN = "0193a1f2-0000-7000-8000-000000000612";
const STEP_1 = "0193a1f2-0000-7000-8000-000000000621";
const OBJ_PERM = "0193a1f2-0000-7000-8000-000000000631";
const OBJ_TRAN = "0193a1f2-0000-7000-8000-000000000632";
const DIGEST_1 = "a".repeat(64);

function idempotencyKey(
  workflowId: ReturnType<typeof parseWorkflowId>,
  activityId: string,
): string {
  return `${workflowId}:${activityId}:1`;
}

function objectiveInput(
  workflowId: ReturnType<typeof parseWorkflowId>,
  objectiveId: string,
  correlationId: string,
): ObjectiveInput {
  return {
    workflowId,
    tenantId: "tenant-aud023",
    correlationId,
    principal: PRINCIPAL_HUMAN,
    objectiveId: parseObjectiveId(objectiveId),
    title: "retry classification objective",
    milestones: [
      {
        milestoneId: parseActivityId(STEP_1),
        title: "step one",
        actionId: actionIdA,
        actionDigest: parseActionDigest(DIGEST_1),
      },
    ],
  };
}

async function startIndependentSession(): Promise<{
  stack: Awaited<ReturnType<typeof startTemporalStack>>;
  connection: NativeConnection;
  sdkConnection: Connection;
  client: Client;
  ownSession: TestSession;
}> {
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
  return { session, stack, connection, sdkConnection, client, ownSession };
}

describe("ep006_failure_retry_classification_real", () => {
  it("ep006_failure_permanent_never_retried_real_server", async () => {
    const { stack, connection, sdkConnection, client, ownSession } =
      await startIndependentSession();
    const workflowId = parseWorkflowId(WID_PERM);
    let attempts = 0;
    const permanentActivities = {
      runEffect: async () => {
        attempts += 1;
        // A PERMANENT SPEC-006 failure: the real interceptor must
        // convert this to an ApplicationFailure with type=POLICY and
        // nonRetryable=true, and the real retry engine must give it
        // exactly one attempt.
        throw workflowError("POLICY", "permission denied");
      },
      verifyEffect: async () => ({ verified: true }),
      applyCompensation: async (input: ApplyCompensationInput) => ({
        compensated: true as const,
        compensationKey: input.compensationKey,
      }),
    };

    let worker: Awaited<ReturnType<typeof startWorker>> | undefined;
    try {
      worker = await startWorker(
        ownSession,
        [TASK_QUEUES.OBJECTIVE],
        permanentActivities,
      );
      const handle = await client.workflow.start(WORKFLOW_TYPES.OBJECTIVE, {
        taskQueue: TASK_QUEUES.OBJECTIVE,
        workflowId,
        args: [objectiveInput(workflowId, OBJ_PERM, "corr-perm")],
      });

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

      // The workflow must FAIL (the permanent failure is rethrown by the
      // step-gate runner - never swallowed), and the failure surfaced to
      // the client must be the classified ApplicationFailure.
      const failure = await handle.result().catch((err: unknown) => err);
      expect(failure).toBeDefined();
      // Walk the cause chain to the classified failure. The workflow
      // rethrows the activity failure, so the client sees
      // WorkflowFailedError -> ActivityFailure -> ApplicationFailure.
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

      // The core AUD-023 claim: a permanent failure is attempted EXACTLY
      // ONCE. Before the fix, toTemporalRetry() never supplied
      // nonRetryableErrorTypes and the interceptor never marked the
      // failure non-retryable, so Temporal retried five times.
      expect(attempts).toBe(1);
    } finally {
      if (worker !== undefined) {
        await worker.shutdown();
      }
      await connection.close();
      await sdkConnection.close();
      await stack.dispose();
    }
  }, 120_000);

  it("ep006_failure_transient_is_retried_real_server", async () => {
    const { stack, connection, sdkConnection, client, ownSession } =
      await startIndependentSession();
    const workflowId = parseWorkflowId(WID_TRAN);
    let attempts = 0;
    const transientActivities = {
      runEffect: async () => {
        attempts += 1;
        if (attempts === 1) {
          // A TRANSIENT SPEC-006 failure: the real interceptor converts
          // this to ApplicationFailure(type=UNAVAILABLE,
          // nonRetryable=false), so the real engine retries.
          throw workflowError("UNAVAILABLE", "provider down");
        }
        return { receiptId: "receipt-transient" };
      },
      verifyEffect: async () => ({ verified: true }),
      applyCompensation: async (input: ApplyCompensationInput) => ({
        compensated: true as const,
        compensationKey: input.compensationKey,
      }),
    };

    let worker: Awaited<ReturnType<typeof startWorker>> | undefined;
    try {
      worker = await startWorker(
        ownSession,
        [TASK_QUEUES.OBJECTIVE],
        transientActivities,
      );
      const handle = await client.workflow.start(WORKFLOW_TYPES.OBJECTIVE, {
        taskQueue: TASK_QUEUES.OBJECTIVE,
        workflowId,
        args: [objectiveInput(workflowId, OBJ_TRAN, "corr-tran")],
      });

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

      const result = await handle.result();
      expect(result.state).toBe("SUCCEEDED");
      expect(result.outcome).toBe("SUCCEEDED");
      // A transient failure must have been retried by the real engine:
      // first attempt throws UNAVAILABLE, second attempt succeeds.
      expect(attempts).toBe(2);
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
