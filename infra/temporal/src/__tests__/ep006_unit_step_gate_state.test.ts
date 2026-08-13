import { describe, expect, it } from "vitest";

import type { ObjectiveInput } from "@nexus/workflows";
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
  applyStepGateSignal,
  beginStepExecution,
  cancelStepGate,
  completeStep,
  currentStep,
  initialStepGateState,
  isTerminalStepGateState,
  markStepGateCompensated,
  startStepGate,
  timeoutStepGate,
} from "../state/step-gate.js";
import type { StepGateRecord, StepGateSeed } from "../state/step-gate.js";

const NOW = "2026-08-13T00:00:00Z";

const SEED: StepGateSeed = {
  workflowId: workflowIdA,
  label: "test objective",
  entityId: "obj-1",
  steps: [
    {
      stepId: "step-1",
      title: "first",
      actionId: actionIdA,
      actionDigest: actionDigestA,
    },
    {
      stepId: "step-2",
      title: "second",
      actionId: actionIdA,
      actionDigest: actionDigestB,
    },
  ],
};

function record(nowIso = NOW): StepGateRecord {
  return startStepGate(initialStepGateState(SEED, nowIso), nowIso);
}

function approvalFor(
  digest: typeof actionDigestA,
  signalId: typeof signalIdA = signalIdA,
) {
  return makeApprovalSignal({ actionDigest: digest, signalId });
}

describe("ep006_unit_step_gate_state", () => {
  it("ep006_unit_step_gate_state_starts_first_step_awaiting", () => {
    const r = record();
    expect(r.state).toBe("AWAITING_APPROVAL");
    expect(currentStep(r)?.stepId).toBe("step-1");
    expect(currentStep(r)?.state).toBe("AWAITING_APPROVAL");
  });

  it("ep006_unit_step_gate_state_approve_advances", () => {
    const r = record();
    const result = applyStepGateSignal(r, approvalFor(actionDigestA), NOW);
    expect(result.kind).toBe("advanced");
    if (result.kind === "advanced") {
      expect(result.record.state).toBe("APPROVED");
      expect(currentStep(result.record)?.state).toBe("APPROVED");
    }
  });

  it("ep006_unit_step_gate_state_wrong_digest_rejected", () => {
    const r = record();
    const result = applyStepGateSignal(r, approvalFor(actionDigestB), NOW);
    expect(result.kind).toBe("invalid");
    expect(result.record.state).toBe("AWAITING_APPROVAL");
  });

  it("ep006_unit_step_gate_state_duplicate_idempotent", () => {
    const r = record();
    const first = applyStepGateSignal(r, approvalFor(actionDigestA), NOW);
    expect(first.kind).toBe("advanced");
    // Same signalId redelivery against the record that observed it.
    const again = applyStepGateSignal(
      (first as { record: StepGateRecord }).record,
      approvalFor(actionDigestA),
      NOW,
    );
    expect(again.kind).toBe("duplicate");
  });

  it("ep006_unit_step_gate_state_execute_verify_advance", () => {
    let r = record();
    r = (
      applyStepGateSignal(r, approvalFor(actionDigestA), NOW) as {
        record: StepGateRecord;
      }
    ).record;
    r = beginStepExecution(r, NOW);
    expect(r.state).toBe("EXECUTING");
    r = completeStep(r, true, NOW);
    expect(r.state).toBe("AWAITING_APPROVAL");
    expect(currentStep(r)?.stepId).toBe("step-2");
  });

  it("ep006_unit_step_gate_state_last_step_succeeds", () => {
    let r = record();
    r = (
      applyStepGateSignal(r, approvalFor(actionDigestA), NOW) as {
        record: StepGateRecord;
      }
    ).record;
    r = completeStep(r, true, NOW);
    r = (
      applyStepGateSignal(r, approvalFor(actionDigestB, signalIdB), NOW) as {
        record: StepGateRecord;
      }
    ).record;
    r = beginStepExecution(r, NOW);
    r = completeStep(r, true, NOW);
    expect(r.state).toBe("SUCCEEDED");
    expect(r.outcome).toBe("SUCCEEDED");
    expect(isTerminalStepGateState(r.state)).toBe(true);
  });

  it("ep006_unit_step_gate_state_verification_failure_fails", () => {
    let r = record();
    r = (
      applyStepGateSignal(r, approvalFor(actionDigestA), NOW) as {
        record: StepGateRecord;
      }
    ).record;
    r = beginStepExecution(r, NOW);
    r = completeStep(r, false, NOW);
    expect(r.state).toBe("FAILED");
    expect(r.outcome).toBe("FAILED");
  });

  it("ep006_unit_step_gate_state_approval_timeout_explicit", () => {
    const r = record();
    const timed = timeoutStepGate(r, NOW);
    expect(timed.state).toBe("TIMED_OUT");
    expect(timed.outcome).toBe("TIMED_OUT");
  });

  it("ep006_unit_step_gate_state_cancel_compensate", () => {
    let r = record();
    r = cancelStepGate(r, "COMPENSATE", NOW);
    expect(r.state).toBe("COMPENSATING");
    r = markStepGateCompensated(r, NOW);
    expect(r.state).toBe("COMPENSATED");
    expect(r.outcome).toBe("COMPENSATED");
    // Executed steps are marked compensated; unexecuted steps remain.
    expect(
      r.steps.every(
        (s) => !["EXECUTING", "APPROVED", "FAILED"].includes(s.state),
      ),
    ).toBe(true);
  });

  it("ep006_unit_step_gate_state_cancel_fail_closed", () => {
    const r = record();
    const cancelled = cancelStepGate(r, "CANCEL", NOW);
    expect(cancelled.state).toBe("CANCELLED");
    expect(cancelled.outcome).toBe("CANCELLED");
  });

  it("ep006_unit_step_gate_state_signal_for_future_step_ignored", () => {
    // A signal for step-2's digest while step-1 is awaiting is invalid.
    const r = record();
    const result = applyStepGateSignal(
      r,
      approvalFor(actionDigestB, signalIdB),
      NOW,
    );
    expect(result.kind).toBe("invalid");
    expect(currentStep(result.record)?.stepId).toBe("step-1");
  });
});

// Keep ObjectiveInput referenced so the fixture contract stays aligned.
void (null as unknown as ObjectiveInput);
