/**
 * Pure, deterministic milestone objective machine (SPEC-023 behavior 5;
 * ADR-010).
 *
 * Long-running objective: each milestone awaits an approval bound to its
 * exact action digest, then its effect is executed and verified; any
 * failure triggers reverse-order compensation of executed effects.
 * No engine imports; unit-tested directly.
 *
 * Per-milestone states: PENDING -> AWAITING_APPROVAL -> APPROVED ->
 * EXECUTING -> VERIFIED. Objective-level states come from the nexus
 * WorkflowState vocabulary; terminals are SUCCEEDED, FAILED, CANCELLED,
 * COMPENSATED, TIMED_OUT, REJECTED.
 */

import { signalKey, validateApprovalSignal } from "@nexus/workflows";
import type {
  ActionDigest,
  ActionId,
  ApprovalSignal,
  ObjectiveInput,
  WorkflowId,
  WorkflowOutcome,
  WorkflowState,
} from "@nexus/workflows";

export type MilestoneState =
  | "PENDING"
  | "AWAITING_APPROVAL"
  | "APPROVED"
  | "EXECUTING"
  | "VERIFIED"
  | "FAILED"
  | "COMPENSATED";

export interface MilestoneRecord {
  readonly milestoneId: string;
  readonly title: string;
  readonly actionId: ActionId;
  readonly actionDigest: ActionDigest;
  readonly state: MilestoneState;
}

export interface ObjectiveRecord {
  readonly workflowId: WorkflowId;
  readonly objectiveId: string;
  readonly title: string;
  readonly milestones: readonly MilestoneRecord[];
  readonly currentIndex: number;
  readonly state: WorkflowState;
  readonly outcome: WorkflowOutcome | undefined;
  readonly observedSignalKeys: readonly string[];
  readonly updatedAt: string;
}

export type ObjectiveTransition =
  | { readonly kind: "advanced"; readonly record: ObjectiveRecord }
  | { readonly kind: "duplicate"; readonly record: ObjectiveRecord }
  | {
      readonly kind: "invalid";
      readonly record: ObjectiveRecord;
      readonly reason: string;
    }
  | {
      readonly kind: "ignored";
      readonly record: ObjectiveRecord;
      readonly reason: string;
    }
  | { readonly kind: "completed"; readonly record: ObjectiveRecord };

export function initialObjectiveState(
  input: ObjectiveInput,
  nowIso: string,
): ObjectiveRecord {
  return {
    workflowId: input.workflowId,
    objectiveId: input.objectiveId,
    title: input.title,
    milestones: input.milestones.map((m) => ({
      milestoneId: m.milestoneId,
      title: m.title,
      actionId: m.actionId,
      actionDigest: m.actionDigest,
      state: "PENDING",
    })),
    currentIndex: 0,
    state: "REQUESTED",
    outcome: undefined,
    observedSignalKeys: [],
    updatedAt: nowIso,
  };
}

export function isTerminalObjectiveState(state: WorkflowState): boolean {
  return (
    state === "SUCCEEDED" ||
    state === "FAILED" ||
    state === "CANCELLED" ||
    state === "COMPENSATED" ||
    state === "TIMED_OUT" ||
    state === "REJECTED"
  );
}

/** Move the first milestone into AWAITING_APPROVAL and start the run. */
export function startObjective(
  record: ObjectiveRecord,
  nowIso: string,
): ObjectiveRecord {
  const milestones = record.milestones.map((m, i) =>
    i === 0 ? { ...m, state: "AWAITING_APPROVAL" as const } : m,
  );
  return {
    ...record,
    milestones,
    state: "AWAITING_APPROVAL",
    updatedAt: nowIso,
  };
}

/** Current milestone awaiting decision, or undefined when complete. */
export function currentMilestone(
  record: ObjectiveRecord,
): MilestoneRecord | undefined {
  return record.milestones[record.currentIndex];
}

/**
 * Apply an approval signal against the CURRENT milestone. The digest must
 * match exactly; duplicates collapse on signalKey; invalid signals are
 * recorded and ignored (fail closed).
 */
export function applyObjectiveSignal(
  record: ObjectiveRecord,
  rawSignal: unknown,
  nowIso: string,
): ObjectiveTransition {
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
  if (isTerminalObjectiveState(record.state)) {
    return { kind: "ignored", record, reason: `objective is ${record.state}` };
  }

  const milestone = currentMilestone(record);
  if (milestone === undefined || milestone.state !== "AWAITING_APPROVAL") {
    return {
      kind: "ignored",
      record,
      reason: `milestone ${milestone?.milestoneId ?? "?"} is not awaiting approval`,
    };
  }
  if (milestone.actionDigest !== signal.actionDigest) {
    return {
      kind: "invalid",
      record: appendObserved(record, key),
      reason: `approval digest does not match milestone ${milestone.milestoneId}`,
    };
  }

  const observed = appendObserved(record, key);
  const milestones = record.milestones.map((m, i) =>
    i === record.currentIndex ? { ...m, state: "APPROVED" as const } : m,
  );
  return {
    kind: "advanced",
    record: { ...observed, milestones, state: "APPROVED", updatedAt: nowIso },
  };
}

/** Mark the approved milestone's effect as executing. */
export function beginMilestoneExecution(
  record: ObjectiveRecord,
  nowIso: string,
): ObjectiveRecord {
  const milestones = record.milestones.map((m, i) =>
    i === record.currentIndex && m.state === "APPROVED"
      ? { ...m, state: "EXECUTING" as const }
      : m,
  );
  return { ...record, milestones, state: "EXECUTING", updatedAt: nowIso };
}

/**
 * Milestone effect verified: complete the milestone and advance. When the
 * last milestone verifies, the objective SUCCEEDS.
 */
export function completeMilestone(
  record: ObjectiveRecord,
  verified: boolean,
  nowIso: string,
): ObjectiveRecord {
  if (!verified) {
    return failMilestone(record, nowIso);
  }
  const isLast = record.currentIndex + 1 >= record.milestones.length;
  const milestones = record.milestones.map((m, i) =>
    i === record.currentIndex ? { ...m, state: "VERIFIED" as const } : m,
  );
  if (isLast) {
    return {
      ...record,
      milestones,
      state: "SUCCEEDED",
      outcome: "SUCCEEDED",
      updatedAt: nowIso,
    };
  }
  return {
    ...record,
    milestones: milestones.map((m, i) =>
      i === record.currentIndex + 1
        ? { ...m, state: "AWAITING_APPROVAL" as const }
        : m,
    ),
    currentIndex: record.currentIndex + 1,
    state: "AWAITING_APPROVAL",
    updatedAt: nowIso,
  };
}

/** Milestone effect failed: the objective FAILS (compensation follows). */
export function failMilestone(
  record: ObjectiveRecord,
  nowIso: string,
): ObjectiveRecord {
  const milestones = record.milestones.map((m, i) =>
    i === record.currentIndex ? { ...m, state: "FAILED" as const } : m,
  );
  return {
    ...record,
    milestones,
    state: "FAILED",
    outcome: "FAILED",
    updatedAt: nowIso,
  };
}

/** Per-milestone approval timeout: fail closed with TIMED_OUT. */
export function timeoutObjective(
  record: ObjectiveRecord,
  nowIso: string,
): ObjectiveRecord {
  if (isTerminalObjectiveState(record.state)) {
    return record;
  }
  return {
    ...record,
    state: "TIMED_OUT",
    outcome: "TIMED_OUT",
    updatedAt: nowIso,
  };
}

/** Cancel transition (CANCEL or COMPENSATE handled by the workflow). */
export function cancelObjective(
  record: ObjectiveRecord,
  cancelAction: "CANCEL" | "COMPENSATE",
  nowIso: string,
): ObjectiveRecord {
  if (isTerminalObjectiveState(record.state)) {
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
  return { ...record, state: "COMPENSATING", updatedAt: nowIso };
}

/** Finalize compensation of a cancelled/failed objective. */
export function markObjectiveCompensated(
  record: ObjectiveRecord,
  nowIso: string,
): ObjectiveRecord {
  const milestones = record.milestones.map((m) =>
    m.state === "EXECUTING" || m.state === "APPROVED" || m.state === "FAILED"
      ? { ...m, state: "COMPENSATED" as const }
      : m,
  );
  return {
    ...record,
    milestones,
    state: "COMPENSATED",
    outcome: "COMPENSATED",
    updatedAt: nowIso,
  };
}

function appendObserved(record: ObjectiveRecord, key: string): ObjectiveRecord {
  return { ...record, observedSignalKeys: [...record.observedSignalKeys, key] };
}
