/**
 * Pure, deterministic approval state machine (SPEC-023 behavior 7,
 * SPEC-005 behavior 4; ADR-010).
 *
 * This module contains NO engine imports and NO I/O: it is the domain rule
 * set for durable human approval, unit-tested directly. The Temporal
 * workflow (src/workflows/approval.ts) applies these transitions to its
 * durable state, so replay rebuilds the same decisions from history.
 *
 * Invariants enforced here:
 * - Approvals bind to the exact action digest and required authentication
 *   strength (assertApprovalBinding).
 * - Duplicate signals (same signalId) are idempotent: re-delivery is a
 *   no-op that returns the original state.
 * - Unauthorized transitions fail closed: a decision after a terminal
 *   state, or a signal below the required strength, never mutates state.
 * - Timeout and cancel are explicit transitions (EP-006 obligation 3).
 */

import {
  assertApprovalBinding,
  signalKey,
  validateApprovalSignal,
} from "@nexus/workflows";
import type {
  ActionDigest,
  ActionId,
  ApprovalDecision,
  ApprovalInput,
  ApprovalSignal,
  PrincipalRef,
  WorkflowId,
  WorkflowOutcome,
  WorkflowState,
} from "@nexus/workflows";
import { WorkflowContractError } from "@nexus/workflows";

export interface ApprovalRecord {
  readonly workflowId: WorkflowId;
  readonly actionId: ActionId;
  readonly actionDigest: ActionDigest;
  readonly requiredStrength: import("@nexus/workflows").AuthenticationStrength;
  readonly requester: PrincipalRef;
  readonly state: WorkflowState;
  readonly outcome: WorkflowOutcome | undefined;
  readonly decision: ApprovalDecision | undefined;
  readonly decidedAt: string | undefined;
  readonly decidedBy: string | undefined;
  /** Canonical signal keys observed so far (idempotency ledger). */
  readonly observedSignalKeys: readonly string[];
  /** Immutable approval signals observed so far (query surface). */
  readonly observedSignals: readonly ApprovalSignal[];
  /** ISO-8601 engine timestamp of the last transition. */
  readonly updatedAt: string;
}

export type ApprovalTransition =
  | { readonly kind: "accepted"; readonly record: ApprovalRecord }
  | { readonly kind: "duplicate"; readonly record: ApprovalRecord }
  | {
      readonly kind: "invalid";
      readonly record: ApprovalRecord;
      readonly reason: string;
    }
  | {
      readonly kind: "ignored";
      readonly record: ApprovalRecord;
      readonly reason: string;
    };

export function initialApprovalState(
  input: ApprovalInput,
  nowIso: string,
): ApprovalRecord {
  return {
    workflowId: input.workflowId,
    actionId: input.actionId,
    actionDigest: input.actionDigest,
    requiredStrength: input.requiredAuthenticationStrength,
    requester: input.principal,
    state: "REQUESTED",
    outcome: undefined,
    decision: undefined,
    decidedAt: undefined,
    decidedBy: undefined,
    observedSignalKeys: [],
    observedSignals: [],
    updatedAt: nowIso,
  };
}

export function isTerminalApprovalState(state: WorkflowState): boolean {
  return (
    state === "APPROVED" ||
    state === "REJECTED" ||
    state === "TIMED_OUT" ||
    state === "CANCELLED" ||
    state === "COMPENSATED" ||
    state === "FAILED"
  );
}

/**
 * Apply an approval signal to the record. Structural validation happens
 * first (validateApprovalSignal); the binding (digest + strength) is
 * enforced via assertApprovalBinding; duplicates collapse on signalKey.
 */
export function applyApprovalSignal(
  record: ApprovalRecord,
  rawSignal: unknown,
  nowIso: string,
): ApprovalTransition {
  let signal: ApprovalSignal;
  try {
    signal = validateApprovalSignal(rawSignal);
  } catch (error) {
    return {
      kind: "invalid",
      record,
      reason: error instanceof Error ? error.message : "malformed signal",
    };
  }

  const key = signalKey(signal);
  if (record.observedSignalKeys.includes(key)) {
    return { kind: "duplicate", record };
  }

  if (isTerminalApprovalState(record.state)) {
    return {
      kind: "ignored",
      record,
      reason: `approval cannot change state ${record.state}`,
    };
  }

  try {
    assertApprovalBinding(
      signal,
      record.actionId,
      record.actionDigest,
      record.requiredStrength,
    );
  } catch (error) {
    return {
      kind: "invalid",
      record: appendObserved(record, key, signal),
      reason: error instanceof Error ? error.message : "binding failed",
    };
  }

  const observed = appendObserved(record, key, signal);
  const nextState: WorkflowState =
    signal.decision === "APPROVE" ? "APPROVED" : "REJECTED";
  const outcome: WorkflowOutcome =
    signal.decision === "APPROVE" ? "SUCCEEDED" : "REJECTED";
  return {
    kind: "accepted",
    record: {
      ...observed,
      state: nextState,
      outcome,
      decision: signal.decision,
      decidedAt: signal.decidedAt,
      decidedBy: `${signal.principal.type}:${signal.principal.id}`,
      updatedAt: nowIso,
    },
  };
}

/** Explicit approval-timeout transition (EP-006 obligation 3). */
export function applyApprovalTimeout(
  record: ApprovalRecord,
  nowIso: string,
): ApprovalRecord {
  if (isTerminalApprovalState(record.state)) {
    return record;
  }
  return {
    ...record,
    state: "TIMED_OUT",
    outcome: "TIMED_OUT",
    updatedAt: nowIso,
  };
}

/**
 * Explicit cancel transition. CANCEL fails closed; COMPENSATE advances the
 * record to COMPENSATING (the workflow then runs compensation activities
 * and lands on COMPENSATED via markCompensated).
 */
export function applyCancel(
  record: ApprovalRecord,
  cancelAction: "CANCEL" | "COMPENSATE",
  nowIso: string,
): ApprovalRecord {
  if (isTerminalApprovalState(record.state)) {
    return record;
  }
  if (cancelAction === "CANCEL") {
    return {
      ...record,
      state: "CANCELLED",
      outcome: "CANCELLED",
      updatedAt: nowIso,
    };
  }
  return {
    ...record,
    state: "COMPENSATING",
    outcome: undefined,
    updatedAt: nowIso,
  };
}

/** Finalize a compensation flow started by applyCancel(COMPENSATE). */
export function markCompensated(
  record: ApprovalRecord,
  nowIso: string,
): ApprovalRecord {
  if (record.state !== "COMPENSATING") {
    throw new WorkflowContractError(
      `cannot mark compensated from state ${record.state}`,
    );
  }
  return {
    ...record,
    state: "COMPENSATED",
    outcome: "COMPENSATED",
    updatedAt: nowIso,
  };
}

function appendObserved(
  record: ApprovalRecord,
  key: string,
  signal: ApprovalSignal,
): ApprovalRecord {
  return {
    ...record,
    observedSignalKeys: [...record.observedSignalKeys, key],
    observedSignals: [...record.observedSignals, signal],
  };
}
