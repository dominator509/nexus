/**
 * Test-zone fixtures for the ep006_unit_* suite in @nexus/temporal.
 * TESTING.md test-double zone: src/__tests__/ is test code.
 */

import {
  parseActionDigest,
  parseActionId,
  parseSignalId,
  parseWorkflowId,
} from "@nexus/workflows";
import type {
  ApprovalSignal,
  AuthenticationContext,
  PrincipalRef,
} from "@nexus/workflows";

export const WID_A = "0193a1f2-0000-7000-8000-000000000001";
export const WID_B = "0193a1f2-0000-7000-8000-000000000002";
export const WID_C = "0193a1f2-0000-7000-8000-000000000003";

export const signalIdA = parseSignalId(WID_A);
export const signalIdB = parseSignalId(WID_B);
export const workflowIdA = parseWorkflowId(WID_A);
export const workflowIdB = parseWorkflowId(WID_B);
export const actionIdA = parseActionId(WID_C);

export const DIGEST_A = "a".repeat(64);
export const DIGEST_B = "b".repeat(64);
export const actionDigestA = parseActionDigest(DIGEST_A);
export const actionDigestB = parseActionDigest(DIGEST_B);

export const ISO_A = "2026-08-13T00:00:00Z";

export const PRINCIPAL_HUMAN: PrincipalRef = { id: "p-hob", type: "HUMAN" };
export const AUTH_STEP_UP: AuthenticationContext = {
  strength: "STEP_UP",
  method: "passkey",
  sessionId: "sess-1",
  verifiedAt: ISO_A,
};

export function makeApprovalSignal(
  overrides: Partial<ApprovalSignal> = {},
): ApprovalSignal {
  const base: ApprovalSignal = {
    signalType: "APPROVAL",
    signalId: signalIdA,
    workflowId: workflowIdA,
    actionId: actionIdA,
    actionDigest: actionDigestA,
    principal: { ...PRINCIPAL_HUMAN },
    authentication: { ...AUTH_STEP_UP },
    decision: "APPROVE",
    decidedAt: ISO_A,
  };
  return { ...base, ...overrides };
}
