/**
 * Pure, deterministic step-gate machine (SPEC-023 behavior 5; ADR-010).
 *
 * A step-gated workflow runs a sequence of steps; each step awaits an
 * approval bound to its exact action digest, executes its effect through
 * an idempotent activity, and verifies the result before advancing. Any
 * failure or cancellation compensates executed steps in reverse order.
 * No engine imports; unit-tested directly. Specialized by the objective,
 * certification, remediation, and deployment workflows.
 */

import { signalKey, validateApprovalSignal } from "@nexus/workflows";
import type {
  ActionDigest,
  ActionId,
  ApprovalSignal,
  WorkflowId,
  WorkflowOutcome,
  WorkflowState,
} from "@nexus/workflows";

export type StepState =
  | "PENDING"
  | "AWAITING_APPROVAL"
  | "APPROVED"
  | "EXECUTING"
  | "VERIFIED"
  | "FAILED"
  | "COMPENSATED";

export interface StepRecord {
  readonly stepId: string;
  readonly title: string;
  readonly actionId: ActionId;
  readonly actionDigest: ActionDigest;
  readonly state: StepState;
}

export interface StepGateRecord {
  readonly workflowId: WorkflowId;
  /** Owner-facing label (e.g. objective title). */
  readonly label: string;
  /** Entity id carried by the seed (objectiveId, incidentId, ...). */
  readonly entityId: string;
  readonly steps: readonly StepRecord[];
  readonly currentIndex: number;
  readonly state: WorkflowState;
  readonly outcome: WorkflowOutcome | undefined;
  readonly observedSignalKeys: readonly string[];
  readonly updatedAt: string;
}

export type StepGateTransition =
  | { readonly kind: "advanced"; readonly record: StepGateRecord }
  | { readonly kind: "duplicate"; readonly record: StepGateRecord }
  | {
      readonly kind: "invalid";
      readonly record: StepGateRecord;
      readonly reason: string;
    }
  | {
      readonly kind: "ignored";
      readonly record: StepGateRecord;
      readonly reason: string;
    }
  | { readonly kind: "completed"; readonly record: StepGateRecord };

export interface StepGateSeed {
  readonly workflowId: WorkflowId;
  readonly label: string;
  readonly entityId: string;
  readonly steps: readonly {
    readonly stepId: string;
    readonly title: string;
    readonly actionId: ActionId;
    readonly actionDigest: ActionDigest;
  }[];
}

export function initialStepGateState(
  seed: StepGateSeed,
  nowIso: string,
): StepGateRecord {
  return {
    workflowId: seed.workflowId,
    label: seed.label,
    entityId: seed.entityId,
    steps: seed.steps.map((s) => ({ ...s, state: "PENDING" })),
    currentIndex: 0,
    state: "REQUESTED",
    outcome: undefined,
    observedSignalKeys: [],
    updatedAt: nowIso,
  };
}

export function isTerminalStepGateState(state: WorkflowState): boolean {
  return (
    state === "SUCCEEDED" ||
    state === "FAILED" ||
    state === "CANCELLED" ||
    state === "COMPENSATED" ||
    state === "TIMED_OUT" ||
    state === "REJECTED"
  );
}

/** Move the first step into AWAITING_APPROVAL and start the run. */
export function startStepGate(
  record: StepGateRecord,
  nowIso: string,
): StepGateRecord {
  const steps = record.steps.map((s, i) =>
    i === 0 ? { ...s, state: "AWAITING_APPROVAL" as const } : s,
  );
  return { ...record, steps, state: "AWAITING_APPROVAL", updatedAt: nowIso };
}

/** Current step awaiting decision, or undefined when complete. */
export function currentStep(record: StepGateRecord): StepRecord | undefined {
  return record.steps[record.currentIndex];
}

/**
 * Apply an approval signal against the CURRENT step. The digest must
 * match exactly; duplicates collapse on signalKey; invalid signals are
 * recorded and ignored (fail closed).
 */
export function applyStepGateSignal(
  record: StepGateRecord,
  rawSignal: unknown,
  nowIso: string,
): StepGateTransition {
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
  if (isTerminalStepGateState(record.state)) {
    return { kind: "ignored", record, reason: `workflow is ${record.state}` };
  }

  const step = currentStep(record);
  if (step === undefined || step.state !== "AWAITING_APPROVAL") {
    return {
      kind: "ignored",
      record,
      reason: `step ${step?.stepId ?? "?"} is not awaiting approval`,
    };
  }
  if (step.actionDigest !== signal.actionDigest) {
    return {
      kind: "invalid",
      record: appendObserved(record, key),
      reason: `approval digest does not match step ${step.stepId}`,
    };
  }

  const observed = appendObserved(record, key);
  const steps = record.steps.map((s, i) =>
    i === record.currentIndex ? { ...s, state: "APPROVED" as const } : s,
  );
  return {
    kind: "advanced",
    record: { ...observed, steps, state: "APPROVED", updatedAt: nowIso },
  };
}

/** Mark the approved step's effect as executing. */
export function beginStepExecution(
  record: StepGateRecord,
  nowIso: string,
): StepGateRecord {
  const steps = record.steps.map((s, i) =>
    i === record.currentIndex && s.state === "APPROVED"
      ? { ...s, state: "EXECUTING" as const }
      : s,
  );
  return { ...record, steps, state: "EXECUTING", updatedAt: nowIso };
}

/**
 * Step effect verified: complete the step and advance. When the last step
 * verifies, the workflow SUCCEEDS.
 */
export function completeStep(
  record: StepGateRecord,
  verified: boolean,
  nowIso: string,
): StepGateRecord {
  if (!verified) {
    return failStep(record, nowIso);
  }
  const isLast = record.currentIndex + 1 >= record.steps.length;
  const steps = record.steps.map((s, i) =>
    i === record.currentIndex ? { ...s, state: "VERIFIED" as const } : s,
  );
  if (isLast) {
    return {
      ...record,
      steps,
      state: "SUCCEEDED",
      outcome: "SUCCEEDED",
      updatedAt: nowIso,
    };
  }
  return {
    ...record,
    steps: steps.map((s, i) =>
      i === record.currentIndex + 1
        ? { ...s, state: "AWAITING_APPROVAL" as const }
        : s,
    ),
    currentIndex: record.currentIndex + 1,
    state: "AWAITING_APPROVAL",
    updatedAt: nowIso,
  };
}

/** Step effect failed: the workflow FAILS (compensation follows). */
export function failStep(
  record: StepGateRecord,
  nowIso: string,
): StepGateRecord {
  const steps = record.steps.map((s, i) =>
    i === record.currentIndex ? { ...s, state: "FAILED" as const } : s,
  );
  return {
    ...record,
    steps,
    state: "FAILED",
    outcome: "FAILED",
    updatedAt: nowIso,
  };
}

/** Per-step approval timeout: fail closed with TIMED_OUT. */
export function timeoutStepGate(
  record: StepGateRecord,
  nowIso: string,
): StepGateRecord {
  if (isTerminalStepGateState(record.state)) {
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
export function cancelStepGate(
  record: StepGateRecord,
  cancelAction: "CANCEL" | "COMPENSATE",
  nowIso: string,
): StepGateRecord {
  if (isTerminalStepGateState(record.state)) {
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

/** Finalize compensation of a cancelled/failed step-gated workflow. */
export function markStepGateCompensated(
  record: StepGateRecord,
  nowIso: string,
): StepGateRecord {
  const steps = record.steps.map((s) =>
    s.state === "EXECUTING" || s.state === "APPROVED" || s.state === "FAILED"
      ? { ...s, state: "COMPENSATED" as const }
      : s,
  );
  return {
    ...record,
    steps,
    state: "COMPENSATED",
    outcome: "COMPENSATED",
    updatedAt: nowIso,
  };
}

function appendObserved(record: StepGateRecord, key: string): StepGateRecord {
  return { ...record, observedSignalKeys: [...record.observedSignalKeys, key] };
}
