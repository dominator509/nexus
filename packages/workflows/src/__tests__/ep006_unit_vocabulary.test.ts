import { describe, expect, it } from "vitest";

import { WorkflowContractError } from "../errors.js";
import {
  VOCABULARIES,
  workflowKind,
  workflowState,
  workflowOutcome,
  signalType,
  queryType,
  approvalDecision,
  authenticationStrength,
  principalType,
  activityKind,
  activityState,
  retryErrorClass,
  cancelAction,
} from "../vocabulary.js";

describe("ep006_unit_vocabulary", () => {
  it("ep006_unit_vocabulary_parses_valid_values", () => {
    expect(workflowKind.parse("OBJECTIVE")).toBe("OBJECTIVE");
    expect(workflowKind.parse("APPROVAL")).toBe("APPROVAL");
    expect(workflowKind.parse("CONNECTOR_CERTIFICATION")).toBe(
      "CONNECTOR_CERTIFICATION",
    );
    expect(workflowKind.parse("INCIDENT_REMEDIATION")).toBe(
      "INCIDENT_REMEDIATION",
    );
    expect(workflowKind.parse("DEPLOYMENT")).toBe("DEPLOYMENT");
    expect(signalType.parse("APPROVAL")).toBe("APPROVAL");
    expect(signalType.parse("CANCEL")).toBe("CANCEL");
    expect(signalType.parse("RESUME")).toBe("RESUME");
    expect(approvalDecision.parse("APPROVE")).toBe("APPROVE");
    expect(approvalDecision.parse("REJECT")).toBe("REJECT");
    expect(authenticationStrength.parse("STEP_UP")).toBe("STEP_UP");
    expect(cancelAction.parse("COMPENSATE")).toBe("COMPENSATE");
    expect(principalType.parse("HUMAN")).toBe("HUMAN");
  });

  it("ep006_unit_vocabulary_rejects_unknown_values", () => {
    for (const vocabulary of VOCABULARIES) {
      expect(() => vocabulary.parse("BOGUS")).toThrow(WorkflowContractError);
    }
  });

  it("ep006_unit_vocabulary_rejects_wrong_types", () => {
    expect(() => workflowKind.parse(123)).toThrow(WorkflowContractError);
    expect(() => workflowKind.parse(null)).toThrow(WorkflowContractError);
    expect(() => workflowKind.parse(undefined)).toThrow(WorkflowContractError);
  });

  it("ep006_unit_vocabulary_is_guards", () => {
    expect(workflowKind.is("OBJECTIVE")).toBe(true);
    expect(workflowKind.is("BOGUS")).toBe(false);
    expect(workflowKind.is(42)).toBe(false);
    expect(signalType.is("APPROVAL")).toBe(true);
  });

  it("ep006_unit_vocabulary_serializes_and_roundtrips", () => {
    const json = JSON.stringify("CANCELLED");
    expect(workflowState.parse(JSON.parse(json))).toBe("CANCELLED");
    expect(JSON.stringify(workflowOutcome.parse("COMPENSATED"))).toBe(
      '"COMPENSATED"',
    );
  });

  it("ep006_unit_vocabulary_locked_signal_names", () => {
    // SPEC-023/ADR-010: new signal types require an ADR, never ad hoc.
    expect([...signalType.values]).toEqual(["APPROVAL", "CANCEL", "RESUME"]);
  });

  it("ep006_unit_vocabulary_explicit_timeout_cancel_states", () => {
    // EP-006 acceptance obligation 3: timeout/cancel paths are explicit.
    expect(workflowState.is("TIMED_OUT")).toBe(true);
    expect(workflowState.is("CANCELLED")).toBe(true);
    expect(workflowState.is("COMPENSATED")).toBe(true);
    expect(workflowState.is("AWAITING_APPROVAL")).toBe(true);
    expect(cancelAction.values).toEqual(["CANCEL", "COMPENSATE"]);
  });

  it("ep006_unit_vocabulary_activity_kinds_cover_side_effects", () => {
    // SPEC-023 behavior 6: side effects only in activities.
    expect(activityKind.is("EXTERNAL_EFFECT")).toBe(true);
    expect(activityKind.is("VERIFY")).toBe(true);
    expect(activityKind.is("COMPENSATE")).toBe(true);
    expect(activityState.is("RETRYING")).toBe(true);
    expect(retryErrorClass.is("PERMANENT")).toBe(true);
  });

  it("ep006_unit_vocabulary_query_types_locked", () => {
    expect(queryType.is("WORKFLOW_STATUS")).toBe(true);
    expect(queryType.is("PENDING_APPROVAL")).toBe(true);
    expect(queryType.is("ACTIVITY_STATE")).toBe(true);
    expect(queryType.is("ACTION_RECEIPT")).toBe(true);
  });
});
