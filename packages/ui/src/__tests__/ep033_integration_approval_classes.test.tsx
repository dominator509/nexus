/**
 * EP-033 M3 integration: ApprovalCardView through REAL React rendering.
 *
 * Proves approval classes survive to rendered output verbatim and that
 * FOUR_EYES surfaces the two-principal requirement (directive L/M).
 */

import { describe, expect, it } from "vitest";
import { renderToString } from "react-dom/server";
import { ApprovalCard } from "@nexus/web";
import { ApprovalCardView } from "../components/approval-card-view";

function uuid(n: number): string {
  return `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
}

function card(approvalClass: string): ApprovalCard {
  return ApprovalCard.fromWire({
    approval_id: uuid(30),
    action_id: uuid(31),
    capability_id: "sentinel.contain.quarantine",
    approval_class: approvalClass,
    risk: "R3",
    action_label: "Quarantine host",
    target: "host:edge-01",
    external_effects: "Blocks network egress",
    cost: "0",
    reversibility: "REVERSIBLE",
    requester_id: uuid(32),
    expires_at_unix_s: 1_800_000_000,
    correlation: uuid(33),
  });
}

describe("ep033_integration_approval_classes", () => {
  it("renders the approval class verbatim, never a boolean", () => {
    for (const klass of ["POLICY", "HUMAN", "STRONG_HUMAN", "FOUR_EYES"]) {
      const html = renderToString(
        <ApprovalCardView card={card(klass)} state="PENDING" distinctApprovers={[]} />,
      );
      expect(html).toContain(`data-approval-class="${klass}"`);
      expect(html).toContain(klass);
    }
  });

  it("renders the full SPEC-017 disclosure fields", () => {
    const html = renderToString(
      <ApprovalCardView card={card("HUMAN")} state="PENDING" distinctApprovers={[]} />,
    );
    expect(html).toContain("Quarantine host");
    expect(html).toContain("host:edge-01");
    expect(html).toContain("Blocks network egress");
    expect(html).toContain("REVERSIBLE");
    expect(html).toContain(uuid(32));
  });

  it("renders the four-eyes two-principal requirement only for FOUR_EYES", () => {
    const fourEyes = renderToString(
      <ApprovalCardView card={card("FOUR_EYES")} state="PENDING" distinctApprovers={[]} />,
    );
    expect(fourEyes).toContain('data-four-eyes="true"');
    expect(fourEyes).toContain("two distinct principals");

    const human = renderToString(
      <ApprovalCardView card={card("HUMAN")} state="PENDING" distinctApprovers={[]} />,
    );
    expect(human).not.toContain('data-four-eyes="true"');
  });

  it("shows distinct approver progress for four-eyes", () => {
    const html = renderToString(
      <ApprovalCardView
        card={card("FOUR_EYES")}
        state="PENDING"
        distinctApprovers={[uuid(40)]}
      />,
    );
    expect(html).toContain("Approvals recorded: 1.");
  });

  it("renders the live state as a status region", () => {
    const html = renderToString(
      <ApprovalCardView card={card("HUMAN")} state="APPROVED" distinctApprovers={[]} />,
    );
    expect(html).toContain('role="status"');
    expect(html).toContain("APPROVED");
  });
});
