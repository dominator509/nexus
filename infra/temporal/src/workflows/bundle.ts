/**
 * Workflow bundle entry (ADR-010; SPEC-023).
 *
 * The Temporal worker resolves a workflow by its canonical type name:
 * `mod[workflowType]` (worker-interface.js). This module is the single
 * bundle entrypoint that exports the five nexus workflows under their
 * canonical versioned names (nexus.approval.v1, ...) so the client and
 * worker agree on the registry keys. Keep this file as the default
 * `workflowsPath`; do not add I/O or clock calls here (determinism audit
 * covers src/workflows).
 */

export { approvalWorkflowV1 as "nexus.approval.v1" } from "./approval.js";
export { connectorCertificationWorkflowV1 as "nexus.connector-certification.v1" } from "./connector-certification.js";
export { deploymentWorkflowV1 as "nexus.deployment.v1" } from "./deployment.js";
export { incidentRemediationWorkflowV1 as "nexus.incident-remediation.v1" } from "./incident-remediation.js";
export { objectiveWorkflowV1 as "nexus.objective.v1" } from "./objective.js";
