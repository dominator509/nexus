/**
 * EP-042 M2 rollback precondition proofs (SPEC-016 behavior 6;
 * SPEC-024 restore).
 *
 * Rollback plan missing -> denied. Rollback target incompatible ->
 * denied. Receipt malformed -> denied. Receipt from wrong version /
 * install -> denied. Drill not run -> not proven. Drill verified ->
 * proven.
 *
 * ROLLBACK RECEIPT EXISTS != ROLLBACK PROVEN.
 */

import { describe, expect, it } from "vitest";
import {
  ReleaseError,
  evaluateRollbackPreconditions,
  parseRollbackReceipt,
  parseUpdatePlan,
} from "@nexus/setup";
import { drillWire, fixtureManifest, receiptWire } from "./fixtures";

describe("ep042_unit rollback preconditions", () => {
  const plan = parseUpdatePlan(planWireForRollback());
  const manifest = fixtureManifest();
  const receipt = parseRollbackReceipt(receiptWire());

  function planWireForRollback(): Record<string, unknown> {
    const wire = {
      schema_version: 1,
      plan_id: "plan-1",
      release_id: "release-1",
      from_version: "1.0.0",
      to_version: "1.1.0",
      channel: "STABLE",
      steps: [
        { order: 1, kind: "BACKUP", description: "backup" },
        { order: 2, kind: "MIGRATE", description: "migrate" },
        { order: 3, kind: "CANARY", description: "canary" },
        { order: 4, kind: "OBSERVE", description: "observe" },
        { order: 5, kind: "ROLLBACK", description: "rollback contingency" },
      ],
      idempotency_key: "idem-1",
      correlation_id: "corr-1",
      created_at: "2026-08-25T00:00:00Z",
      state: "PLANNED",
    };
    return wire;
  }

  it("ep042_unit_rollback_requires_plan_rollback_path", () => {
    const wire = structuredClone(planWireForRollback());
    const steps = wire["steps"] as Array<Record<string, unknown>>;
    const withoutRollback = steps.filter((step) => step.kind !== "ROLLBACK");
    wire["steps"] = withoutRollback;
    const noRollbackPlan = parseUpdatePlan(wire);
    const verdict = evaluateRollbackPreconditions(
      noRollbackPlan,
      manifest.compatibility,
      manifest.components,
      receipt,
      drillWire(),
    );
    expect(verdict.decision).toBe("DENIED");
    expect(verdict.reasons.join(" ")).toContain("no rollback path");
  });

  it("ep042_unit_rollback_rejects_missing_receipt", () => {
    const verdict = evaluateRollbackPreconditions(
      plan,
      manifest.compatibility,
      manifest.components,
      undefined,
      drillWire(),
    );
    expect(verdict.decision).toBe("DENIED");
    expect(verdict.reasons.join(" ")).toContain("receipt missing");
  });

  it("ep042_unit_rollback_rejects_receipt_wrong_plan_ref", () => {
    const wrong = parseRollbackReceipt({
      ...receiptWire(),
      update_plan_ref: "plan-999",
    });
    const verdict = evaluateRollbackPreconditions(
      plan,
      manifest.compatibility,
      manifest.components,
      wrong,
      drillWire(),
    );
    expect(verdict.decision).toBe("DENIED");
    expect(verdict.reasons.join(" ")).toContain("references plan plan-999");
  });

  it("ep042_unit_rollback_rejects_receipt_wrong_versions", () => {
    const wrong = parseRollbackReceipt({
      ...receiptWire(),
      from_version: "9.9.9",
      to_version: "8.8.8",
    });
    const verdict = evaluateRollbackPreconditions(
      plan,
      manifest.compatibility,
      manifest.components,
      wrong,
      drillWire(),
    );
    expect(verdict.decision).toBe("DENIED");
  });

  it("ep042_unit_rollback_rejects_receipt_without_backup_ref", () => {
    // The parser fails closed: a receipt without a valid backup_ref
    // cannot be constructed through the typed boundary.
    const wire = structuredClone(receiptWire());
    wire["backup_ref"] = { backend: "", key: "" };
    expect(() => parseRollbackReceipt(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_rollback_receipt_exists_not_proven_without_drill", () => {
    const verdict = evaluateRollbackPreconditions(
      plan,
      manifest.compatibility,
      manifest.components,
      receipt,
      undefined,
    );
    expect(verdict.decision).toBe("NOT_PROVEN");
    expect(verdict.reasons.join(" ")).toContain("drill has not been run");
  });

  it("ep042_unit_rollback_drill_not_run_not_proven", () => {
    const verdict = evaluateRollbackPreconditions(
      plan,
      manifest.compatibility,
      manifest.components,
      receipt,
      { ...drillWire(), outcome: "NOT_RUN" },
    );
    expect(verdict.decision).toBe("NOT_PROVEN");
  });

  it("ep042_unit_rollback_drill_failed_denied", () => {
    const verdict = evaluateRollbackPreconditions(
      plan,
      manifest.compatibility,
      manifest.components,
      receipt,
      { ...drillWire(), outcome: "FAILED" },
    );
    expect(verdict.decision).toBe("DENIED");
    expect(verdict.reasons.join(" ")).toContain("drill failed");
  });

  it("ep042_unit_rollback_drill_wrong_install_denied", () => {
    const verdict = evaluateRollbackPreconditions(
      plan,
      manifest.compatibility,
      manifest.components,
      receipt,
      { ...drillWire(), install_id: "other-install" },
    );
    expect(verdict.decision).toBe("DENIED");
  });

  it("ep042_unit_rollback_all_preconditions_met_proven", () => {
    const verdict = evaluateRollbackPreconditions(
      plan,
      manifest.compatibility,
      manifest.components,
      receipt,
      drillWire(),
    );
    expect(verdict.decision).toBe("PROVEN");
  });

  it("ep042_unit_rollback_rejects_target_not_older", () => {
    const wire = structuredClone(planWireForRollback());
    wire["from_version"] = "2.0.0";
    wire["to_version"] = "1.0.0";
    const upgradePlan = parseUpdatePlan(wire);
    const verdict = evaluateRollbackPreconditions(
      upgradePlan,
      manifest.compatibility,
      manifest.components,
      parseRollbackReceipt({
        ...receiptWire(),
        update_plan_ref: "plan-1",
        from_version: "1.0.0",
        to_version: "2.0.0",
      }),
      drillWire(),
    );
    expect(verdict.decision).toBe("DENIED");
  });
});
