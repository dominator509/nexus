/**
 * EP-006 M4: denied-permission failure class (execplan M4 content 1).
 *
 * The approval binding invariant: a signal whose actionId, digest, or
 * authentication strength does not match the awaited action must NOT
 * advance the workflow - it is quarantined as an invalid transition with
 * a structured reason. A REJECT decision with a valid binding must land
 * on the REJECTED terminal outcome.
 */

import { describe, expect, it } from "vitest";

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
  initialApprovalState,
} from "../state/approval.js";
import type { ApprovalRecord } from "../state/approval.js";
import {
  applyStepGateSignal,
  initialStepGateState,
  startStepGate,
} from "../state/step-gate.js";
import type { StepGateRecord, StepGateSeed } from "../state/step-gate.js";

const NOW = "2026-08-13T00:00:00Z";

function approvalRecord(): ApprovalRecord {
  return initialApprovalState({
    workflowId: workflowIdA,
    actionId: actionIdA,
    actionDigest: actionDigestA,
    requiredAuthenticationStrength: "STEP_UP",
    approvalTimeoutMs: 30_000,
    tenantId: "tenant-m4",
    correlationId: "corr-denied",
    principal: { id: "p-hob", type: "HUMAN" },
  }, NOW);
}

const SEED: StepGateSeed = {
  workflowId: workflowIdA,
  label: "denied objective",
  entityId: "obj-denied",
  steps: [
    {
      stepId: "step-1",
      title: "first",
      actionId: actionIdA,
      actionDigest: actionDigestA,
    },
  ],
};

function stepGateRecord(): StepGateRecord {
  return startStepGate(initialStepGateState(SEED, NOW), NOW);
}

describe("ep006_failure_denied_permission", () => {
  it("ep006_failure_denied_wrong_action_id_quarantined", () => {
    const transition = applyApprovalSignal(
      approvalRecord(),
      makeApprovalSignal({
        signalId: signalIdA,
        actionId: "0193a1f2-0000-7000-8000-000000009999" as never,
      }),
      NOW,
    );
    expect(transition.kind).toBe("invalid");
    if (transition.kind !== "invalid") {
      throw new Error("expected invalid transition");
    }
    expect(transition.reason).toMatch(/actionId/);
    expect(transition.record.state).toBe("REQUESTED");
  });

  it("ep006_failure_denied_wrong_digest_quarantined", () => {
    const transition = applyApprovalSignal(
      approvalRecord(),
      makeApprovalSignal({ signalId: signalIdA, actionDigest: actionDigestB }),
      NOW,
    );
    expect(transition.kind).toBe("invalid");
    if (transition.kind !== "invalid") {
      throw new Error("expected invalid transition");
    }
    expect(transition.reason).toMatch(/digest/);
    expect(transition.record.state).toBe("REQUESTED");
    // Quarantined: the observed signal is recorded but the gate never
    // advanced.
    expect(transition.record.observedSignalKeys.length).toBe(1);
  });

  it("ep006_failure_denied_insufficient_strength_quarantined", () => {
    const transition = applyApprovalSignal(
      approvalRecord(),
      makeApprovalSignal({
        signalId: signalIdA,
        authentication: {
          strength: "SINGLE_FACTOR",
          method: "password",
          sessionId: "sess-weak",
          verifiedAt: NOW,
        },
      }),
      NOW,
    );
    expect(transition.kind).toBe("invalid");
    if (transition.kind !== "invalid") {
      throw new Error("expected invalid transition");
    }
    expect(transition.reason).toMatch(/strength/);
    expect(transition.record.state).toBe("REQUESTED");
  });

  it("ep006_failure_denied_reject_decision_terminal", () => {
    const transition = applyApprovalSignal(
      approvalRecord(),
      makeApprovalSignal({ signalId: signalIdA, decision: "REJECT" }),
      NOW,
    );
    expect(transition.kind).toBe("accepted");
    expect(transition.record.state).toBe("REJECTED");
    expect(transition.record.outcome).toBe("REJECTED");
  });

  it("ep006_failure_denied_step_gate_wrong_digest_no_advance", () => {
    const transition = applyStepGateSignal(
      stepGateRecord(),
      makeApprovalSignal({ signalId: signalIdA, actionDigest: actionDigestB }),
      NOW,
    );
    expect(transition.kind).toBe("invalid");
    if (transition.kind !== "invalid") {
      throw new Error("expected invalid transition");
    }
    expect(transition.reason).toMatch(/does not match step/);
    expect(transition.record.state).toBe("AWAITING_APPROVAL");
  });

  it("ep006_failure_denied_step_gate_not_awaiting_ignored", () => {
    const r = stepGateRecord();
    const advanced = applyStepGateSignal(
      r,
      makeApprovalSignal({ signalId: signalIdA }),
      NOW,
    );
    expect(advanced.kind).toBe("advanced");
    // A second approval while step 1 is already approved is ignored.
    const ignored = applyStepGateSignal(
      advanced.record,
      makeApprovalSignal({ signalId: signalIdB }),
      NOW,
    );
    expect(ignored.kind).toBe("ignored");
  });
});
