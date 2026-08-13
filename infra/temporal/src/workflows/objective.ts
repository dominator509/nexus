/**
 * ObjectiveWorkflow (nexus.objective.v1) - long-running objective with
 * milestone approvals (SPEC-023 behavior 5; ADR-010; EP-006 obligations
 * 1, 3, 4). Orchestration lives in the shared step-gate runner; state
 * transitions in src/state/step-gate.ts.
 */

import { defineQuery, defineSignal } from "@temporalio/workflow";

import {
  ObjectiveWorkflow as ObjectiveContract,
  idempotencyKeyFor,
} from "@nexus/workflows";
import type {
  ApprovalSignal,
  ObjectiveInput,
  ObjectiveOutput,
  WorkflowResult,
  WorkflowStatusQueryResponse,
} from "@nexus/workflows";

import { queryChannel, signalChannel, WORKFLOW_TYPES } from "../config.js";
import { runStepGateWorkflow } from "./step-gate-runner.js";
import type { CancelSignalPayload } from "./step-gate-runner.js";

const approvalSignal = defineSignal<[ApprovalSignal]>(
  signalChannel(WORKFLOW_TYPES.OBJECTIVE, "approval"),
);
const cancelSignal = defineSignal<[CancelSignalPayload]>(
  signalChannel(WORKFLOW_TYPES.OBJECTIVE, "cancel"),
);
const statusQuery = defineQuery<WorkflowStatusQueryResponse>(
  queryChannel(WORKFLOW_TYPES.OBJECTIVE, "status"),
);

export async function objectiveWorkflowV1(
  input: ObjectiveInput,
): Promise<WorkflowResult & { output?: ObjectiveOutput }> {
  return runStepGateWorkflow({
    seed: {
      workflowId: input.workflowId,
      label: input.title,
      entityId: input.objectiveId,
      steps: input.milestones.map((m) => ({
        stepId: m.milestoneId,
        title: m.title,
        actionId: m.actionId,
        actionDigest: m.actionDigest,
      })),
    },
    policy: ObjectiveContract.policy,
    approvalSignal,
    cancelSignal,
    statusQuery,
    effectKeyFor: (step) =>
      idempotencyKeyFor(
        input.workflowId,
        step.stepId as import("@nexus/workflows").ActivityId,
        1,
      ),
    buildOutput: (record) => ({
      objectiveId: record.entityId as ObjectiveInput["objectiveId"],
      completedMilestones: record.steps
        .filter((s) => s.state === "VERIFIED")
        .map((s) => s.stepId),
    }),
  });
}
