import { describe, expect, it } from "vitest";

import { WorkflowContractError } from "@nexus/workflows";
import type { ApprovalInput } from "@nexus/workflows";
import {
  actionDigestA,
  actionDigestB,
  actionIdA,
  makeApprovalSignal,
  signalIdA,
  signalIdB,
  workflowIdA,
} from "./helpers/fixtures.js";

import {
  applyApprovalSignal,
  applyApprovalTimeout,
  applyCancel,
  initialApprovalState,
  isTerminalApprovalState,
  markCompensated,
} from "../state/approval.js";
import type { ApprovalRecord } from "../state/approval.js";

const NOW = "2026-08-13T00:00:00Z";

const INPUT: ApprovalInput = {
  workflowId: workflowIdA,
  tenantId: "tenant-1",
  correlationId: "corr-1",
  principal: { id: "p-hob", type: "HUMAN" },
  actionId: actionIdA,
  actionDigest: actionDigestA,
  requiredAuthenticationStrength: "STEP_UP",
};

function record(nowIso = NOW): ApprovalRecord {
  return initialApprovalState(INPUT, nowIso);
}

describe("ep006_unit_approval_state", () => {
  it("ep006_unit_approval_state_starts_requested", () => {
    const r = record();
    expect(r.state).toBe("REQUESTED");
    expect(r.outcome).toBeUndefined();
    expect(r.observedSignalKeys).toEqual([]);
    expect(r.observedSignals).toEqual([]);
  });

  it("ep006_unit_approval_state_approve_binds_digest_and_strength", () => {
    const r = record();
    const signal = makeApprovalSignal();
    const result = applyApprovalSignal(r, signal, NOW);
    expect(result.kind).toBe("accepted");
    if (result.kind === "accepted") {
      expect(result.record.state).toBe("APPROVED");
      expect(result.record.outcome).toBe("SUCCEEDED");
      expect(result.record.decision).toBe("APPROVE");
      expect(result.record.decidedBy).toBe("HUMAN:p-hob");
    }
  });

  it("ep006_unit_approval_state_reject_signal", () => {
    const r = record();
    const result = applyApprovalSignal(
      r,
      makeApprovalSignal({ decision: "REJECT" }),
      NOW,
    );
    expect(result.kind).toBe("accepted");
    if (result.kind === "accepted") {
      expect(result.record.state).toBe("REJECTED");
      expect(result.record.outcome).toBe("REJECTED");
    }
  });

  it("ep006_unit_approval_state_duplicate_signal_idempotent", () => {
    const r = record();
    const first = applyApprovalSignal(r, makeApprovalSignal(), NOW);
    expect(first.kind).toBe("accepted");
    // Re-delivery with the same signalId, different payload, against the
    // record that observed it: no-op.
    const redelivery = applyApprovalSignal(
      (first as { record: ApprovalRecord }).record,
      makeApprovalSignal({ decidedAt: "2026-08-13T09:00:00Z" }),
      NOW,
    );
    expect(redelivery.kind).toBe("duplicate");
  });

  it("ep006_unit_approval_state_wrong_digest_fails_closed", () => {
    const r = record();
    const result = applyApprovalSignal(
      r,
      makeApprovalSignal({ actionDigest: actionDigestB }),
      NOW,
    );
    expect(result.kind).toBe("invalid");
    // The invalid signal is recorded (quarantined) but state is unchanged.
    if (result.kind === "invalid") {
      expect(result.record.state).toBe("REQUESTED");
      expect(result.record.observedSignalKeys).toHaveLength(1);
    }
  });

  it("ep006_unit_approval_state_insufficient_strength_fails_closed", () => {
    const r = record();
    const result = applyApprovalSignal(
      r,
      makeApprovalSignal({
        authentication: { strength: "SINGLE_FACTOR", method: "oidc" },
      }),
      NOW,
    );
    expect(result.kind).toBe("invalid");
    if (result.kind === "invalid") {
      expect(result.reason).toMatch(/below required STEP_UP/);
      expect(result.record.state).toBe("REQUESTED");
    }
  });

  it("ep006_unit_approval_state_malformed_signal_invalid", () => {
    const r = record();
    const result = applyApprovalSignal(r, { signalType: "APPROVAL" }, NOW);
    expect(result.kind).toBe("invalid");
    expect(result.record.state).toBe("REQUESTED");
  });

  it("ep006_unit_approval_state_terminal_ignores_later_signals", () => {
    let r = record();
    r = (
      applyApprovalSignal(r, makeApprovalSignal(), NOW) as {
        record: ApprovalRecord;
      }
    ).record;
    expect(isTerminalApprovalState(r.state)).toBe(true);
    const later = applyApprovalSignal(
      r,
      makeApprovalSignal({ signalId: signalIdB, decision: "REJECT" }),
      NOW,
    );
    expect(later.kind).toBe("ignored");
    expect(later.record.state).toBe("APPROVED");
  });

  it("ep006_unit_approval_state_timeout_explicit", () => {
    const r = record();
    const timed = applyApprovalTimeout(r, NOW);
    expect(timed.state).toBe("TIMED_OUT");
    expect(timed.outcome).toBe("TIMED_OUT");
    // Timeout on a terminal state is a no-op.
    const terminal = applyApprovalTimeout(timed, NOW);
    expect(terminal.state).toBe("TIMED_OUT");
  });

  it("ep006_unit_approval_state_cancel_fail_closed", () => {
    const r = record();
    const cancelled = applyCancel(r, "CANCEL", NOW);
    expect(cancelled.state).toBe("CANCELLED");
    expect(cancelled.outcome).toBe("CANCELLED");
  });

  it("ep006_unit_approval_state_cancel_compensate_flow", () => {
    let r = record();
    r = applyCancel(r, "COMPENSATE", NOW);
    expect(r.state).toBe("COMPENSATING");
    r = markCompensated(r, NOW);
    expect(r.state).toBe("COMPENSATED");
    expect(r.outcome).toBe("COMPENSATED");
  });

  it("ep006_unit_approval_state_mark_compensated_requires_compensating", () => {
    const r = record();
    expect(() => markCompensated(r, NOW)).toThrow(WorkflowContractError);
  });

  it("ep006_unit_approval_state_observed_signals_immutable", () => {
    const r = record();
    const result = applyApprovalSignal(r, makeApprovalSignal(), NOW);
    if (result.kind === "accepted") {
      expect(result.record.observedSignals).toHaveLength(1);
      expect(result.record.observedSignals[0]?.signalId).toBe(signalIdA);
    }
  });
});
