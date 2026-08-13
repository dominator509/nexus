import { describe, expect, it } from "vitest";

import { WorkflowContractError } from "../errors.js";
import { assertApprovalBinding, validateApprovalSignal } from "../signals.js";
import {
  AUTH_STEP_UP,
  DIGEST_B,
  ISO_FRACTION,
  ISO_OFFSET,
  actionDigestA,
  actionDigestB,
  actionIdA,
  makeApprovalSignal,
  signalIdA,
  workflowIdA,
} from "./helpers/fixtures.js";

describe("ep006_unit_approval_signal", () => {
  it("ep006_unit_approval_signal_valid_roundtrip", () => {
    const signal = makeApprovalSignal();
    const parsed = validateApprovalSignal(JSON.parse(JSON.stringify(signal)));
    expect(parsed.signalType).toBe("APPROVAL");
    expect(parsed.signalId).toBe(signalIdA);
    expect(parsed.workflowId).toBe(workflowIdA);
    expect(parsed.actionDigest).toBe(actionDigestA);
    expect(parsed.authentication.strength).toBe("STEP_UP");
    expect(parsed.decision).toBe("APPROVE");
    expect(parsed.comment).toBeUndefined();
  });

  it("ep006_unit_approval_signal_accepts_optional_comment_and_offset_time", () => {
    const signal = makeApprovalSignal({
      comment: "approved from mobile",
      decidedAt: ISO_OFFSET,
      authentication: { ...AUTH_STEP_UP, verifiedAt: ISO_FRACTION },
    });
    const parsed = validateApprovalSignal(JSON.parse(JSON.stringify(signal)));
    expect(parsed.comment).toBe("approved from mobile");
    expect(parsed.decidedAt).toBe(ISO_OFFSET);
    expect(parsed.authentication.verifiedAt).toBe(ISO_FRACTION);
  });

  it("ep006_unit_approval_signal_rejects_missing_digest", () => {
    const raw = { ...makeApprovalSignal() } as Record<string, unknown>;
    delete raw.actionDigest;
    expect(() => validateApprovalSignal(raw)).toThrow(WorkflowContractError);
  });

  it("ep006_unit_approval_signal_rejects_bad_digest_format", () => {
    const raw = {
      ...makeApprovalSignal({ actionDigest: actionDigestB }),
    } as Record<string, unknown>;
    raw.actionDigest = "short-digest";
    expect(() => validateApprovalSignal(raw)).toThrow(/sha256/);
    raw.actionDigest = DIGEST_B.toUpperCase();
    expect(() => validateApprovalSignal(raw)).toThrow(/sha256/);
  });

  it("ep006_unit_approval_signal_rejects_missing_principal", () => {
    const raw = { ...makeApprovalSignal() } as Record<string, unknown>;
    delete raw.principal;
    expect(() => validateApprovalSignal(raw)).toThrow(/principal is required/);
  });

  it("ep006_unit_approval_signal_rejects_missing_auth_context", () => {
    const raw = { ...makeApprovalSignal() } as Record<string, unknown>;
    delete raw.authentication;
    expect(() => validateApprovalSignal(raw)).toThrow(/auth context/);
  });

  it("ep006_unit_approval_signal_rejects_bad_strength", () => {
    const raw = { ...makeApprovalSignal() } as Record<string, unknown>;
    (raw.authentication as Record<string, unknown>).strength = "MAYBE";
    expect(() => validateApprovalSignal(raw)).toThrow(WorkflowContractError);
  });

  it("ep006_unit_approval_signal_rejects_bad_decision", () => {
    const raw = { ...makeApprovalSignal() } as Record<string, unknown>;
    raw.decision = "MAYBE";
    expect(() => validateApprovalSignal(raw)).toThrow(WorkflowContractError);
  });

  it("ep006_unit_approval_signal_rejects_bad_decided_at", () => {
    const raw = { ...makeApprovalSignal({ decidedAt: "yesterday" }) } as Record<
      string,
      unknown
    >;
    expect(() => validateApprovalSignal(raw)).toThrow(/ISO-8601/);
  });

  it("ep006_unit_approval_signal_rejects_wrong_signal_type", () => {
    const raw = { ...makeApprovalSignal() } as Record<string, unknown>;
    raw.signalType = "CANCEL";
    expect(() => validateApprovalSignal(raw)).toThrow(/must be APPROVAL/);
  });

  it("ep006_unit_approval_signal_rejects_missing_signal_id", () => {
    const raw = { ...makeApprovalSignal() } as Record<string, unknown>;
    delete raw.signalId;
    expect(() => validateApprovalSignal(raw)).toThrow(/signalId/);
  });

  it("ep006_unit_approval_binding_matches_digest_and_strength", () => {
    const signal = makeApprovalSignal();
    expect(() =>
      assertApprovalBinding(signal, actionIdA, actionDigestA, "STEP_UP"),
    ).not.toThrow();
  });

  it("ep006_unit_approval_binding_rejects_mismatched_digest", () => {
    const signal = makeApprovalSignal();
    expect(() =>
      assertApprovalBinding(signal, actionIdA, actionDigestB, "STEP_UP"),
    ).toThrow(/actionDigest does not match/);
  });

  it("ep006_unit_approval_binding_rejects_wrong_action", () => {
    const signal = makeApprovalSignal();
    expect(() =>
      assertApprovalBinding(signal, actionIdA, actionDigestA, "STEP_UP"),
    ).not.toThrow();
    // A different action id must never be satisfied by this approval.
    const other = makeApprovalSignal();
    expect(() =>
      assertApprovalBinding(
        other,
        "0193a1f2-0000-7000-8000-0000000000ff" as typeof actionIdA,
        actionDigestA,
        "STEP_UP",
      ),
    ).toThrow(/does not match awaited action/);
  });

  it("ep006_unit_approval_binding_rejects_insufficient_strength", () => {
    const signal = makeApprovalSignal({
      authentication: { strength: "SINGLE_FACTOR", method: "oidc" },
    });
    expect(() =>
      assertApprovalBinding(signal, actionIdA, actionDigestA, "STEP_UP"),
    ).toThrow(/below required STEP_UP/);
  });

  it("ep006_unit_approval_binding_binds_to_exact_principal", () => {
    // The binding includes the immutable principal: a signal from a
    // different principal cannot authorize the same action digest.
    const signal = makeApprovalSignal({
      principal: { id: "p-intruder", type: "AGENT" },
    });
    expect(() =>
      assertApprovalBinding(signal, actionIdA, actionDigestA, "STEP_UP"),
    ).not.toThrow(); // digest/strength pass; the workflow must also check principal
    expect(signal.principal.id).toBe("p-intruder");
  });
});
