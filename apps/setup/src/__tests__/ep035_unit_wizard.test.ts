/**
 * EP-035 M1 SetupWizard state-machine tests.
 *
 * The wizard models state, not visual progress: a page being visited
 * never completes a step; COMPLETE_LOCAL is a local checkpoint that
 * never equals VERIFIED; transitions are typed and invalid leaps fail
 * closed.
 */

import { describe, expect, it } from "vitest";
import {
  SetupWizardState,
  WizardAdvanceRequest,
  WizardVerifyRequest,
  isValidStepStatusTransition,
  isValidWizardStateTransition,
} from "../contracts/wizard";
import { ErrorCode, Spec006Error } from "../contracts/errors";

const CORRELATION = "00000000-0000-4000-8000-000000000001";

describe("ep035_unit_wizard", () => {
  it("begins NOT_STARTED with every step PENDING", () => {
    const wizard = SetupWizardState.notStarted(CORRELATION, 1000);
    expect(wizard.state).toBe("NOT_STARTED");
    expect(wizard.steps.length).toBe(8);
    for (const step of wizard.steps) {
      expect(step.status).toBe("PENDING");
    }
  });

  it("accepts the canonical NOT_STARTED -> IN_PROGRESS start", () => {
    const started = SetupWizardState.notStarted(CORRELATION, 1000).advance(
      "IN_PROGRESS",
      1001,
    );
    expect(started.state).toBe("IN_PROGRESS");
  });

  it("rejects NOT_STARTED -> COMPLETED leap", () => {
    const wizard = SetupWizardState.notStarted(CORRELATION, 1000);
    try {
      wizard.advance("COMPLETED", 1001);
      throw new Error("expected rejection");
    } catch (err) {
      expect((err as Spec006Error).code).toBe(ErrorCode.Policy);
    }
  });

  it("rejects FAILED -> COMPLETED without recovery", () => {
    const wizard = SetupWizardState.notStarted(CORRELATION, 1000)
      .advance("IN_PROGRESS", 1001)
      .advance("FAILED", 1002);
    expect(() => wizard.advance("COMPLETED", 1003)).toThrowError(Spec006Error);
  });

  it("cannot complete while any step is unverified", () => {
    const wizard = SetupWizardState.notStarted(CORRELATION, 1000).advance(
      "IN_PROGRESS",
      1001,
    );
    try {
      wizard.advance("COMPLETED", 1002);
      throw new Error("expected rejection");
    } catch (err) {
      expect((err as Spec006Error).code).toBe(ErrorCode.Policy);
    }
  });

  it("COMPLETE_LOCAL is never VERIFIED and requires a verification record", () => {
    const wizard = SetupWizardState.notStarted(CORRELATION, 1000)
      .advance("IN_PROGRESS", 1001)
      .advanceStep("DEPLOYMENT_CHOICE", "IN_PROGRESS", 1002)
      .advanceStep("DEPLOYMENT_CHOICE", "COMPLETE_LOCAL", 1003);
    expect(wizard.stepRecord("DEPLOYMENT_CHOICE").status).toBe(
      "COMPLETE_LOCAL",
    );
    expect(wizard.stepRecord("DEPLOYMENT_CHOICE").verification).toBeUndefined();
    // COMPLETE_LOCAL -> VERIFIED without a record is rejected.
    try {
      wizard.advanceStep("DEPLOYMENT_CHOICE", "VERIFIED", 1004);
      throw new Error("expected rejection");
    } catch (err) {
      expect((err as Spec006Error).code).toBe(ErrorCode.Verification);
    }
  });

  it("VERIFIED requires an explicit remote verification record", () => {
    const wizard = SetupWizardState.notStarted(CORRELATION, 1000)
      .advance("IN_PROGRESS", 1001)
      .advanceStep("DEPLOYMENT_CHOICE", "IN_PROGRESS", 1002)
      .advanceStep("DEPLOYMENT_CHOICE", "COMPLETE_LOCAL", 1003)
      .advanceStep("DEPLOYMENT_CHOICE", "VERIFIED", 1004, {
        verified_at_unix_s: 1005,
        verifier: "setup-probe",
      });
    const record = wizard.stepRecord("DEPLOYMENT_CHOICE");
    expect(record.status).toBe("VERIFIED");
    expect(record.verification?.verifier).toBe("setup-probe");
  });

  it("rejects a verification record on a non-VERIFIED step", () => {
    const wizard = SetupWizardState.notStarted(CORRELATION, 1000)
      .advance("IN_PROGRESS", 1001)
      .advanceStep("DEPLOYMENT_CHOICE", "IN_PROGRESS", 1002);
    try {
      wizard.advanceStep("DEPLOYMENT_CHOICE", "IN_PROGRESS", 1003, {
        verified_at_unix_s: 1003,
        verifier: "probe",
      });
      throw new Error("expected rejection");
    } catch (err) {
      expect((err as Spec006Error).code).toBe(ErrorCode.Validation);
    }
  });

  it("round-trips serialization without losing state", () => {
    const wizard = SetupWizardState.notStarted(CORRELATION, 1000)
      .advance("IN_PROGRESS", 1001)
      .advanceStep("DEPLOYMENT_CHOICE", "IN_PROGRESS", 1002);
    const parsed = SetupWizardState.parse(JSON.parse(JSON.stringify(wizard)));
    expect(parsed.state).toBe("IN_PROGRESS");
    expect(parsed.stepRecord("DEPLOYMENT_CHOICE").status).toBe("IN_PROGRESS");
  });

  it("rejects unknown fields and unknown enum values on wire parse", () => {
    const good = JSON.parse(
      JSON.stringify(SetupWizardState.notStarted(CORRELATION, 1000)),
    );
    expect(() =>
      SetupWizardState.parse({ ...good, forged: true }),
    ).toThrowError(Spec006Error);
    const badEnum = JSON.parse(
      JSON.stringify(SetupWizardState.notStarted(CORRELATION, 1000)),
    );
    badEnum.state = "MADE_UP";
    try {
      SetupWizardState.parse(badEnum);
      throw new Error("expected rejection");
    } catch (err) {
      expect((err as Spec006Error).code).toBe(ErrorCode.Vocabulary);
    }
  });

  it("rejects a missing required step in the steps array", () => {
    const good = JSON.parse(
      JSON.stringify(SetupWizardState.notStarted(CORRELATION, 1000)),
    );
    good.steps = good.steps.slice(0, 7);
    expect(() => SetupWizardState.parse(good)).toThrowError(Spec006Error);
  });

  it("exposes typed transition predicates", () => {
    expect(isValidWizardStateTransition("NOT_STARTED", "IN_PROGRESS")).toBe(
      true,
    );
    expect(isValidWizardStateTransition("NOT_STARTED", "COMPLETED")).toBe(
      false,
    );
    expect(isValidStepStatusTransition("COMPLETE_LOCAL", "VERIFIED")).toBe(
      true,
    );
    expect(isValidStepStatusTransition("PENDING", "VERIFIED")).toBe(false);
  });

  it("parses advance and verify requests with deny-unknown", () => {
    const advance = WizardAdvanceRequest.parse({
      correlation_id: CORRELATION,
      step: "DEPLOYMENT_CHOICE",
      to_status: "IN_PROGRESS",
    });
    expect(advance.step).toBe("DEPLOYMENT_CHOICE");
    expect(() =>
      WizardAdvanceRequest.parse({
        correlation_id: CORRELATION,
        step: "MADE_UP",
        to_status: "IN_PROGRESS",
      }),
    ).toThrowError(Spec006Error);
    expect(() =>
      WizardAdvanceRequest.parse({
        correlation_id: CORRELATION,
        step: "DEPLOYMENT_CHOICE",
        to_status: "IN_PROGRESS",
        forged: true,
      }),
    ).toThrowError(Spec006Error);
    const verify = WizardVerifyRequest.parse({
      correlation_id: CORRELATION,
      step: "DEPLOYMENT_CHOICE",
      verification: { verified_at_unix_s: 5, verifier: "probe" },
    });
    expect(verify.verification.verifier).toBe("probe");
  });
});
