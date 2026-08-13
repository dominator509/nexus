/**
 * Workflow registry: canonical Temporal type names -> workflow functions.
 * The client starts workflows by these names; the worker registers the
 * functions. Versioning strategy (docs/versioning.md): a breaking change
 * ships a new name here plus a new task queue.
 */

import { WORKFLOW_TYPES } from "../config.js";
import type { WorkflowTypeName } from "../config.js";

import { approvalWorkflowV1 } from "./approval.js";
import { connectorCertificationWorkflowV1 } from "./connector-certification.js";
import { deploymentWorkflowV1 } from "./deployment.js";
import { incidentRemediationWorkflowV1 } from "./incident-remediation.js";
import { objectiveWorkflowV1 } from "./objective.js";

export type WorkflowFunction = (input: unknown) => Promise<unknown>;

export const WORKFLOW_REGISTRY: Record<WorkflowTypeName, WorkflowFunction> = {
  [WORKFLOW_TYPES.OBJECTIVE]: objectiveWorkflowV1 as WorkflowFunction,
  [WORKFLOW_TYPES.APPROVAL]: approvalWorkflowV1 as WorkflowFunction,
  [WORKFLOW_TYPES.CONNECTOR_CERTIFICATION]:
    connectorCertificationWorkflowV1 as WorkflowFunction,
  [WORKFLOW_TYPES.INCIDENT_REMEDIATION]:
    incidentRemediationWorkflowV1 as WorkflowFunction,
  [WORKFLOW_TYPES.DEPLOYMENT]: deploymentWorkflowV1 as WorkflowFunction,
};

export function workflowFunctionByName(
  name: WorkflowTypeName,
): WorkflowFunction {
  const fn = WORKFLOW_REGISTRY[name];
  if (fn === undefined) {
    throw new Error(`unknown workflow type ${name}`);
  }
  return fn;
}
