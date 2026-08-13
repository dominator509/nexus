/**
 * DeploymentWorkflow (nexus.deployment.v1).
 *
 * Staged rollout with canary: each stage requires approval against its
 * action digest, runs the deploy effect idempotently, verifies, and rolls
 * back (compensates) on verification failure or cancellation.
 */

import { defineQuery, defineSignal } from "@temporalio/workflow";

import {
  DeploymentWorkflow as Contract,
  idempotencyKeyFor,
} from "@nexus/workflows";
import type {
  ApprovalSignal,
  DeploymentInput,
  DeploymentOutput,
  WorkflowResult,
  WorkflowStatusQueryResponse,
} from "@nexus/workflows";

import { queryChannel, signalChannel, WORKFLOW_TYPES } from "../config.js";
import { runStepGateWorkflow } from "./step-gate-runner.js";
import type { CancelSignalPayload } from "./step-gate-runner.js";

const approvalSignal = defineSignal<[ApprovalSignal]>(
  signalChannel(WORKFLOW_TYPES.DEPLOYMENT, "approval"),
);
const cancelSignal = defineSignal<[CancelSignalPayload]>(
  signalChannel(WORKFLOW_TYPES.DEPLOYMENT, "cancel"),
);
const statusQuery = defineQuery<WorkflowStatusQueryResponse>(
  queryChannel(WORKFLOW_TYPES.DEPLOYMENT, "status"),
);

export async function deploymentWorkflowV1(
  input: DeploymentInput,
): Promise<WorkflowResult & { output?: DeploymentOutput }> {
  return runStepGateWorkflow<DeploymentOutput>({
    seed: {
      workflowId: input.workflowId,
      label: input.canary
        ? `deploy ${input.releaseId} (canary)`
        : `deploy ${input.releaseId}`,
      entityId: input.releaseId,
      steps: input.stages.map((s) => ({
        stepId: s.stageId,
        title: s.name,
        actionId: s.actionId,
        actionDigest: s.actionDigest,
      })),
    },
    policy: Contract.policy,
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
      releaseId: record.entityId as DeploymentOutput["releaseId"],
      deployed: record.steps.every((s) => s.state === "VERIFIED"),
      rollbackRequired: record.state === "COMPENSATED",
    }),
  });
}
