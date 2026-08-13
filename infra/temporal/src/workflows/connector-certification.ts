/**
 * ConnectorCertificationWorkflow (nexus.connector-certification.v1).
 *
 * Certifies a connector against a real provider: each step runs as an
 * approved, idempotent effect, is verified, and failure compensates.
 * Effect and verification activities are registered by the connector
 * nodes; orchestration is the shared step-gate runner.
 */

import { defineQuery, defineSignal } from "@temporalio/workflow";

import {
  ConnectorCertificationWorkflow as Contract,
  idempotencyKeyFor,
} from "@nexus/workflows";
import type {
  ApprovalSignal,
  ConnectorCertificationInput,
  ConnectorCertificationOutput,
  WorkflowResult,
  WorkflowStatusQueryResponse,
} from "@nexus/workflows";

import { queryChannel, signalChannel, WORKFLOW_TYPES } from "../config.js";
import { runStepGateWorkflow } from "./step-gate-runner.js";
import type { CancelSignalPayload } from "./step-gate-runner.js";

const approvalSignal = defineSignal<[ApprovalSignal]>(
  signalChannel(WORKFLOW_TYPES.CONNECTOR_CERTIFICATION, "approval"),
);
const cancelSignal = defineSignal<[CancelSignalPayload]>(
  signalChannel(WORKFLOW_TYPES.CONNECTOR_CERTIFICATION, "cancel"),
);
const statusQuery = defineQuery<WorkflowStatusQueryResponse>(
  queryChannel(WORKFLOW_TYPES.CONNECTOR_CERTIFICATION, "status"),
);

export async function connectorCertificationWorkflowV1(
  input: ConnectorCertificationInput,
): Promise<WorkflowResult & { output?: ConnectorCertificationOutput }> {
  return runStepGateWorkflow<ConnectorCertificationOutput>({
    seed: {
      workflowId: input.workflowId,
      label: `connector ${input.provider}`,
      entityId: input.connectorId,
      steps: input.steps,
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
      connectorId:
        record.entityId as ConnectorCertificationOutput["connectorId"],
      certified: record.steps.every((s) => s.state === "VERIFIED"),
      evidenceRef: `nexus:cert:${record.entityId}`,
    }),
  });
}
