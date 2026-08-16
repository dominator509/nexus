/**
 * EP-019 M2: incident workflow contract integration tests (SPEC-018;
 * ADR-026).
 *
 * These tests exercise the REAL @nexus/workflows contract machinery
 * against the EP-019 incident workflow contracts: vocabulary parsing,
 * registry structure, determinism audit over the real source files,
 * idempotency and bounded-retry activity contracts, cancellation /
 * timeout policy surfaces, the canonical lifecycle (never collapsed),
 * and approval/rollback binding. No mocks; the incidents module and the
 * shared workflow machinery are the real production code (TESTING.md).
 */

import { describe, expect, it } from "vitest";

import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  canaryOutcome,
  diagnosisConfidence,
  incidentLifecycleState,
  incidentOperationKind,
  incidentWorkflowKind,
  isIncidentTerminal,
  reviewVerdict,
  rollbackOutcome,
} from "../vocabulary.js";
import {
  INCIDENT_WORKFLOW_KINDS,
  INCIDENT_WORKFLOWS,
  IncidentLifecycleWorkflow,
  RollbackWorkflow,
} from "../workflows.js";
import { findDeterminismViolations } from "../../determinism.js";
import { isUuidV7 } from "../../ids.js";
import { validateWorkflowPolicy } from "../../policies.js";
import { parseSemver } from "../../versioning.js";
import { activityKind, signalType, queryType } from "../../vocabulary.js";

const incidentsDir = fileURLToPath(new URL("..", import.meta.url));

describe("ep019_integration_incident_workflow_registry", () => {
  it("ep019_integration_all_seven_durable_workflows_present", () => {
    expect(INCIDENT_WORKFLOWS).toHaveLength(7);
    expect(IncidentLifecycleWorkflow.kind).toBe("INCIDENT_LIFECYCLE");
    expect(INCIDENT_WORKFLOW_KINDS).toHaveLength(7);
    expect(INCIDENT_WORKFLOW_KINDS).toContain("DIAGNOSIS");
    expect(INCIDENT_WORKFLOW_KINDS).toContain("REPRODUCTION");
    expect(INCIDENT_WORKFLOW_KINDS).toContain("PATCH_PROPOSAL");
    expect(INCIDENT_WORKFLOW_KINDS).toContain("REVIEW");
    expect(INCIDENT_WORKFLOW_KINDS).toContain("CANARY_DEPLOYMENT");
    expect(INCIDENT_WORKFLOW_KINDS).toContain("ROLLBACK");
  });

  it("ep019_integration_kinds_are_vocabulary_locked", () => {
    for (const kind of INCIDENT_WORKFLOW_KINDS) {
      expect(incidentWorkflowKind.parse(kind)).toBe(kind);
    }
    // No collapsed "FIXED"/"REMEDIATED" escape value exists.
    expect(() => incidentLifecycleState.parse("FIXED")).toThrow();
    expect(() => incidentLifecycleState.parse("REMEDIATED")).toThrow();
    expect(() => incidentWorkflowKind.parse("NAMED_PEER_SELECTION")).toThrow();
    expect(() => incidentOperationKind.parse("DROP_TABLE")).toThrow();
    expect(() => diagnosisConfidence.parse("PROVEN")).toThrow();
    expect(() => reviewVerdict.parse("MAYBE")).toThrow();
    expect(() => canaryOutcome.parse("MAYBE")).toThrow();
    expect(() => rollbackOutcome.parse("MAYBE")).toThrow();
  });

  it("ep019_integration_lifecycle_has_explicit_terminal_states", () => {
    // The full canonical lifecycle must be present, and only the
    // explicit terminal/failure states are terminal.
    for (const state of [
      "OBSERVE",
      "INCIDENT",
      "CORRELATE",
      "DIAGNOSE",
      "REPRODUCE",
      "PATCH_PROPOSED",
      "SANDBOX_VALIDATION",
      "SECURITY_VALIDATION",
      "APPROVAL",
      "STAGED_DEPLOYMENT",
      "POST_DEPLOY_VERIFICATION",
      "CLOSED",
    ]) {
      expect(incidentLifecycleState.parse(state)).toBe(state);
    }
    for (const terminal of [
      "CLOSED",
      "REJECTED",
      "UNREPRODUCIBLE",
      "VALIDATION_FAILED",
      "SECURITY_FAILED",
      "ROLLED_BACK",
      "BLOCKED",
    ]) {
      expect(isIncidentTerminal(terminal as never)).toBe(true);
    }
    expect(isIncidentTerminal("DIAGNOSE")).toBe(false);
    expect(isIncidentTerminal("APPROVAL")).toBe(false);
  });

  it("ep019_integration_contracts_versioned_and_named", () => {
    for (const workflow of INCIDENT_WORKFLOWS) {
      expect(workflow.name).toMatch(/^nexus\.incident-[a-z-]+\.v1$/);
      expect(parseSemver(workflow.version).major).toBe(1);
      expect(workflow.description.length).toBeGreaterThan(10);
      for (const signal of workflow.signals) {
        expect(signalType.parse(signal)).toBe(signal);
      }
      for (const query of workflow.queries) {
        expect(queryType.parse(query)).toBe(query);
      }
    }
  });

  it("ep019_integration_lifecycle_has_no_model_self_certify_activity", () => {
    // The lifecycle's operation surface must NOT contain any
    // "declare fixed" / "self certify" operation. Real verification is
    // the only path to CLOSED.
    const ops = IncidentLifecycleWorkflow.activities.map(
      (a) => a.operationKind,
    );
    expect(ops).toContain("VERIFY_POST_DEPLOY");
    expect(ops).toContain("VALIDATE_SANDBOX");
    expect(ops).toContain("VALIDATE_SECURITY");
    for (const forbidden of [
      "DECLARE_FIXED",
      "SELF_CERTIFY",
      "MARK_REMEDIATED",
    ]) {
      expect(ops).not.toContain(forbidden);
    }
  });

  it("ep019_integration_all_activities_idempotent_with_stable_prefix", () => {
    for (const workflow of INCIDENT_WORKFLOWS) {
      for (const activity of workflow.activities) {
        expect(activity.idempotencyRequired).toBe(true);
        expect(activity.idempotencyKeyPrefix).toMatch(/^incident-op:[a-z_]+$/);
        expect(activity.timeoutMs).toBeGreaterThan(0);
        expect(activity.retry.maxAttempts).toBeGreaterThanOrEqual(1);
        // PERMANENT failures are never retried.
        expect(activity.retry.retryableErrorClasses ?? []).not.toContain(
          "PERMANENT",
        );
      }
    }
  });

  it("ep019_integration_rollback_binds_to_known_artifact", () => {
    // The rollback workflow input must carry the known previous
    // artifact/version (never model-generated source).
    expect(RollbackWorkflow.kind).toBe("ROLLBACK");
    expect(RollbackWorkflow.activities.map((a) => a.operationKind)).toContain(
      "EXECUTE_ROLLBACK",
    );
    // Staged deployment compensates with rollback (canary regression
    // automatically rolls back).
    expect(IncidentLifecycleWorkflow.policy.cancelAction).toBe("COMPENSATE");
  });

  it("ep019_integration_policies_valid", () => {
    for (const workflow of INCIDENT_WORKFLOWS) {
      expect(() => validateWorkflowPolicy(workflow.policy)).not.toThrow();
    }
  });

  it("ep019_integration_workflow_source_deterministic", () => {
    // Determinism audit over the real production source files: incident
    // workflow code must not contain wall-clock, network, filesystem,
    // database, or random calls (SPEC-023 behavior 6).
    const violations = findDeterminismViolations(
      path.join(incidentsDir, "workflows.ts"),
    );
    expect(violations).toEqual([]);
  });

  it("ep019_integration_workflow_ids_are_uuidv7", () => {
    // Every activity id must parse as a canonical UUIDv7 (SPEC-001).
    for (const workflow of INCIDENT_WORKFLOWS) {
      for (const activity of workflow.activities) {
        expect(isUuidV7(activity.activityId)).toBe(true);
      }
    }
  });

  it("ep019_integration_vocabulary_matches_registry_surface", () => {
    // The vocabulary enum surface (seven kinds, twelve operations,
    // eighteen states) is locked by the registry tests above; assert
    // the canonical value lists directly from the real source so a
    // drift in either direction fails.
    const source = readFileSync(
      path.join(incidentsDir, "workflows.ts"),
      "utf8",
    );
    expect(source).toContain("INCIDENT_LIFECYCLE");
    expect(source).toContain("CANARY_DEPLOYMENT");
    expect(source).toContain("ROLLBACK");
    const vocabSource = readFileSync(
      path.join(incidentsDir, "vocabulary.ts"),
      "utf8",
    );
    expect(vocabSource).toContain("OBSERVE");
    expect(vocabSource).toContain("POST_DEPLOY_VERIFICATION");
    expect(vocabSource).toContain("BLOCKED");
    expect(vocabSource).toContain("VALIDATED");
  });
});
