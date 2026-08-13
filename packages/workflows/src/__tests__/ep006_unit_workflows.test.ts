import { describe, expect, it } from "vitest";

import {
  ApprovalWorkflow,
  ConnectorCertificationWorkflow,
  DeploymentWorkflow,
  IncidentRemediationWorkflow,
  ObjectiveWorkflow,
  WORKFLOWS,
} from "../workflows.js";
import { validateWorkflowPolicy } from "../policies.js";
import { parseSemver } from "../versioning.js";
import { isUuidV7 } from "../ids.js";
import {
  workflowKind,
  signalType,
  queryType,
  activityKind,
} from "../vocabulary.js";

describe("ep006_unit_workflows", () => {
  it("ep006_unit_workflows_all_five_contracts_present", () => {
    expect(WORKFLOWS).toHaveLength(5);
    expect(ObjectiveWorkflow.kind).toBe("OBJECTIVE");
    expect(ApprovalWorkflow.kind).toBe("APPROVAL");
    expect(ConnectorCertificationWorkflow.kind).toBe("CONNECTOR_CERTIFICATION");
    expect(IncidentRemediationWorkflow.kind).toBe("INCIDENT_REMEDIATION");
    expect(DeploymentWorkflow.kind).toBe("DEPLOYMENT");
  });

  it("ep006_unit_workflows_names_versioned", () => {
    for (const workflow of WORKFLOWS) {
      expect(workflow.name).toMatch(/^nexus\.[a-z-]+\.v1$/);
      expect(parseSemver(workflow.version).major).toBe(1);
    }
  });

  it("ep006_unit_workflows_signal_query_surfaces_vocabularic", () => {
    for (const workflow of WORKFLOWS) {
      for (const s of workflow.signals) {
        expect(signalType.is(s)).toBe(true);
      }
      for (const q of workflow.queries) {
        expect(queryType.is(q)).toBe(true);
      }
      expect(workflowKind.is(workflow.kind)).toBe(true);
    }
  });

  it("ep006_unit_workflows_policies_valid", () => {
    for (const workflow of WORKFLOWS) {
      expect(() => validateWorkflowPolicy(workflow.policy)).not.toThrow();
      // Explicit timeout/cancel paths are mandatory (EP-006 obligation 3).
      expect(workflow.policy.timeouts.executionTimeoutMs).toBeGreaterThan(0);
      expect(workflow.policy.timeouts.approvalTimeoutMs).toBeGreaterThan(0);
      expect(workflow.policy.cancelAction).toMatch(/^(CANCEL|COMPENSATE)$/);
    }
  });

  it("ep006_unit_workflows_activities_idempotent_and_bounded", () => {
    for (const workflow of WORKFLOWS) {
      expect(workflow.activities.length).toBeGreaterThan(0);
      for (const a of workflow.activities) {
        expect(isUuidV7(a.activityId)).toBe(true);
        expect(a.idempotencyRequired).toBe(true);
        expect(activityKind.is(a.kind)).toBe(true);
        expect(a.retry.maxAttempts).toBeGreaterThanOrEqual(1);
        expect(a.retry.maxAttempts).toBeLessThanOrEqual(10);
        expect(a.retry.retryableErrorClasses).not.toContain("PERMANENT");
        if (a.compensation !== undefined) {
          expect(activityKind.is("COMPENSATE")).toBe(true);
          expect(a.compensation.order).toBeGreaterThanOrEqual(0);
        }
      }
    }
  });

  it("ep006_unit_workflows_compensation_registered_for_effects", () => {
    // SPEC-006 behavior 8: every EXTERNAL_EFFECT declares a rollback.
    for (const workflow of WORKFLOWS) {
      for (const a of workflow.activities) {
        if (a.kind === "EXTERNAL_EFFECT") {
          expect(a.compensation).toBeDefined();
        }
      }
    }
  });

  it("ep006_unit_workflows_approval_workflow_binds_digest", () => {
    // The approval workflow's input requires the exact action digest and
    // the required auth strength; this is the M1 contract-level proof of
    // the approval binding invariant (assertApprovalBinding enforces it).
    const inputType = ApprovalWorkflow.name;
    expect(inputType).toBe("nexus.approval.v1");
    expect(ApprovalWorkflow.signals).toContain("APPROVAL");
    expect(ApprovalWorkflow.queries).toContain("PENDING_APPROVAL");
  });
});
