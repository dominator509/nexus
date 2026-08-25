/**
 * EP-042 M2 backup-before-update policy proofs (SPEC-016 behavior 6).
 *
 * Backup requirement missing -> denied. Backup requested but not
 * completed -> denied. Backup proof malformed -> denied. Backup from
 * wrong install id -> denied. Completed + verified -> approved.
 *
 * BACKUP REQUESTED != BACKUP COMPLETED.
 */

import { describe, expect, it } from "vitest";
import { evaluateBackupRequirement, parseUpdatePlan } from "@nexus/setup";
import { backupProofWire, planWire } from "./fixtures";

describe("ep042_unit backup policy", () => {
  const plan = parseUpdatePlan(planWire());

  it("ep042_unit_backup_requires_plan_backup_first_step", () => {
    const wire = structuredClone(planWire());
    const steps = wire["steps"] as Array<Record<string, unknown>>;
    steps[0] = { order: 1, kind: "MIGRATE", description: "no backup" };
    // The plan parser fails closed: no plan without a backup first step
    // can exist (backup-before-update).
    expect(() => parseUpdatePlan(wire)).toThrow();
  });

  it("ep042_unit_backup_requested_not_completed_denied", () => {
    const verdict = evaluateBackupRequirement(plan, undefined, "install-1");
    expect(verdict.decision).toBe("DENIED");
    expect(verdict.state).toBe("REQUESTED");
    expect(verdict.reasons.join(" ")).toContain("not completed");
  });

  it("ep042_unit_backup_proof_malformed_denied", () => {
    const verdict = evaluateBackupRequirement(
      plan,
      { ...backupProofWire(), backup_id: "" },
      "install-1",
    );
    expect(verdict.decision).toBe("DENIED");
    expect(verdict.reasons.join(" ")).toContain("no backup_id");
  });

  it("ep042_unit_backup_proof_bad_digest_denied", () => {
    const verdict = evaluateBackupRequirement(
      plan,
      { ...backupProofWire(), digest: "not-a-digest" },
      "install-1",
    );
    expect(verdict.decision).toBe("DENIED");
  });

  it("ep042_unit_backup_proof_wrong_install_id_denied", () => {
    const verdict = evaluateBackupRequirement(
      plan,
      { ...backupProofWire(), install_id: "install-999" },
      "install-1",
    );
    expect(verdict.decision).toBe("DENIED");
    expect(verdict.reasons.join(" ")).toContain("does not match expected");
  });

  it("ep042_unit_backup_proof_not_verified_denied", () => {
    const verdict = evaluateBackupRequirement(
      plan,
      { ...backupProofWire(), state: "UNVERIFIED" },
      "install-1",
    );
    expect(verdict.decision).toBe("DENIED");
    expect(verdict.reasons.join(" ")).toContain("not VERIFIED");
  });

  it("ep042_unit_backup_completed_and_verified_approved", () => {
    const verdict = evaluateBackupRequirement(
      plan,
      backupProofWire(),
      "install-1",
    );
    expect(verdict.decision).toBe("APPROVED");
    expect(verdict.state).toBe("COMPLETED");
  });

  it("ep042_unit_backup_requested_not_completed_state_ladder", () => {
    // BACKUP REQUESTED != BACKUP COMPLETED: the state ladder is visible.
    const requested = evaluateBackupRequirement(plan, undefined, "install-1");
    expect(requested.state).toBe("REQUESTED");
    const completed = evaluateBackupRequirement(
      plan,
      backupProofWire(),
      "install-1",
    );
    expect(completed.state).toBe("COMPLETED");
  });

  it("ep042_unit_backup_rejects_missing_completed_at", () => {
    const verdict = evaluateBackupRequirement(
      plan,
      { ...backupProofWire(), completed_at: "" },
      "install-1",
    );
    expect(verdict.decision).toBe("DENIED");
    expect(verdict.reasons.join(" ")).toContain("no completed_at");
  });
});
