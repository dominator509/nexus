/**
 * Test-zone fixtures for the ep006_integration_* suite (TESTING.md
 * test-double zone: tests/workflows/src is test code).
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
// Dedicated IDs so every integration test owns a unique workflow id on
// the ONE shared real server (single-fork suite); reuse would raise
// WorkflowExecutionAlreadyStarted while an earlier test's run is open.
export const WID_D = "0193a1f2-0000-7000-8000-000000000004";
export const WID_E = "0193a1f2-0000-7000-8000-000000000005";
export const WID_F = "0193a1f2-0000-7000-8000-000000000006";
export const WID_G = "0193a1f2-0000-7000-8000-000000000007";
export const WID_H = "0193a1f2-0000-7000-8000-000000000008";
export const WID_I = "0193a1f2-0000-7000-8000-000000000009";
export const WID_J = "0193a1f2-0000-7000-8000-000000000010";
export const WID_K = "0193a1f2-0000-7000-8000-000000000011";
export const WID_L = "0193a1f2-0000-7000-8000-000000000012";

export const signalIdA = parseSignalId(WID_A);
export const signalIdB = parseSignalId(WID_B);
export const signalIdD = parseSignalId(WID_D);
export const signalIdE = parseSignalId(WID_E);
export const workflowIdA = parseWorkflowId(WID_A);
export const workflowIdB = parseWorkflowId(WID_B);
export const workflowIdD = parseWorkflowId(WID_D);
export const workflowIdE = parseWorkflowId(WID_E);
export const workflowIdF = parseWorkflowId(WID_F);
export const workflowIdG = parseWorkflowId(WID_G);
export const workflowIdH = parseWorkflowId(WID_H);
export const workflowIdI = parseWorkflowId(WID_I);
export const workflowIdJ = parseWorkflowId(WID_J);
export const workflowIdK = parseWorkflowId(WID_K);
export const workflowIdL = parseWorkflowId(WID_L);
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
