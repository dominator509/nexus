/**
 * EP-042 M2 canary / manual promotion gate proofs (SPEC-016 behavior 6, 7).
 *
 * Canary ring defined != rollout approved. Ring health missing ->
 * denied. Ring failure -> denied. Unknown ring -> denied. Manual
 * approval missing -> denied. Promotion never automatic. Promotion does
 * not bypass compatibility, backup, or rollback preconditions.
 *
 * AUD-070: promotion authority is NEVER reducible to a nonempty
 * approval reference string. An approval is real only when a full
 * ManualPromotion record:
 *   - is cryptographically signed by a pinned Ed25519 key (real
 *     signature verification over the canonical payload),
 *   - was approved by an authorized approver (policy lookup),
 *   - is within its validity window (expiry; stale/future denied),
 *   - was approved by an identity different from the requester
 *     (requester/approver separation),
 *   - binds to the exact canary ring and release (record binding),
 *   - is in APPROVED_MANUAL_ONLY state with an exact manual command.
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
  type PromotionApprovalPolicy,
} from "@nexus/setup";
import {
  approvalTestKey,
  backupProofWire,
  drillWire,
  fixtureManifest,
  planWire,
  promotionWire,
  receiptWire,
  ringWire,
  signedPromotionWire,
  type ApprovalTestKey,
} from "./fixtures";

function policyFor(
  key: ApprovalTestKey,
  overrides: Partial<PromotionApprovalPolicy> = {},
): PromotionApprovalPolicy {
  return {
    authorizedApprovers: new Set(["operator-1"]),
    approverPublicKey: key.publicKeyRaw,
    validityMinutes: 60,
    requesterId: "requester-1",
    nowMs: Date.parse("2026-08-25T02:30:00Z"),
    ...overrides,
  };
}

describe("ep042_unit canary / promotion gate", () => {
  const plan = parseUpdatePlan(planWire());
  const manifest = fixtureManifest();

  it("ep042_unit_canary_ring_defined_not_rollout_approved", async () => {
    const ring = parseCanaryRing(ringWire());
    const key = await approvalTestKey();
    const verdict = await evaluatePromotionGate(
      ring,
      parseManualPromotion(await signedPromotionWire(key)),
      policyFor(key),
    );
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.requiresHuman).toBe(true);
  });

  it("ep042_unit_canary_ready_without_evidence_denied", async () => {
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
    });
    const key = await approvalTestKey();
    const verdict = await evaluatePromotionGate(
      ring,
      parseManualPromotion(await signedPromotionWire(key)),
      policyFor(key),
    );
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("evidence is missing");
  });

  it("ep042_unit_canary_failure_verdict_denied", async () => {
    const ring = parseCanaryRing({ ...ringWire(), verdict: "ROLLBACK" });
    const key = await approvalTestKey();
    const verdict = await evaluatePromotionGate(
      ring,
      parseManualPromotion(await signedPromotionWire(key)),
      policyFor(key),
    );
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("ROLLBACK");
  });

  it("ep042_unit_canary_missing_health_criterion_denied", async () => {
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
    const key = await approvalTestKey();
    const verdict = await evaluatePromotionGate(
      literal,
      parseManualPromotion(await signedPromotionWire(key)),
      policyFor(key),
    );
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

  it("ep042_unit_canary_ready_with_evidence_requires_approval", async () => {
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const key = await approvalTestKey();
    const verdict = await evaluatePromotionGate(
      ring,
      undefined,
      policyFor(key),
    );
    expect(verdict.decision).toBe("AWAITING_HUMAN_APPROVAL");
  });

  it("ep042_unit_canary_ready_with_signed_approval_manual_only", async () => {
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const key = await approvalTestKey();
    const promotion = parseManualPromotion(
      await signedPromotionWire(key),
    );
    const verdict = await evaluatePromotionGate(ring, promotion, policyFor(key));
    expect(verdict.decision).toBe("APPROVED_MANUAL_ONLY");
  });

  it("ep042_unit_approval_bare_string_is_not_authority", async () => {
    // AUD-070: a promotion whose approval_ref is nonempty but whose
    // signature is absent/placeholder is NOT authority. The old gate
    // approved any nonblank approvalRef; the fixed gate requires a real
    // verified signature.
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const key = await approvalTestKey();
    const unsignedWire = promotionWire();
    // Parse fails closed: an approval record without a signature cannot
    // even be constructed through the typed boundary.
    expect(() => parseManualPromotion(unsignedWire)).toThrow();
  });

  it("ep042_unit_approval_unauthorized_approver_denied", async () => {
    // Policy lookup: an approver outside the authorized set is denied
    // even with a valid signature.
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const key = await approvalTestKey();
    const promotion = parseManualPromotion(
      await signedPromotionWire(key, { approver: "intruder-9" }),
    );
    const verdict = await evaluatePromotionGate(ring, promotion, policyFor(key));
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("not authorized");
  });

  it("ep042_unit_approval_requester_approver_separation", async () => {
    // Requester/approver separation: the same identity cannot both
    // request and approve the promotion.
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const key = await approvalTestKey();
    const promotion = parseManualPromotion(
      await signedPromotionWire(key, { approver: "requester-1" }),
    );
    const verdict = await evaluatePromotionGate(
      ring,
      promotion,
      policyFor(key, { authorizedApprovers: new Set(["requester-1"]) }),
    );
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("different identities");
  });

  it("ep042_unit_approval_expired_denied", async () => {
    // Expiry: an approval older than the validity window is denied.
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const key = await approvalTestKey();
    const promotion = parseManualPromotion(
      await signedPromotionWire(key, {
        approved_at: "2026-08-25T01:00:00Z",
      }),
    );
    const verdict = await evaluatePromotionGate(ring, promotion, policyFor(key));
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("expired");
  });

  it("ep042_unit_approval_future_dated_denied", async () => {
    // A future-dated approval cannot be authority now.
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const key = await approvalTestKey();
    const promotion = parseManualPromotion(
      await signedPromotionWire(key, {
        approved_at: "2026-08-25T03:00:00Z",
      }),
    );
    const verdict = await evaluatePromotionGate(ring, promotion, policyFor(key));
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("future");
  });

  it("ep042_unit_approval_tampered_signature_denied", async () => {
    // Real signature: tampering any authority field after signing
    // invalidates the approval even when the approver is authorized.
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const key = await approvalTestKey();
    const wire = await signedPromotionWire(key, {
      exact_manual_command: "sh scripts/deploy.sh --release 9.9.9",
    });
    // Tamper AFTER signing: the wire's signature no longer matches the
    // canonical payload of the mutated record.
    const promotion = parseManualPromotion({
      ...wire,
      exact_manual_command: "sh scripts/deploy.sh --release 1.0.0-evil",
    });
    const verdict = await evaluatePromotionGate(ring, promotion, policyFor(key));
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("signature");
  });

  it("ep042_unit_approval_wrong_key_denied", async () => {
    // The pinned key is policy: a signature from a different key fails.
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const signerKey = await approvalTestKey();
    const pinnedKey = await approvalTestKey();
    const promotion = parseManualPromotion(
      await signedPromotionWire(signerKey),
    );
    const verdict = await evaluatePromotionGate(
      ring,
      promotion,
      policyFor(pinnedKey),
    );
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("signature");
  });

  it("ep042_unit_approval_record_binding_denied", async () => {
    // Record binding: an approval for a different ring cannot promote
    // this ring.
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const key = await approvalTestKey();
    const promotion = parseManualPromotion(
      await signedPromotionWire(key, { canary_ring_ref: "ring-OTHER" }),
    );
    const verdict = await evaluatePromotionGate(ring, promotion, policyFor(key));
    expect(verdict.decision).toBe("LOCKED");
    expect(verdict.reasons.join(" ")).toContain("ring");
  });

  it("ep042_unit_promotion_never_automatic", async () => {
    const ring = parseCanaryRing({
      ...ringWire(),
      verdict: "READY_TO_PROMOTE",
      evidence_ref: "evidence/run-1.json",
    });
    const key = await approvalTestKey();
    const verdict = await evaluatePromotionGate(
      ring,
      parseManualPromotion(await signedPromotionWire(key)),
      policyFor(key),
    );
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
    const key = await approvalTestKey();
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
      promotion: parseManualPromotion(await signedPromotionWire(key)),
      policy: policyFor(key),
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
    const key = await approvalTestKey();
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
      promotion: parseManualPromotion(await signedPromotionWire(key)),
      policy: policyFor(key),
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
    const key = await approvalTestKey();
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
      promotion: parseManualPromotion(await signedPromotionWire(key)),
      policy: policyFor(key),
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
    const key = await approvalTestKey();
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
      promotion: parseManualPromotion(await signedPromotionWire(key)),
      policy: policyFor(key),
    });
    expect(verdict.decision).toBe("APPROVED_MANUAL_ONLY");
  });

  it("ep042_unit_manual_promotion_requires_approval_ref", async () => {
    const key = await approvalTestKey();
    const wire = await signedPromotionWire(key);
    wire["approval_ref"] = "";
    expect(() => parseManualPromotion(wire)).toThrow();
  });

  it("ep042_unit_manual_promotion_requires_signature", () => {
    // AUD-070: an approval record without a signature is not an
    // approval record. The parser fails closed.
    expect(() => parseManualPromotion(promotionWire())).toThrow();
  });

  it("ep042_unit_manual_promotion_never_deploys", async () => {
    const key = await approvalTestKey();
    const promotion = parseManualPromotion(
      await signedPromotionWire(key),
    );
    expect(promotionNeverDeploys(promotion)).toBe(true);
    expect(promotion.state).toBe("APPROVED_MANUAL_ONLY");
    // The record is a decision record carrying an exact manual command;
    // it contains no executor and cannot perform deployment.
    expect(promotion.exact_manual_command.length).toBeGreaterThan(0);
    expect(promotion.signature.value_b64.length).toBeGreaterThan(0);
  });

  it("ep042_unit_promotion_rejects_bad_ring_vocabulary", () => {
    const wire = { ...ringWire(), verdict: "PROMOTED" };
    expect(() => parseCanaryRing(wire)).toThrow();
  });
});
