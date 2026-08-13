/**
 * EP-006 M4: partial-side-effect failure class (execplan M4 content 1).
 *
 * When an effect fails after earlier steps succeeded, the step-gated
 * workflow must FAIL and then compensate every executed step (reverse
 * order). The compensation state machine marks executed/approved/failed
 * steps COMPENSATED and lands on the COMPENSATED terminal outcome.
 */

import { describe, expect, it } from "vitest";

import {
  actionDigestA,
  actionDigestB,
  actionIdA,
  makeApprovalSignal,
  signalIdA,
  workflowIdA,
} from "./helpers/fixtures.js";

import {
  applyStepGateSignal,
  beginStepExecution,
  cancelStepGate,
  completeStep,
  failStep,
  initialStepGateState,
  markStepGateCompensated,
  startStepGate,
} from "../state/step-gate.js";
import type { StepGateRecord, StepGateSeed } from "../state/step-gate.js";
import {
  applyCancel,
  initialApprovalState,
  markCompensated,
} from "../state/approval.js";
import type { ApprovalRecord } from "../state/approval.js";

const NOW = "2026-08-13T00:00:00Z";

const SEED: StepGateSeed = {
  workflowId: workflowIdA,
  label: "partial objective",
  entityId: "obj-partial",
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

function stepGateRecord(): StepGateRecord {
  return startStepGate(initialStepGateState(SEED, NOW), NOW);
}

describe("ep006_failure_partial_side_effect", () => {
  it("ep006_failure_partial_effect_failure_fails_workflow", () => {
    let r = stepGateRecord();
    // Step 1 approved and its effect VERIFIED (partial success).
    r = applyStepGateSignal(
      r,
      makeApprovalSignal({ signalId: signalIdA, actionDigest: actionDigestA }),
      NOW,
    ).record;
    r = beginStepExecution(r, NOW);
    r = completeStep(r, true, NOW);
    expect(r.state).toBe("AWAITING_APPROVAL");
    expect(r.currentIndex).toBe(1);
    // Step 2's effect FAILS: the workflow fails.
    r = failStep(r, NOW);
    expect(r.state).toBe("FAILED");
    expect(r.outcome).toBe("FAILED");
  });

  it("ep006_failure_partial_effect_failure_compensates_executed_steps", () => {
    let r = stepGateRecord();
    r = applyStepGateSignal(
      r,
      makeApprovalSignal({ signalId: signalIdA, actionDigest: actionDigestA }),
      NOW,
    ).record;
    r = beginStepExecution(r, NOW);
    r = completeStep(r, true, NOW);
    r = failStep(r, NOW);
    // Compensation: the FAILED step is marked COMPENSATED; the earlier
    // VERIFIED step stays VERIFIED (its effect is compensated by the
    // runner through applyCompensation activities, not by the state
    // machine). The workflow lands on COMPENSATED.
    r = markStepGateCompensated(r, NOW);
    expect(r.state).toBe("COMPENSATED");
    expect(r.outcome).toBe("COMPENSATED");
    expect(r.steps[0]?.state).toBe("VERIFIED");
    expect(r.steps[1]?.state).toBe("COMPENSATED");
  });

  it("ep006_failure_partial_effect_cancel_compensates_executed_steps", () => {
    let r = stepGateRecord();
    r = applyStepGateSignal(
      r,
      makeApprovalSignal({ signalId: signalIdA, actionDigest: actionDigestA }),
      NOW,
    ).record;
    r = beginStepExecution(r, NOW);
    r = completeStep(r, true, NOW);
    // Cancel with COMPENSATE policy after step 1's effect ran.
    r = cancelStepGate(r, "COMPENSATE", NOW);
    expect(r.state).toBe("COMPENSATING");
    r = markStepGateCompensated(r, NOW);
    expect(r.state).toBe("COMPENSATED");
    expect(r.outcome).toBe("COMPENSATED");
  });

  it("ep006_failure_partial_effect_terminal_cancel_ignored", () => {
    let r = stepGateRecord();
    r = applyStepGateSignal(
      r,
      makeApprovalSignal({ signalId: signalIdA, actionDigest: actionDigestA }),
      NOW,
    ).record;
    r = beginStepExecution(r, NOW);
    r = completeStep(r, true, NOW);
    r = completeStep(r, true, NOW);
    expect(r.state).toBe("SUCCEEDED");
    // Cancel after terminal state is a no-op (never rewinds success).
    const after = cancelStepGate(r, "COMPENSATE", NOW);
    expect(after.state).toBe("SUCCEEDED");
  });

  it("ep006_failure_partial_approval_cancel_compensates", () => {
    const approval: ApprovalRecord = initialApprovalState(
      {
        workflowId: workflowIdA,
        actionId: actionIdA,
        actionDigest: actionDigestA,
        requiredAuthenticationStrength: "STEP_UP",
        approvalTimeoutMs: 30_000,
        tenantId: "tenant-m4",
        correlationId: "corr-partial",
        principal: { id: "p-hob", type: "HUMAN" },
      },
      NOW,
    );
    const compensating = applyCancel(approval, "COMPENSATE", NOW);
    expect(compensating.state).toBe("COMPENSATING");
    const compensated = markCompensated(compensating, NOW);
    expect(compensated.state).toBe("COMPENSATED");
    expect(compensated.outcome).toBe("COMPENSATED");
  });
});
