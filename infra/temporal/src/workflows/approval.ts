/**
 * ApprovalWorkflow (nexus.approval.v1) - durable human approval gate.
 *
 * SPEC-023 behavior 7; SPEC-005 behavior 4; EP-006 acceptance obligations
 * 2 and 3. Real Temporal workflow code: deterministic, replay-safe, all
 * side effects through activities. Waits up to the contract's
 * approvalTimeoutMs for an ApprovalSignal bound to the exact action
 * digest and required authentication strength; explicit timeout and
 * cancel/compensation paths.
 */

import {
  type CancelledFailure,
  condition,
  defineQuery,
  defineSignal,
  isCancellation,
  proxyActivities,
  setHandler,
} from "@temporalio/workflow";

import { ApprovalWorkflow as ApprovalContract } from "@nexus/workflows";
import type {
  ApprovalInput,
  ApprovalOutput,
  ApprovalSignal,
  PendingApprovalQueryResponse,
  WorkflowResult,
  WorkflowStatusQueryResponse,
} from "@nexus/workflows";

import type { NexusActivityRegistry } from "../activity-types.js";
import {
  queryChannel,
  signalChannel,
  TASK_QUEUES,
  WORKFLOW_TYPES,
} from "../config.js";
import { createWorkflowEnv } from "../context.js";
import { toTemporalRetry } from "../retry.js";
import { compensationKeyFor } from "../state/compensation.js";
import {
  applyApprovalSignal,
  applyApprovalTimeout,
  applyCancel,
  initialApprovalState,
  isTerminalApprovalState,
  markCompensated,
} from "../state/approval.js";
import type { ApprovalRecord } from "../state/approval.js";

const activities = proxyActivities<NexusActivityRegistry>({
  taskQueue: TASK_QUEUES.ACTIVITY,
  startToCloseTimeout: "1m",
  retry: toTemporalRetry(ApprovalContract.policy.defaultActivityRetry),
});

const approvalSignal = defineSignal<[ApprovalSignal]>(
  signalChannel(WORKFLOW_TYPES.APPROVAL, "approval"),
);
const cancelSignal = defineSignal<
  [{ signalId: string; reason?: string; requestedAt: string }]
>(signalChannel(WORKFLOW_TYPES.APPROVAL, "cancel"));
const statusQuery = defineQuery<WorkflowStatusQueryResponse>(
  queryChannel(WORKFLOW_TYPES.APPROVAL, "status"),
);
const pendingQuery = defineQuery<PendingApprovalQueryResponse>(
  queryChannel(WORKFLOW_TYPES.APPROVAL, "pending"),
);

function statusResponse(
  info: ReturnType<typeof createWorkflowEnv>,
  record: ApprovalRecord,
): WorkflowStatusQueryResponse {
  const base = {
    queryType: "WORKFLOW_STATUS" as const,
    workflowId: info.workflowId as ApprovalInput["workflowId"],
    state: record.state,
    updatedAt: record.updatedAt,
  };
  return record.outcome === undefined
    ? base
    : { ...base, outcome: record.outcome };
}

export async function approvalWorkflowV1(
  input: ApprovalInput,
): Promise<WorkflowResult & { output?: ApprovalOutput }> {
  const env = createWorkflowEnv();
  let record: ApprovalRecord = initialApprovalState(input, env.nowIso());
  let cancelRequested = false;

  setHandler(approvalSignal, (signal) => {
    const result = applyApprovalSignal(record, signal, env.nowIso());
    record = result.record;
  });
  setHandler(cancelSignal, () => {
    cancelRequested = true;
  });
  setHandler(statusQuery, () => statusResponse(env, record));
  setHandler(pendingQuery, () => ({
    queryType: "PENDING_APPROVAL",
    workflowId: input.workflowId,
    approvals: record.observedSignals,
  }));

  const approvalTimeoutMs =
    ApprovalContract.policy.timeouts.approvalTimeoutMs ?? 0;

  try {
    const decided = await condition(
      () => isTerminalApprovalState(record.state) || cancelRequested,
      approvalTimeoutMs,
    );
    if (!decided) {
      record = applyApprovalTimeout(record, env.nowIso());
    }
  } catch (error) {
    if (isCancellation(error)) {
      // Client-initiated cancellation: explicit cancel/compensate path.
      record = applyCancel(
        record,
        ApprovalContract.policy.cancelAction,
        env.nowIso(),
      );
      if (
        ApprovalContract.policy.cancelAction === "COMPENSATE" &&
        record.state === "COMPENSATING"
      ) {
        const effectKey = `${input.workflowId}:approval-wait`;
        await activities.applyCompensation({
          workflowId: input.workflowId,
          effectIdempotencyKey: effectKey,
          compensationKey: compensationKeyFor(effectKey),
          reason: "approval workflow cancelled",
        });
        record = markCompensated(record, env.nowIso());
      }
    } else {
      throw error;
    }
  }

  if (cancelRequested && !isTerminalApprovalState(record.state)) {
    record = applyCancel(
      record,
      ApprovalContract.policy.cancelAction,
      env.nowIso(),
    );
    if (
      ApprovalContract.policy.cancelAction === "COMPENSATE" &&
      record.state === "COMPENSATING"
    ) {
      const effectKey = `${input.workflowId}:approval-wait`;
      await activities.applyCompensation({
        workflowId: input.workflowId,
        effectIdempotencyKey: effectKey,
        compensationKey: compensationKeyFor(effectKey),
        reason: "approval cancelled by signal",
      });
      record = markCompensated(record, env.nowIso());
    }
  }

  const result: WorkflowResult & { output?: ApprovalOutput } = {
    state: record.state,
    ...(record.outcome === undefined ? {} : { outcome: record.outcome }),
  };
  if (record.decision !== undefined) {
    result.output = {
      actionId: record.actionId,
      actionDigest: record.actionDigest,
      decision: record.decision,
    };
  }
  return result;
}
