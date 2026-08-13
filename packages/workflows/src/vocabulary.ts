/**
 * Workflow vocabulary (SPEC-023 canonical terms; ADR-010).
 *
 * SPEC-023 locks the names Workflow, Activity, Signal, Query, Schedule,
 * ApprovalWorkflow, and Compensation to this node. SPEC-005 locks
 * Authentication Strength and Approval Assertion. Every enum here is
 * vocabulary locked: unknown values are rejected at parse time, and a new
 * synonym requires an ADR and schema update.
 */

import { WorkflowContractError } from "./errors.js";

export const WORKFLOW_VOCABULARY_VERSION = "1.0.0";

export interface Vocabulary<T extends string> {
  readonly name: string;
  readonly values: readonly T[];
  is(value: unknown): value is T;
  parse(value: unknown, context?: string): T;
}

export function defineVocabulary<T extends string>(
  name: string,
  values: readonly T[],
): Vocabulary<T> {
  const set = new Set<string>(values);
  return {
    name,
    values,
    is(value: unknown): value is T {
      return typeof value === "string" && set.has(value);
    },
    parse(value: unknown, context = name): T {
      if (typeof value === "string" && set.has(value)) {
        return value as T;
      }
      throw new WorkflowContractError(
        `invalid ${context}: ${JSON.stringify(value)}; expected one of ${values.join(", ")}`,
      );
    },
  };
}

/** Workflow kinds owned by this node (ADR-010). */
export const workflowKind = defineVocabulary("WorkflowKind", [
  "OBJECTIVE",
  "APPROVAL",
  "CONNECTOR_CERTIFICATION",
  "INCIDENT_REMEDIATION",
  "DEPLOYMENT",
] as const);
export type WorkflowKind = (typeof workflowKind.values)[number];

/**
 * Workflow lifecycle states. The action-facing states mirror SPEC-006
 * ActionLifecycle (requested, evaluated, awaiting approval, approved,
 * executing, verifying, succeeded, failed, compensating, compensated,
 * rejected); Temporal-owned terminal states CANCELLED and TIMED_OUT are
 * explicit (SPEC-023 behavior 5, EP-006 acceptance obligation 3).
 */
export const workflowState = defineVocabulary("WorkflowState", [
  "REQUESTED",
  "EVALUATED",
  "AWAITING_APPROVAL",
  "APPROVED",
  "EXECUTING",
  "VERIFYING",
  "SUCCEEDED",
  "FAILED",
  "REJECTED",
  "COMPENSATING",
  "COMPENSATED",
  "CANCELLED",
  "TIMED_OUT",
] as const);
export type WorkflowState = (typeof workflowState.values)[number];

/** Terminal workflow outcomes (query responses and receipts). */
export const workflowOutcome = defineVocabulary("WorkflowOutcome", [
  "SUCCEEDED",
  "REJECTED",
  "FAILED",
  "CANCELLED",
  "TIMED_OUT",
  "COMPENSATED",
] as const);
export type WorkflowOutcome = (typeof workflowOutcome.values)[number];

/** Durable signal types. New signal types require an ADR (ADR-010). */
export const signalType = defineVocabulary("SignalType", [
  "APPROVAL",
  "CANCEL",
  "RESUME",
] as const);
export type SignalType = (typeof signalType.values)[number];

/** Durable query types. New query types require an ADR (ADR-010). */
export const queryType = defineVocabulary("QueryType", [
  "WORKFLOW_STATUS",
  "PENDING_APPROVAL",
  "ACTIVITY_STATE",
  "ACTION_RECEIPT",
] as const);
export type QueryType = (typeof queryType.values)[number];

/** Human approval decision (SPEC-023 behavior 7). */
export const approvalDecision = defineVocabulary("ApprovalDecision", [
  "APPROVE",
  "REJECT",
] as const);
export type ApprovalDecision = (typeof approvalDecision.values)[number];

/**
 * Authentication strength (SPEC-005 canonical term). STEP_UP is a
 * cryptographic step-up; SPEC-005 behavior 4 requires it for R3 and R4
 * actions and forbids model approval for R4.
 */
export const authenticationStrength = defineVocabulary(
  "AuthenticationStrength",
  ["NONE", "SINGLE_FACTOR", "MULTI_FACTOR", "STEP_UP"] as const,
);
export type AuthenticationStrength =
  (typeof authenticationStrength.values)[number];

/** Principal types (EP-002/SPEC-001/SPEC-005 locked). */
export const principalType = defineVocabulary("PrincipalType", [
  "HUMAN",
  "SERVICE",
  "AGENT",
  "DEVICE",
  "SYSTEM",
] as const);
export type PrincipalType = (typeof principalType.values)[number];

/**
 * Activity kinds. EXTERNAL_EFFECT and VERIFY are the only surfaces that
 * touch the outside world; COMPENSATE rolls back a prior effect (SPEC-006
 * behavior 8). SPEC-023 behavior 6 requires every side effect to live in
 * an activity, never in workflow code.
 */
export const activityKind = defineVocabulary("ActivityKind", [
  "EXTERNAL_EFFECT",
  "VERIFY",
  "COMPENSATE",
] as const);
export type ActivityKind = (typeof activityKind.values)[number];

export const activityState = defineVocabulary("ActivityState", [
  "PENDING",
  "SCHEDULED",
  "RUNNING",
  "RETRYING",
  "SUCCEEDED",
  "FAILED",
  "CANCELLED",
] as const);
export type ActivityState = (typeof activityState.values)[number];

/**
 * Retry error classes (SPEC-006 behavior 7: retries are bounded and
 * classified by error). PERMANENT is never retried.
 */
export const retryErrorClass = defineVocabulary("RetryErrorClass", [
  "TRANSIENT",
  "RATE_LIMIT",
  "UNAVAILABLE",
  "TIMEOUT",
  "PERMANENT",
] as const);
export type RetryErrorClass = (typeof retryErrorClass.values)[number];

/**
 * Cancel semantics (EP-006 acceptance obligation 3). CANCEL fails closed
 * and stops; COMPENSATE runs the registered compensation activities in
 * reverse order before terminating (SPEC-006 behavior 8).
 */
export const cancelAction = defineVocabulary("CancelAction", [
  "CANCEL",
  "COMPENSATE",
] as const);
export type CancelAction = (typeof cancelAction.values)[number];

export const VOCABULARIES: readonly Vocabulary<string>[] = [
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
];
