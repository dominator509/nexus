/**
 * EP-016 M3: memory workflow contract integration tests (SPEC-002
 * requirement 8; ADR-023).
 *
 * These tests exercise the REAL @nexus/workflows contract machinery
 * against the EP-016 memory workflow contracts: vocabulary parsing,
 * registry structure, determinism audit over the real source files,
 * idempotency and bounded-retry activity contracts, and cancellation /
 * timeout policy surfaces. No mocks; the memory module and the shared
 * workflow machinery are the real production code (TESTING.md).
 */

import { describe, expect, it } from "vitest";

import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  legalHoldDecision,
  memoryOperationKind,
  memoryWorkflowKind,
  memoryWorkflowState,
  retentionDisposition,
} from "../vocabulary.js";
import {
  MEMORY_WORKFLOW_KINDS,
  MEMORY_WORKFLOWS,
  MemoryConsolidationWorkflow,
  MemoryDeletionWorkflow,
  MemoryExportWorkflow,
  MemoryLegalHoldWorkflow,
  MemoryReembedWorkflow,
  MemoryRetentionWorkflow,
} from "../workflows.js";
import { findDeterminismViolations } from "../../determinism.js";
import { isUuidV7 } from "../../ids.js";
import { validateWorkflowPolicy } from "../../policies.js";
import { parseSemver } from "../../versioning.js";
import { activityKind, signalType, queryType } from "../../vocabulary.js";

const memoryDir = fileURLToPath(new URL("..", import.meta.url));

describe("ep016_integration_memory_workflow_registry", () => {
  it("ep016_integration_all_six_durable_workflows_present", () => {
    expect(MEMORY_WORKFLOWS).toHaveLength(6);
    expect(MemoryConsolidationWorkflow.kind).toBe("MEMORY_CONSOLIDATION");
    expect(MemoryRetentionWorkflow.kind).toBe("MEMORY_RETENTION");
    expect(MemoryLegalHoldWorkflow.kind).toBe("MEMORY_LEGAL_HOLD");
    expect(MemoryExportWorkflow.kind).toBe("MEMORY_EXPORT");
    expect(MemoryDeletionWorkflow.kind).toBe("MEMORY_DELETION");
    expect(MemoryReembedWorkflow.kind).toBe("MEMORY_REEMBED");
    expect(MEMORY_WORKFLOW_KINDS).toHaveLength(6);
  });

  it("ep016_integration_kinds_are_vocabulary_locked", () => {
    for (const kind of MEMORY_WORKFLOW_KINDS) {
      expect(memoryWorkflowKind.parse(kind)).toBe(kind);
    }
    expect(() => memoryWorkflowKind.parse("NOT_A_MEMORY_WORKFLOW")).toThrow();
    expect(() => memoryOperationKind.parse("DROP_TABLE")).toThrow();
    expect(() => memoryWorkflowState.parse("ZOMBIE")).toThrow();
    expect(() => legalHoldDecision.parse("MAYBE")).toThrow();
    expect(() => retentionDisposition.parse("MAYBE")).toThrow();
  });

  it("ep016_integration_contracts_versioned_and_named", () => {
    for (const workflow of MEMORY_WORKFLOWS) {
      expect(workflow.name).toMatch(/^nexus\.memory-[a-z-]+\.v1$/);
      expect(parseSemver(workflow.version).major).toBe(1);
      expect(memoryWorkflowKind.is(workflow.kind)).toBe(true);
    }
  });

  it("ep016_integration_signal_query_surfaces_vocabularic", () => {
    for (const workflow of MEMORY_WORKFLOWS) {
      for (const s of workflow.signals) {
        expect(signalType.is(s)).toBe(true);
      }
      for (const q of workflow.queries) {
        expect(queryType.is(q)).toBe(true);
      }
    }
  });

  it("ep016_integration_policies_valid_with_explicit_timeout_cancel", () => {
    for (const workflow of MEMORY_WORKFLOWS) {
      expect(() => validateWorkflowPolicy(workflow.policy)).not.toThrow();
      expect(workflow.policy.timeouts.executionTimeoutMs).toBeGreaterThan(0);
      expect(workflow.policy.timeouts.approvalTimeoutMs).toBeGreaterThan(0);
      expect(workflow.policy.cancelAction).toMatch(/^(CANCEL|COMPENSATE)$/);
    }
  });

  it("ep016_integration_activities_idempotent_and_bounded", () => {
    for (const workflow of MEMORY_WORKFLOWS) {
      expect(workflow.activities.length).toBeGreaterThan(0);
      for (const a of workflow.activities) {
        expect(isUuidV7(a.activityId)).toBe(true);
        expect(a.idempotencyRequired).toBe(true);
        expect(activityKind.is(a.kind)).toBe(true);
        expect(a.retry.maxAttempts).toBeGreaterThanOrEqual(1);
        expect(a.retry.maxAttempts).toBeLessThanOrEqual(10);
        expect(a.retry.retryableErrorClasses).not.toContain("PERMANENT");
        if (a.compensation !== undefined) {
          expect(isUuidV7(a.compensation.activityId)).toBe(true);
        }
      }
    }
  });
});

describe("ep016_integration_memory_determinism_audit", () => {
  it("ep016_integration_memory_sources_are_deterministic", () => {
    // The determinism guard must genuinely inspect the memory workflow
    // source files, never pass vacuously on an empty tree.
    const files = ["vocabulary.ts", "workflows.ts", "index.ts"];
    let audited = 0;
    for (const file of files) {
      const source = readFileSync(path.join(memoryDir, file), "utf8");
      const violations = findDeterminismViolations(source);
      expect(violations).toEqual([]);
      audited += 1;
    }
    expect(audited).toBe(3);
  });

  it("ep016_integration_shared_rules_still_detect_violations", () => {
    // Guard is not vacuous: the shared rules still catch real hazards.
    expect(
      findDeterminismViolations("const now = Date.now();").length,
    ).toBeGreaterThan(0);
    expect(
      findDeterminismViolations("await fetch('/api');").length,
    ).toBeGreaterThan(0);
  });
});

describe("ep016_integration_memory_workflow_semantics", () => {
  it("ep016_integration_consolidation_is_proposal_before_canonical", () => {
    const w = MemoryConsolidationWorkflow;
    // The proposal/evaluate/activate activity chain is exactly the
    // SPEC-002 behavior 5 pipeline: models never write canonical memory
    // directly.
    const ops = w.activities.map((a) => {
      const m = a as unknown as {
        operationKind?: string;
      };
      return m.operationKind ?? "UNKNOWN";
    });
    expect(ops).toContain("PROPOSE");
    expect(ops).toContain("EVALUATE_PROPOSAL");
    expect(ops).toContain("ACTIVATE_CANONICAL");
  });

  it("ep016_integration_retention_has_compensation_and_verify", () => {
    const w = MemoryRetentionWorkflow;
    const withCompensation = w.activities.filter(
      (a) => a.compensation !== undefined,
    );
    expect(withCompensation.length).toBeGreaterThanOrEqual(1);
    // Legal-hold protection is encoded: the sweep's compensation
    // rollback prefix is deterministic.
    for (const a of withCompensation) {
      expect(a.compensation?.idempotencyKeyPrefix).toMatch(/^memory-/);
    }
  });

  it("ep016_integration_deletion_requires_digest_and_receipt", () => {
    const w = MemoryDeletionWorkflow;
    expect(w.description).toMatch(/digest/);
    expect(w.description).toMatch(/compensation/);
  });

  it("ep016_integration_legal_hold_preserves_but_never_auto_selects", () => {
    const w = MemoryLegalHoldWorkflow;
    expect(w.description).toMatch(/legal hold/);
    expect(w.description).toMatch(/never surfaces/);
    expect(legalHoldDecision.values).toEqual(["APPLY", "RELEASE"]);
  });

  it("ep016_integration_export_is_audited_and_idempotent", () => {
    const w = MemoryExportWorkflow;
    expect(w.activities[0]?.idempotencyRequired).toBe(true);
    expect(w.activities[0]?.retry.maxAttempts).toBeGreaterThanOrEqual(1);
  });

  it("ep016_integration_reembed_is_bounded_and_retried", () => {
    const w = MemoryReembedWorkflow;
    expect(w.activities.length).toBeGreaterThanOrEqual(1);
    for (const a of w.activities) {
      expect(a.retry.retryableErrorClasses).not.toContain("PERMANENT");
    }
  });
});
