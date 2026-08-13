import { describe, expect, it } from "vitest";

import { existsSync } from "node:fs";

import {
  NAMESPACE,
  queryChannel,
  signalChannel,
  TASK_QUEUES,
  WORKFLOW_TYPES,
} from "../config.js";
import { defaultWorkflowsPath } from "../worker.js";
import {
  WORKFLOW_REGISTRY,
  workflowFunctionByName,
} from "../workflows/index.js";

describe("ep006_unit_registry", () => {
  it("ep006_unit_registry_one_namespace_six_queues", () => {
    expect(NAMESPACE).toBe("nexus");
    expect(Object.values(TASK_QUEUES)).toHaveLength(6);
    // Activities share one queue so a single worker serves all kinds
    // (EP-006 fallback doctrine).
    expect(TASK_QUEUES.ACTIVITY).toBe("nexus-activities");
    expect(new Set(Object.values(TASK_QUEUES)).size).toBe(6);
  });

  it("ep006_unit_registry_five_workflow_types_versioned", () => {
    expect(Object.values(WORKFLOW_TYPES)).toHaveLength(5);
    for (const type of Object.values(WORKFLOW_TYPES)) {
      expect(type).toMatch(/^nexus\.[a-z-]+\.v1$/);
    }
  });

  it("ep006_unit_registry_maps_every_type_to_function", () => {
    for (const type of Object.values(WORKFLOW_TYPES)) {
      const fn = workflowFunctionByName(type);
      expect(typeof fn).toBe("function");
    }
    expect(Object.keys(WORKFLOW_REGISTRY)).toHaveLength(5);
  });

  it("ep006_unit_registry_unknown_type_throws", () => {
    expect(() =>
      workflowFunctionByName(
        "nexus.bogus.v1" as (typeof WORKFLOW_TYPES)[keyof typeof WORKFLOW_TYPES],
      ),
    ).toThrow(/unknown workflow type/);
  });

  it("ep006_unit_registry_channels_versioned", () => {
    expect(signalChannel(WORKFLOW_TYPES.APPROVAL, "approval")).toBe(
      "nexus.approval.v1.signal.approval",
    );
    expect(queryChannel(WORKFLOW_TYPES.APPROVAL, "status")).toBe(
      "nexus.approval.v1.query.status",
    );
  });

  it("ep006_unit_registry_workflows_bundle_path_exists", () => {
    const p = defaultWorkflowsPath();
    expect(existsSync(p)).toBe(true);
  });

  it("ep006_unit_registry_contract_names_match_m1", () => {
    // The five workflow names must match the M1 contracts registry names.
    expect(WORKFLOW_TYPES.OBJECTIVE).toBe("nexus.objective.v1");
    expect(WORKFLOW_TYPES.APPROVAL).toBe("nexus.approval.v1");
    expect(WORKFLOW_TYPES.CONNECTOR_CERTIFICATION).toBe(
      "nexus.connector-certification.v1",
    );
    expect(WORKFLOW_TYPES.INCIDENT_REMEDIATION).toBe(
      "nexus.incident-remediation.v1",
    );
    expect(WORKFLOW_TYPES.DEPLOYMENT).toBe("nexus.deployment.v1");
  });
});
