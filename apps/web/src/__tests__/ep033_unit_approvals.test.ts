import { describe, expect, it } from "vitest";
import {
  ApprovalAction,
  ApprovalCard,
  FourEyesRecord,
  APPROVAL_ACTIONS,
  APPROVAL_STATES,
} from "../contracts/approval-center";
import { APPROVAL_CLASSES } from "../contracts/command";
import { ErrorCode, Spec006Error } from "../contracts/errors";

function uuid(n: number): string {
  return `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
}

function cardWire(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    approval_id: uuid(30),
    action_id: uuid(31),
    capability_id: "sentinel.contain.quarantine",
    approval_class: "FOUR_EYES",
    risk: "R3",
    action_label: "Quarantine host",
    target: "host:edge-01",
    external_effects: "Blocks network egress for the host",
    cost: "0",
    reversibility: "REVERSIBLE",
    requester_id: uuid(32),
    expires_at_unix_s: 1_800_000_000,
    correlation: uuid(33),
    ...overrides,
  };
}

describe("ep033_unit_approval_center", () => {
  it("constructs an approval card with full SPEC-017 disclosure fields", () => {
    const card = ApprovalCard.fromWire(cardWire());
    expect(card.action_label).toBe("Quarantine host");
    expect(card.target).toBe("host:edge-01");
    expect(card.external_effects).toContain("egress");
    expect(card.reversibility).toBe("REVERSIBLE");
    expect(card.requester_id).toBe(uuid(32));
    expect(card.isExpired(1_900_000_000)).toBe(true);
    expect(card.isExpired(1_700_000_000)).toBe(false);
  });

  it("preserves the canonical approval classes distinctly", () => {
    expect([...APPROVAL_CLASSES]).toEqual([
      "NONE",
      "POLICY",
      "HUMAN",
      "STRONG_HUMAN",
      "FOUR_EYES",
    ]);
  });

  it("never collapses approval class into a boolean (class preserved verbatim)", () => {
    const human = ApprovalCard.fromWire(cardWire({ approval_class: "HUMAN" }));
    const strong = ApprovalCard.fromWire(cardWire({ approval_class: "STRONG_HUMAN" }));
    const fourEyes = ApprovalCard.fromWire(cardWire({ approval_class: "FOUR_EYES" }));
    expect(human.approval_class).toBe("HUMAN");
    expect(strong.approval_class).toBe("STRONG_HUMAN");
    expect(fourEyes.approval_class).toBe("FOUR_EYES");
    expect(fourEyes.requiresTwoPrincipals()).toBe(true);
    expect(human.requiresTwoPrincipals()).toBe(false);
  });

  it("rejects unknown approval classes", () => {
    expect(() => ApprovalCard.fromWire(cardWire({ approval_class: "GENERIC_APPROVE" }))).toThrowError(
      Spec006Error,
    );
  });

  it("four-eyes requires two distinct principals", () => {
    const fourEyes = new FourEyesRecord(uuid(30));
    fourEyes.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 1));
    expect(fourEyes.isSatisfied(uuid(32))).toBe(false);

    // Same principal approves again: still not satisfied (one account
    // can never satisfy FOUR_EYES).
    expect(() => fourEyes.requireNewPrincipal(uuid(40))).toThrowError(Spec006Error);
    fourEyes.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 2));
    expect(fourEyes.isSatisfied(uuid(32))).toBe(false);
    expect(fourEyes.distinctApprovers()).toEqual([uuid(40)]);
  });

  it("four-eyes becomes satisfied with two distinct principals", () => {
    const fourEyes = new FourEyesRecord(uuid(30));
    fourEyes.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 1));
    fourEyes.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(41), 2));
    expect(fourEyes.isSatisfied(uuid(32))).toBe(true);
    expect([...fourEyes.distinctApprovers()].sort()).toEqual([uuid(40), uuid(41)].sort());
  });

  it("excludes the requester from satisfying four-eyes", () => {
    const fourEyes = new FourEyesRecord(uuid(30));
    fourEyes.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(32), 1)); // requester
    fourEyes.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 2));
    // Requester + one other is only one distinct non-requester.
    expect(fourEyes.isSatisfied(uuid(32))).toBe(false);
  });

  it("refuses actions targeting a different approval", () => {
    const fourEyes = new FourEyesRecord(uuid(30));
    expect(() => fourEyes.apply(ApprovalAction.record(uuid(99), "APPROVE", uuid(40), 1))).toThrowError(
      Spec006Error,
    );
  });

  it("exposes the canonical approval states and actions", () => {
    expect([...APPROVAL_STATES]).toEqual(["PENDING", "APPROVED", "DENIED", "EXPIRED", "REVOKED"]);
    expect([...APPROVAL_ACTIONS]).toEqual(["APPROVE", "DENY"]);
  });

  it("rejects unknown fields in approval card wire input", () => {
    expect(() =>
      ApprovalCard.fromWire(cardWire({ auto_approve: true })),
    ).toThrowError(Spec006Error);
  });
});
