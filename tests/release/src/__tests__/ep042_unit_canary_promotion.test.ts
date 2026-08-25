/**
 * EP-042 M2 canary / manual promotion gate proofs (SPEC-016 behavior 6, 7).
 *
 * Canary ring defined != rollout approved. Ring health missing ->
 * denied. Ring failure -> denied. Unknown ring -> denied. Manual
 * approval missing -> denied. Promotion never automatic. Promotion does
 * not bypass compatibility, backup, or rollback preconditions.
 *
 * MANUAL PROMOTION EXISTS != AUTOMATIC DEPLOYMENT.
 */

import { describe, expect, it } from "vitest";
import {
  evaluateFullPromotionGate,
  evaluatePromotionGate,
  parseCanaryRing,
  parseManualPromotion,
  parseRollbackReceipt,
  parseUpdatePlan,
  promotionNeverDeploys,
} from "@nexus/setup";
import {
  backupProofWire,
  drillWire,
  fixtureManifest,
  planWire,
  promotionWire,
  receiptWire,
  ringWire,
} from "./fixtures";

describe("ep042_unit canary / promotion gate", () => {
  const plan = parseUpdatePlan(planWire());
  const manifest = fixtureManifest();

  it("ep042_unit_canary_ring_defined_not_rollout_approved", () => {
    const ring = parseCanaryRing(ringWire());
    const verdict = evaluatePromotionGate(ring, "approval-42");
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.requiresHuman).toBe(true);
  });

  it("ep042_unit_canary_ready_without_evidence_denied", () => {
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
    });
    const verdict = evaluatePromotionGate(ring, "approval-42");
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("evidence is missing");
  });

  it("ep042_unit_canary_failure_verdict_denied", () => {
    const ring = parseCanaryRing({ ...ringWire(), verdict: "ROLLBACK" });
    const verdict = evaluatePromotionGate(ring, "approval-42");
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("ROLLBACK");
  });

  it("ep042_unit_canary_missing_health_criterion_denied", () => {
    // Parser fails closed: a ring without a health criterion cannot be
    // constructed through the typed boundary.
    expect(() => parseCanaryRing(ringWireWithEmptyCriterion())).toThrow();
    // Defense-in-depth: the gate re-checks the criterion on any ring
    // object, even one that bypassed the parser (e.g. legacy store data).
    const literal = {
      ...ringWire(),
      verdict: "READY_TO_PROMOTE" as const,
      evidence_ref: "evidence/run-1.json",
      health_criterion: "",
    } as unknown as ReturnType<typeof parseCanaryRing>;
    const verdict = evaluatePromotionGate(literal, "approval-42");
    expect(verdict.decision).toBe("LOCKED");
  });

  function ringWireWithEmptyCriterion(): Record<string, unknown> {
    return {
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
      health_criterion: "",
    };
  }

  it("ep042_unit_canary_ready_with_evidence_requires_approval", () => {
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const verdict = evaluatePromotionGate(ring, undefined);
    expect(verdict.decision).toBe("AWAITING_HUMAN_APPROVAL");
  });

  it("ep042_unit_canary_ready_with_approval_manual_only", () => {
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const verdict = evaluatePromotionGate(ring, "approval-42");
    expect(verdict.decision).toBe("APPROVED_MANUAL_ONLY");
  });

  it("ep042_unit_promotion_never_automatic", () => {
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const verdict = evaluatePromotionGate(ring, "approval-42");
    // The only possible decisions are lock/await/approve-manual; no
    // automatic deployment decision exists in the vocabulary.
    expect([
      "LOCKED",
      "AWAITING_HUMAN_APPROVAL",
      "APPROVED_MANUAL_ONLY",
    ]).toContain(verdict.decision);
    expect(verdict.requiresHuman).toBe(true);
  });

  it("ep042_unit_promotion_unknown_ring_denied", async () => {
    const ring = parseCanaryRing({ ...ringWire(), release_id: "release-999" });
    const verdict = await evaluateFullPromotionGate({
      ring,
      releaseId: "release-1",
      plan,
      matrix: manifest.compatibility,
      components: manifest.components,
      backupProof: backupProofWire(),
      rollbackReceipt: parseRollbackReceipt(receiptWire()),
      rollbackDrill: drillWire(),
      installId: "install-1",
      approvalRef: "approval-42",
    });
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain(
      "references release release-999",
    );
  });

  it("ep042_unit_promotion_backup_precondition_not_bypassed", async () => {
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const verdict = await evaluateFullPromotionGate({
      ring,
      releaseId: "release-1",
      plan,
      matrix: manifest.compatibility,
      components: manifest.components,
      backupProof: undefined,
      rollbackReceipt: parseRollbackReceipt(receiptWire()),
      rollbackDrill: drillWire(),
      installId: "install-1",
      approvalRef: "approval-42",
    });
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("backup precondition denied");
  });

  it("ep042_unit_promotion_rollback_precondition_not_bypassed", async () => {
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const verdict = await evaluateFullPromotionGate({
      ring,
      releaseId: "release-1",
      plan,
      matrix: manifest.compatibility,
      components: manifest.components,
      backupProof: backupProofWire(),
      rollbackReceipt: undefined,
      rollbackDrill: undefined,
      installId: "install-1",
      approvalRef: "approval-42",
    });
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("rollback precondition denied");
  });

  it("ep042_unit_promotion_full_gate_approves_manual_only", async () => {
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const verdict = await evaluateFullPromotionGate({
      ring,
      releaseId: "release-1",
      plan,
      matrix: manifest.compatibility,
      components: manifest.components,
      backupProof: backupProofWire(),
      rollbackReceipt: parseRollbackReceipt(receiptWire()),
      rollbackDrill: drillWire(),
      installId: "install-1",
      approvalRef: "approval-42",
    });
    expect(verdict.decision).toBe("APPROVED_MANUAL_ONLY");
  });

  it("ep042_unit_manual_promotion_requires_approval_ref", () => {
    const wire = structuredClone(promotionWire());
    wire["approval_ref"] = "";
    expect(() => parseManualPromotion(wire)).toThrow();
  });

  it("ep042_unit_manual_promotion_never_deploys", () => {
    const promotion = parseManualPromotion(promotionWire());
    expect(promotionNeverDeploys(promotion)).toBe(true);
    expect(promotion.state).toBe("APPROVED_MANUAL_ONLY");
    // The record is a decision record carrying an exact manual command;
    // it contains no executor and cannot perform deployment.
    expect(promotion.exact_manual_command.length).toBeGreaterThan(0);
  });

  it("ep042_unit_promotion_rejects_bad_ring_vocabulary", () => {
    const wire = { ...ringWire(), verdict: "PROMOTED" };
    expect(() => parseCanaryRing(wire)).toThrow();
  });
});
