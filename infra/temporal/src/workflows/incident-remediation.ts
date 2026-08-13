/**
 * IncidentRemediationWorkflow (nexus.incident-remediation.v1).
 *
 * Remediation loop with human approval per step (SPEC-018 discipline):
 * each remediation step is approved against its action digest, executed
 * idempotently, verified, and compensated on failure or cancellation.
 */

import { defineQuery, defineSignal } from "@temporalio/workflow";

import {
  IncidentRemediationWorkflow as Contract,
  idempotencyKeyFor,
} from "@nexus/workflows";
import type {
  ApprovalSignal,
  IncidentRemediationInput,
  IncidentRemediationOutput,
  WorkflowResult,
  WorkflowStatusQueryResponse,
} from "@nexus/workflows";

import { queryChannel, signalChannel, WORKFLOW_TYPES } from "../config.js";
import { runStepGateWorkflow } from "./step-gate-runner.js";
import type { CancelSignalPayload } from "./step-gate-runner.js";

const approvalSignal = defineSignal<[ApprovalSignal]>(
  signalChannel(WORKFLOW_TYPES.INCIDENT_REMEDIATION, "approval"),
);
const cancelSignal = defineSignal<[CancelSignalPayload]>(
  signalChannel(WORKFLOW_TYPES.INCIDENT_REMEDIATION, "cancel"),
);
const statusQuery = defineQuery<WorkflowStatusQueryResponse>(
  queryChannel(WORKFLOW_TYPES.INCIDENT_REMEDIATION, "status"),
);

export async function incidentRemediationWorkflowV1(
  input: IncidentRemediationInput,
): Promise<WorkflowResult & { output?: IncidentRemediationOutput }> {
  return runStepGateWorkflow<IncidentRemediationOutput>({
    seed: {
      workflowId: input.workflowId,
      label: `incident ${input.severity}: ${input.diagnosis}`,
      entityId: input.incidentId,
      steps: input.remediationPlan,
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
      incidentId: record.entityId as IncidentRemediationOutput["incidentId"],
      remediated: record.steps.every((s) => s.state === "VERIFIED"),
      verificationRef: `nexus:remediation:${record.entityId}`,
    }),
  });
}
