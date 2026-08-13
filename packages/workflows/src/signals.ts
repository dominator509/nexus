/**
 * Durable workflow signals (SPEC-023 behavior 7; ADR-010).
 *
 * Hard invariants encoded in this contract:
 *
 * 1. Duplicate signals are idempotent. Every signal carries a signalId
 *    (UUIDv7); signalKey() is the canonical dedup key, a pure function of
 *    workflow, type, and signalId. Re-delivering the same signalId must
 *    produce the same logical effect exactly once.
 *
 * 2. Approval signals bind to the exact action digest and principal/auth
 *    context. The ApprovalSignal carries the canonical SHA-256 actionDigest
 *    of the exact action payload, the immutable principal, and the
 *    immutable authentication strength/context. A workflow accepts an
 *    approval only when the digest matches the action it is waiting on and
 *    the principal/auth context satisfies the approval class (SPEC-005
 *    behavior 4: R3/R4 require STEP_UP; R4 never accepts model approval).
 *
 * 3. Signals are immutable once emitted: principal, authentication,
 *    decision, and decidedAt are set by the signer boundary and never
 *    mutated by workflow code.
 */

import {
  parseActionDigest,
  parseSignalId,
  parseUuidV7,
  parseWorkflowId,
} from "./ids.js";
import type { ActionDigest, ActionId, SignalId, WorkflowId } from "./ids.js";
import { WorkflowContractError } from "./errors.js";
import {
  approvalDecision,
  authenticationStrength,
  principalType,
  signalType,
} from "./vocabulary.js";
import type { ApprovalDecision, AuthenticationStrength } from "./vocabulary.js";
import type { PrincipalRef } from "./activities.js";

export interface AuthenticationContext {
  readonly strength: AuthenticationStrength;
  /** Authentication method label from the auth boundary, e.g. "passkey". */
  readonly method: string;
  readonly sessionId?: string;
  /** ISO-8601 timestamp set by the auth boundary at verification time. */
  readonly verifiedAt?: string;
}

export interface ApprovalSignal {
  readonly signalType: "APPROVAL";
  /** Idempotency key for the signal; must be a UUIDv7. */
  readonly signalId: SignalId;
  readonly workflowId: WorkflowId;
  /** The exact action this approval binds to. */
  readonly actionId: ActionId;
  /** Canonical SHA-256 of the exact action payload being approved. */
  readonly actionDigest: ActionDigest;
  /** Immutable signer identity. */
  readonly principal: PrincipalRef;
  /** Immutable authentication strength and context. */
  readonly authentication: AuthenticationContext;
  readonly decision: ApprovalDecision;
  /** ISO-8601 signer wall time; immutable once emitted. */
  readonly decidedAt: string;
  readonly comment?: string;
}

export interface CancelSignal {
  readonly signalType: "CANCEL";
  readonly signalId: SignalId;
  readonly workflowId: WorkflowId;
  readonly reason?: string;
  /** ISO-8601 request time; immutable. */
  readonly requestedAt: string;
}

export interface ResumeSignal {
  readonly signalType: "RESUME";
  readonly signalId: SignalId;
  readonly workflowId: WorkflowId;
  /** ISO-8601 request time; immutable. */
  readonly requestedAt: string;
}

export type WorkflowSignal = ApprovalSignal | CancelSignal | ResumeSignal;

/**
 * Canonical signal dedup key. Duplicate signals (same workflow, type, and
 * signalId) collapse to the same key; the engine and workflow code use it
 * to guarantee at-most-once logical processing of a re-delivered signal.
 */
export function signalKey(signal: WorkflowSignal): string {
  return `${signal.workflowId}:${signal.signalType}:${signal.signalId}`;
}

/**
 * True when `incoming` is a re-delivery of `existing` (idempotent
 * duplicate). The key is compared, never the payload.
 */
export function isIdempotentDuplicate(
  existing: WorkflowSignal,
  incoming: WorkflowSignal,
): boolean {
  return signalKey(existing) === signalKey(incoming);
}

/**
 * Deduplicate a signal sequence by canonical key, first delivery wins.
 * Re-delivered signals (same workflow, type, and signalId) collapse to
 * one logical effect; this is the contract-level half of exactly-once
 * signal processing (the durable engine supplies the other half).
 */
export function dedupeSignals(
  signals: readonly WorkflowSignal[],
): WorkflowSignal[] {
  const seen = new Set<string>();
  const result: WorkflowSignal[] = [];
  for (const signal of signals) {
    const key = signalKey(signal);
    if (!seen.has(key)) {
      seen.add(key);
      result.push(signal);
    }
  }
  return result;
}

/** ISO-8601 basic timestamp validation (deterministic, no wall clock). */
const ISO8601_RE =
  /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/;

export function isIso8601(value: unknown): value is string {
  return typeof value === "string" && ISO8601_RE.test(value);
}

function requireIso8601(value: unknown, field: string): string {
  if (!isIso8601(value)) {
    throw new WorkflowContractError(
      `${field} must be an ISO-8601 timestamp, got ${JSON.stringify(value)}`,
    );
  }
  return value;
}

function requireNonEmptyString(value: unknown, field: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new WorkflowContractError(
      `${field} must be a non-empty string, got ${JSON.stringify(value)}`,
    );
  }
  return value;
}

/**
 * Validate an ApprovalSignal structurally. Every field is checked: signalId
 * and workflowId are UUIDv7, actionDigest is a 64-char sha256 hex, the
 * principal and authentication context are present and vocabularic, and
 * decidedAt is ISO-8601. Throws WorkflowContractError on violation.
 */
export function validateApprovalSignal(value: unknown): ApprovalSignal {
  if (typeof value !== "object" || value === null) {
    throw new WorkflowContractError(
      `ApprovalSignal must be an object, got ${JSON.stringify(value)}`,
    );
  }
  const record = value as Record<string, unknown>;
  signalType.parse(record.signalType, "signalType");
  if (record.signalType !== "APPROVAL") {
    throw new WorkflowContractError(
      `ApprovalSignal.signalType must be APPROVAL, got ${JSON.stringify(record.signalType)}`,
    );
  }
  const signalId = parseSignalId(record.signalId);
  const workflowId = parseWorkflowId(record.workflowId);
  const actionId = parseUuidV7(record.actionId, "actionId") as ActionId;
  const actionDigest = parseActionDigest(record.actionDigest);
  const decision = approvalDecision.parse(record.decision, "decision");
  const principal = record.principal as Record<string, unknown> | undefined;
  if (typeof principal !== "object" || principal === null) {
    throw new WorkflowContractError("ApprovalSignal.principal is required");
  }
  const principalTypeValue = principalType.parse(
    principal.type,
    "principal.type",
  );
  const principalId = requireNonEmptyString(principal.id, "principal.id");
  const rawAuthentication = record.authentication as
    | Record<string, unknown>
    | undefined;
  if (typeof rawAuthentication !== "object" || rawAuthentication === null) {
    throw new WorkflowContractError(
      "ApprovalSignal.authentication is required; approval must carry auth context",
    );
  }
  const strength = authenticationStrength.parse(
    rawAuthentication.strength,
    "authentication.strength",
  );
  const method = requireNonEmptyString(
    rawAuthentication.method,
    "authentication.method",
  );
  const sessionId =
    rawAuthentication.sessionId === undefined
      ? undefined
      : requireNonEmptyString(
          rawAuthentication.sessionId,
          "authentication.sessionId",
        );
  const verifiedAt =
    rawAuthentication.verifiedAt === undefined
      ? undefined
      : requireIso8601(
          rawAuthentication.verifiedAt,
          "authentication.verifiedAt",
        );
  const decidedAt = requireIso8601(record.decidedAt, "decidedAt");
  const comment =
    record.comment === undefined
      ? undefined
      : requireNonEmptyString(record.comment, "comment");

  const authenticationBase: {
    strength: AuthenticationStrength;
    method: string;
  } = { strength, method };
  const authentication: AuthenticationContext =
    sessionId === undefined && verifiedAt === undefined
      ? authenticationBase
      : {
          ...authenticationBase,
          ...(sessionId === undefined ? {} : { sessionId }),
          ...(verifiedAt === undefined ? {} : { verifiedAt }),
        };

  const approval: ApprovalSignal = {
    signalType: "APPROVAL",
    signalId,
    workflowId,
    actionId,
    actionDigest,
    principal: { id: principalId, type: principalTypeValue },
    authentication,
    decision,
    decidedAt,
  };
  return comment === undefined ? approval : { ...approval, comment };
}

/** Validate any WorkflowSignal by its signalType discriminant. */
export function validateSignal(value: unknown): WorkflowSignal {
  if (typeof value !== "object" || value === null) {
    throw new WorkflowContractError(
      `WorkflowSignal must be an object, got ${JSON.stringify(value)}`,
    );
  }
  const record = value as Record<string, unknown>;
  const kind = signalType.parse(record.signalType, "signalType");
  const signalId = parseSignalId(record.signalId);
  const workflowId = parseWorkflowId(record.workflowId);
  switch (kind) {
    case "APPROVAL":
      return validateApprovalSignal(value);
    case "CANCEL": {
      const reason =
        record.reason === undefined
          ? undefined
          : requireNonEmptyString(record.reason, "reason");
      const requestedAt = requireIso8601(record.requestedAt, "requestedAt");
      return reason === undefined
        ? { signalType: "CANCEL", signalId, workflowId, requestedAt }
        : { signalType: "CANCEL", signalId, workflowId, reason, requestedAt };
    }
    case "RESUME": {
      const requestedAt = requireIso8601(record.requestedAt, "requestedAt");
      return { signalType: "RESUME", signalId, workflowId, requestedAt };
    }
  }
}

/**
 * Assert that an approval binds to the exact action digest and satisfies
 * the approval class. Throws WorkflowContractError when the digest does
 * not match the action being waited on, or when the authentication
 * strength is below the class requirement (SPEC-005 behavior 4).
 */
export function assertApprovalBinding(
  signal: ApprovalSignal,
  expectedActionId: ActionId,
  expectedActionDigest: ActionDigest,
  requiredStrength: AuthenticationStrength,
): void {
  if (signal.actionId !== expectedActionId) {
    throw new WorkflowContractError(
      `approval actionId ${signal.actionId} does not match awaited action ${expectedActionId}`,
    );
  }
  if (signal.actionDigest !== expectedActionDigest) {
    throw new WorkflowContractError(
      "approval actionDigest does not match the action payload digest",
    );
  }
  if (signal.authentication.strength !== requiredStrength) {
    throw new WorkflowContractError(
      `approval authentication strength ${signal.authentication.strength} is below required ${requiredStrength}`,
    );
  }
}
