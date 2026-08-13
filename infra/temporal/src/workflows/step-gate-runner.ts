/**
 * Shared step-gate Temporal orchestration (SPEC-023 behaviors 5-6;
 * ADR-010; EP-006 obligations 1, 3, 4).
 *
 * Used by the objective, connector-certification, incident-remediation,
 * and deployment workflows: each step awaits an approval bound to its
 * action digest, executes its effect through an idempotent activity,
 * verifies, and advances; failure or cancellation compensates executed
 * steps in reverse order. Deterministic and replay-safe; all side effects
 * live in activities.
 */

import {
  condition,
  isCancellation,
  proxyActivities,
  setHandler,
} from "@temporalio/workflow";
import type { QueryDefinition, SignalDefinition } from "@temporalio/common";

import { idempotencyKeyFor } from "@nexus/workflows";
import type {
  ApprovalSignal,
  WorkflowPolicy,
  WorkflowResult,
  WorkflowStatusQueryResponse,
} from "@nexus/workflows";

import type { NexusActivityRegistry } from "../activity-types.js";
import { TASK_QUEUES } from "../config.js";
import { createWorkflowEnv } from "../context.js";
import { toTemporalRetry } from "../retry.js";
import { compensationKeyFor } from "../state/compensation.js";
import {
  applyStepGateSignal,
  beginStepExecution,
  cancelStepGate,
  completeStep,
  currentStep,
  initialStepGateState,
  isTerminalStepGateState,
  markStepGateCompensated,
  startStepGate,
  timeoutStepGate,
} from "../state/step-gate.js";
import type {
  StepGateRecord,
  StepGateSeed,
  StepRecord,
} from "../state/step-gate.js";

export interface CancelSignalPayload {
  readonly signalId: string;
  readonly reason?: string;
  readonly requestedAt: string;
}

export interface StepGateRunOptions<O = unknown> {
  readonly seed: StepGateSeed;
  readonly policy: WorkflowPolicy;
  readonly approvalSignal: SignalDefinition<[ApprovalSignal]>;
  readonly cancelSignal: SignalDefinition<[CancelSignalPayload]>;
  readonly statusQuery: QueryDefinition<WorkflowStatusQueryResponse>;
  /** Canonical idempotency key for a step's effect. */
  readonly effectKeyFor: (step: StepRecord) => string;
  /** Output derived from the final record on SUCCESS. */
  readonly buildOutput: (record: StepGateRecord) => O;
}

function statusResponse(record: StepGateRecord): WorkflowStatusQueryResponse {
  const base = {
    queryType: "WORKFLOW_STATUS" as const,
    workflowId: record.workflowId,
    state: record.state,
    updatedAt: record.updatedAt,
  };
  return record.outcome === undefined
    ? base
    : { ...base, outcome: record.outcome };
}

async function compensateStep(
  activities: NexusActivityRegistry,
  workflowId: string,
  effectKey: string,
  reason: string,
): Promise<void> {
  await activities.applyCompensation({
    workflowId: workflowId as StepGateRecord["workflowId"],
    effectIdempotencyKey: effectKey,
    compensationKey: compensationKeyFor(effectKey),
    reason,
  });
}

export async function runStepGateWorkflow<O = unknown>(
  opts: StepGateRunOptions<O>,
): Promise<WorkflowResult & { output?: O }> {
  const env = createWorkflowEnv();
  const activities = proxyActivities<NexusActivityRegistry>({
    taskQueue: TASK_QUEUES.ACTIVITY,
    startToCloseTimeout: "10m",
    retry: toTemporalRetry(opts.policy.defaultActivityRetry),
  });

  let record: StepGateRecord = startStepGate(
    initialStepGateState(opts.seed, env.nowIso()),
    env.nowIso(),
  );
  let cancelRequested = false;

  setHandler(opts.approvalSignal, (signal) => {
    const result = applyStepGateSignal(record, signal, env.nowIso());
    record = result.record;
  });
  setHandler(opts.cancelSignal, () => {
    cancelRequested = true;
  });
  setHandler(opts.statusQuery, () => statusResponse(record));

  const perStepTimeoutMs = opts.policy.timeouts.approvalTimeoutMs ?? 0;

  try {
    while (!isTerminalStepGateState(record.state)) {
      const step = currentStep(record);
      if (step === undefined) {
        break;
      }
      if (step.state === "AWAITING_APPROVAL") {
        // Wake when the approval signal advances the CURRENT step to
        // APPROVED, when the whole run turns terminal, or on cancel.
        // A signal handler mutates `record` (closure) so the predicate
        // must re-read the live step state - checking only terminal
        // states here would deadlock: step-gate APPROVED is not a
        // terminal workflow state.
        const decided = await condition(
          () =>
            cancelRequested ||
            isTerminalStepGateState(record.state) ||
            currentStep(record)?.state !== "AWAITING_APPROVAL",
          perStepTimeoutMs,
        );
        if (!decided && !cancelRequested) {
          record = timeoutStepGate(record, env.nowIso());
        }
        continue;
      }
      if (step.state === "APPROVED") {
        record = beginStepExecution(record, env.nowIso());
        const effectKey = opts.effectKeyFor(step);
        const effect = await activities.runEffect({
          workflowId: opts.seed.workflowId,
          idempotencyKey: effectKey,
          actionDigest: step.actionDigest,
          payload: { stepId: step.stepId, title: step.title },
        });
        const verification = await activities.verifyEffect({
          workflowId: opts.seed.workflowId,
          idempotencyKey: effectKey,
          ...(effect.receiptId === undefined
            ? {}
            : { receiptId: effect.receiptId }),
          expectedState: { stepId: step.stepId },
        });
        record = completeStep(record, verification.verified, env.nowIso());
        if (record.state === "FAILED") {
          await compensateStep(
            activities,
            opts.seed.workflowId,
            effectKey,
            `step ${step.stepId} verification failed`,
          );
          record = markStepGateCompensated(record, env.nowIso());
        }
      } else {
        break;
      }
    }
  } catch (error) {
    if (isCancellation(error)) {
      record = cancelStepGate(record, opts.policy.cancelAction, env.nowIso());
      if (
        opts.policy.cancelAction === "COMPENSATE" &&
        record.state === "COMPENSATING"
      ) {
        const step = currentStep(record);
        if (step !== undefined) {
          await compensateStep(
            activities,
            opts.seed.workflowId,
            opts.effectKeyFor(step),
            "workflow cancelled",
          );
        }
        record = markStepGateCompensated(record, env.nowIso());
      }
    } else {
      throw error;
    }
  }

  if (cancelRequested && !isTerminalStepGateState(record.state)) {
    record = cancelStepGate(record, opts.policy.cancelAction, env.nowIso());
    if (
      opts.policy.cancelAction === "COMPENSATE" &&
      record.state === "COMPENSATING"
    ) {
      const step = currentStep(record);
      if (step !== undefined) {
        await compensateStep(
          activities,
          opts.seed.workflowId,
          opts.effectKeyFor(step),
          "workflow cancelled by signal",
        );
      }
      record = markStepGateCompensated(record, env.nowIso());
    }
  }

  const result: WorkflowResult & { output?: O } = {
    state: record.state,
    ...(record.outcome === undefined ? {} : { outcome: record.outcome }),
  };
  if (record.state === "SUCCEEDED") {
    result.output = opts.buildOutput(record);
  }
  return result;
}
