/**
 * Canonical Temporal configuration (EP-006 fallback doctrine: one
 * namespace, one worker process, task queues separated by capability).
 * SPEC-023 behavior 5; ADR-010.
 */

export const NAMESPACE = "nexus";

export const TASK_QUEUES = {
  OBJECTIVE: "nexus-objective",
  APPROVAL: "nexus-approval",
  CONNECTOR_CERTIFICATION: "nexus-connector-certification",
  INCIDENT_REMEDIATION: "nexus-incident-remediation",
  DEPLOYMENT: "nexus-deployment",
  /** Activities run on a shared queue so one worker serves all kinds. */
  ACTIVITY: "nexus-activities",
} as const;

export type TaskQueueName = (typeof TASK_QUEUES)[keyof typeof TASK_QUEUES];

/** Canonical Temporal workflow type names (registry keys in src/workflows). */
export const WORKFLOW_TYPES = {
  OBJECTIVE: "nexus.objective.v1",
  APPROVAL: "nexus.approval.v1",
  CONNECTOR_CERTIFICATION: "nexus.connector-certification.v1",
  INCIDENT_REMEDIATION: "nexus.incident-remediation.v1",
  DEPLOYMENT: "nexus.deployment.v1",
} as const;

export type WorkflowTypeName =
  (typeof WORKFLOW_TYPES)[keyof typeof WORKFLOW_TYPES];

/** Signal/query channel names are versioned with the workflow name. */
export function signalChannel(
  workflowType: WorkflowTypeName,
  kind: string,
): string {
  return `${workflowType}.signal.${kind}`;
}

export function queryChannel(
  workflowType: WorkflowTypeName,
  kind: string,
): string {
  return `${workflowType}.query.${kind}`;
}
