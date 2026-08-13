/**
 * EP-006 M4: malformed-input failure class (execplan M4 content 1).
 *
 * Malformed signals, ids, and policies must fail CLOSED with the typed
 * WorkflowContractError carrying the SPEC-006 VALIDATION code and, where
 * present, correlation context - never with a bare TypeError and never
 * silently accepted.
 */

import { describe, expect, it } from "vitest";

import {
  parseActionDigest,
  parseActionId,
  parseActivityId,
  parseSignalId,
  parseWorkflowId,
  validateApprovalSignal,
  validateSignal,
  validateWorkflowPolicy,
  WorkflowContractError,
  DEFAULT_RETRY_POLICY,
} from "../index.js";
import {
  DIGEST_B,
  ISO_A,
  makeApprovalSignal,
  signalIdA,
  workflowIdA,
} from "./helpers/fixtures.js";

describe("ep006_failure_malformed_input", () => {
  it("ep006_failure_malformed_signal_type_rejected_structured", () => {
    const raw = { ...makeApprovalSignal(), signalType: "CANCEL" };
    try {
      validateApprovalSignal(raw);
      expect.unreachable("malformed signalType must be rejected");
    } catch (error) {
      expect(error).toBeInstanceOf(WorkflowContractError);
      expect((error as WorkflowContractError).code).toBe("VALIDATION");
    }
  });

  it("ep006_failure_malformed_signal_id_rejected_structured", () => {
    const raw = { ...makeApprovalSignal(), signalId: "not-a-uuid" };
    expect(() => validateApprovalSignal(raw)).toThrow(WorkflowContractError);
    expect(() => validateApprovalSignal(raw)).toThrow(/signalId/);
  });

  it("ep006_failure_malformed_digest_rejected_structured", () => {
    const raw = { ...makeApprovalSignal(), actionDigest: "short" };
    expect(() => validateApprovalSignal(raw)).toThrow(/sha256/);
    // Uppercase hex is not a canonical lowercase sha256 digest.
    const upper = { ...makeApprovalSignal(), actionDigest: DIGEST_B.toUpperCase() };
    expect(() => validateApprovalSignal(upper)).toThrow(/sha256/);
  });

  it("ep006_failure_malformed_principal_rejected_structured", () => {
    const raw = { ...makeApprovalSignal() } as Record<string, unknown>;
    delete raw.principal;
    expect(() => validateApprovalSignal(raw)).toThrow(WorkflowContractError);
    expect(() => validateApprovalSignal(raw)).toThrow(/principal is required/);
  });

  it("ep006_failure_malformed_auth_context_rejected_structured", () => {
    const raw = { ...makeApprovalSignal() } as Record<string, unknown>;
    delete raw.authentication;
    expect(() => validateApprovalSignal(raw)).toThrow(/auth context/);
    const weak = { ...makeApprovalSignal() } as Record<string, unknown>;
    (weak.authentication as Record<string, unknown>).strength = "MAYBE";
    expect(() => validateApprovalSignal(weak)).toThrow(WorkflowContractError);
  });

  it("ep006_failure_malformed_decision_rejected_structured", () => {
    const raw = { ...makeApprovalSignal(), decision: "MAYBE" };
    expect(() => validateApprovalSignal(raw)).toThrow(WorkflowContractError);
  });

  it("ep006_failure_malformed_workflow_id_rejected_structured", () => {
    expect(() => parseWorkflowId("nope")).toThrow(WorkflowContractError);
    expect(() => parseSignalId("nope")).toThrow(WorkflowContractError);
    expect(() => parseActionId("nope")).toThrow(WorkflowContractError);
    expect(() => parseActivityId("nope")).toThrow(WorkflowContractError);
  });

  it("ep006_failure_malformed_digest_parse_rejected_structured", () => {
    expect(() => parseActionDigest("not-hex")).toThrow(WorkflowContractError);
    expect(() => parseActionDigest("a".repeat(63))).toThrow(
      WorkflowContractError,
    );
  });

  it("ep006_failure_malformed_generic_signal_rejected_structured", () => {
    // validateSignal covers the generic WorkflowSignal shape (cancel etc.).
    const raw = {
      signalType: "CANCEL",
      signalId: signalIdA,
      workflowId: workflowIdA,
      reason: 42,
      requestedAt: ISO_A,
    };
    expect(() => validateSignal(raw)).toThrow(WorkflowContractError);
  });

  it("ep006_failure_malformed_policy_rejected_structured", () => {
    expect(() =>
      validateWorkflowPolicy({
        timeouts: { approvalTimeoutMs: -1 },
        cancelAction: "COMPENSATE",
        defaultActivityRetry: DEFAULT_RETRY_POLICY,
      } as never),
    ).toThrow(WorkflowContractError);
  });
});
