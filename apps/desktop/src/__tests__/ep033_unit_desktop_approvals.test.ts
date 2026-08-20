import { describe, expect, it } from "vitest";
import { ApprovalAction, ApprovalCard, ErrorCode, Spec006Error } from "@nexus/web";
import { DesktopApprovalFlow } from "../approvals";

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
    external_effects: "Blocks network egress",
    cost: "0",
    reversibility: "REVERSIBLE",
    requester_id: uuid(32),
    expires_at_unix_s: 1_800_000_000,
    correlation: uuid(33),
    ...overrides,
  };
}

describe("ep033_unit_desktop_approval_flow", () => {
  it("approves a single-principal class with one distinct approval", () => {
    const card = ApprovalCard.fromWire(cardWire({ approval_class: "HUMAN" }));
    const flow = new DesktopApprovalFlow(card);
    const state = flow.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 1), 1_700_000_000);
    expect(state).toBe("APPROVED");
  });

  it("denies when the requester approves their own action", () => {
    const card = ApprovalCard.fromWire(cardWire({ approval_class: "HUMAN" }));
    const flow = new DesktopApprovalFlow(card);
    try {
      flow.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(32), 1), 1_700_000_000);
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Policy);
    }
  });

  it("four-eyes is not satisfied by one principal even with repeated clicks", () => {
    const card = ApprovalCard.fromWire(cardWire());
    const flow = new DesktopApprovalFlow(card);
    flow.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 1), 1_700_000_000);
    expect(flow.state).toBe("PENDING");
    // A duplicate principal is a Conflict, never a second approver:
    // one account can never satisfy FOUR_EYES.
    expect(() =>
      flow.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 2), 1_700_000_001),
    ).toThrowError(Spec006Error);
    expect(flow.state).toBe("PENDING");
    expect(flow.progression().distinctApprovers).toEqual([uuid(40)]);
    expect(flow.progression().satisfied).toBe(false);
  });

  it("four-eyes becomes APPROVED with two distinct principals", () => {
    const card = ApprovalCard.fromWire(cardWire());
    const flow = new DesktopApprovalFlow(card);
    flow.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 1), 1_700_000_000);
    flow.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(41), 2), 1_700_000_001);
    expect(flow.state).toBe("APPROVED");
    expect(flow.progression().satisfied).toBe(true);
  });

  it("expires a pending approval after the card deadline", () => {
    const card = ApprovalCard.fromWire(cardWire());
    const flow = new DesktopApprovalFlow(card);
    expect(() =>
      flow.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 1), 1_900_000_000),
    ).toThrowError(Spec006Error);
    expect(flow.state).toBe("EXPIRED");
  });

  it("refuses actions after a terminal state", () => {
    const card = ApprovalCard.fromWire(cardWire({ approval_class: "HUMAN" }));
    const flow = new DesktopApprovalFlow(card);
    flow.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 1), 1_700_000_000);
    expect(() =>
      flow.apply(ApprovalAction.record(uuid(30), "DENY", uuid(41), 2), 1_700_000_001),
    ).toThrowError(Spec006Error);
  });

  it("revokes an approved approval", () => {
    const card = ApprovalCard.fromWire(cardWire({ approval_class: "HUMAN" }));
    const flow = new DesktopApprovalFlow(card);
    flow.apply(ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 1), 1_700_000_000);
    flow.revoke();
    expect(flow.state).toBe("REVOKED");
  });

  it("rejects actions targeting a different approval card", () => {
    const card = ApprovalCard.fromWire(cardWire());
    const flow = new DesktopApprovalFlow(card);
    expect(() =>
      flow.apply(ApprovalAction.record(uuid(99), "APPROVE", uuid(40), 1), 1_700_000_000),
    ).toThrowError(Spec006Error);
  });
});
