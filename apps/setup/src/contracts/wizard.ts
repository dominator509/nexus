/**
 * EP-035 M1 SetupWizard contract (SPEC-004 / SPEC-016).
 *
 * The wizard models STATE, not visual progress. A page being visited or
 * rendered never implies a step is complete, a provider is configured,
 * or a resource is healthy. Two structurally distinct step statuses
 * exist on purpose:
 *
 *   COMPLETE_LOCAL  - local checkpoint only (LOCAL_PROGRESS_SAVED)
 *   VERIFIED        - remote effect verified (REMOTE_EFFECT_VERIFIED)
 *
 * A COMPLETE_LOCAL step is never VERIFIED without an explicit remote
 * verification record, and a cached completed-step flag can never
 * satisfy readiness. Transitions are typed and validated: invalid
 * leaps (NOT_STARTED -> COMPLETED, FAILED -> COMPLETED without
 * recovery) are rejected.
 */

import {
  assertEnum,
  assertInt,
  assertNonNegativeInt,
  assertObject,
  assertString,
  assertUuid,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const WIZARD_STATES = [
  "NOT_STARTED",
  "IN_PROGRESS",
  "BLOCKED",
  "FAILED",
  "RECOVERY_REQUIRED",
  "COMPLETED",
] as const;
export type WizardState = (typeof WIZARD_STATES)[number];
const WIZARD_STATE_SET: ReadonlySet<WizardState> = new Set(WIZARD_STATES);

export const WIZARD_STEPS = [
  "DEPLOYMENT_CHOICE",
  "HARDWARE_PROFILE",
  "OWNER_BOOTSTRAP",
  "RECOVERY_MATERIAL",
  "EDGE_ENROLLMENT",
  "DISCOVERY",
  "INTEGRATION_REVIEW",
  "PLAN_REVIEW",
] as const;
export type WizardStep = (typeof WIZARD_STEPS)[number];
const WIZARD_STEP_SET: ReadonlySet<WizardStep> = new Set(WIZARD_STEPS);

export const WIZARD_STEP_STATUSES = [
  "PENDING",
  "IN_PROGRESS",
  "BLOCKED",
  "FAILED",
  "COMPLETE_LOCAL",
  "VERIFIED",
] as const;
export type WizardStepStatus = (typeof WIZARD_STEP_STATUSES)[number];
const WIZARD_STEP_STATUS_SET: ReadonlySet<WizardStepStatus> = new Set(
  WIZARD_STEP_STATUSES,
);

/** Allowed whole-wizard state transitions (typed, validated). */
const WIZARD_TRANSITIONS: Readonly<
  Record<WizardState, ReadonlySet<WizardState>>
> = {
  NOT_STARTED: new Set<WizardState>(["IN_PROGRESS"]),
  IN_PROGRESS: new Set<WizardState>([
    "BLOCKED",
    "FAILED",
    "RECOVERY_REQUIRED",
    "COMPLETED",
  ]),
  BLOCKED: new Set<WizardState>(["IN_PROGRESS", "RECOVERY_REQUIRED"]),
  FAILED: new Set<WizardState>(["RECOVERY_REQUIRED", "IN_PROGRESS"]),
  RECOVERY_REQUIRED: new Set<WizardState>(["IN_PROGRESS"]),
  COMPLETED: new Set<WizardState>([]),
};

/** Allowed per-step status transitions (typed, validated). */
const STEP_STATUS_TRANSITIONS: Readonly<
  Record<WizardStepStatus, ReadonlySet<WizardStepStatus>>
> = {
  PENDING: new Set<WizardStepStatus>(["IN_PROGRESS"]),
  IN_PROGRESS: new Set<WizardStepStatus>([
    "BLOCKED",
    "FAILED",
    "COMPLETE_LOCAL",
  ]),
  BLOCKED: new Set<WizardStepStatus>(["IN_PROGRESS"]),
  FAILED: new Set<WizardStepStatus>(["IN_PROGRESS"]),
  COMPLETE_LOCAL: new Set<WizardStepStatus>(["VERIFIED"]),
  VERIFIED: new Set<WizardStepStatus>([]),
};

export interface RemoteVerificationShape {
  verified_at_unix_s: number;
  verifier: string;
}

export interface WizardStepRecordShape {
  step: WizardStep;
  status: WizardStepStatus;
  last_transition_at_unix_s: number;
  verification?: RemoteVerificationShape | null;
}

export interface SetupWizardStateShape {
  state: WizardState;
  current_step: WizardStep;
  steps: Array<WizardStepRecordShape>;
  correlation_id: string;
  updated_at_unix_s: number;
}

export interface WizardBeginRequestShape {
  correlation_id: string;
}

export interface WizardAdvanceRequestShape {
  correlation_id: string;
  step: WizardStep;
  to_status: WizardStepStatus;
}

export interface WizardVerifyRequestShape {
  correlation_id: string;
  step: WizardStep;
  verification: RemoteVerificationShape;
}

const REMOTE_VERIFICATION_FIELDS = new Set<string>([
  "verified_at_unix_s",
  "verifier",
]);

function parseRemoteVerification(
  value: unknown,
  what: string,
): RemoteVerificationShape {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, REMOTE_VERIFICATION_FIELDS, what);
  return {
    verified_at_unix_s: assertNonNegativeInt(
      obj["verified_at_unix_s"],
      `${what}.verified_at_unix_s`,
    ),
    verifier: assertNonEmptyVerifier(obj["verifier"], `${what}.verifier`),
  };
}

function assertNonEmptyVerifier(value: unknown, what: string): string {
  const s = assertString(value, what);
  if (s.length === 0) {
    throw new Spec006Error(ErrorCode.Validation, `${what} must not be empty`);
  }
  return s;
}

const STEP_RECORD_FIELDS = new Set<string>([
  "step",
  "status",
  "last_transition_at_unix_s",
  "verification",
]);

export class WizardStepRecord {
  readonly step: WizardStep;
  readonly status: WizardStepStatus;
  readonly last_transition_at_unix_s: number;
  readonly verification: RemoteVerificationShape | undefined;

  /** Constructed only by wizard state operations; exposed for deterministic tests. */
  constructor(
    step: WizardStep,
    status: WizardStepStatus,
    lastTransitionAtUnixS: number,
    verification: RemoteVerificationShape | undefined,
  ) {
    this.step = step;
    this.status = status;
    this.last_transition_at_unix_s = lastTransitionAtUnixS;
    this.verification = verification;
  }

  static parse(value: unknown, what = "wizard step record"): WizardStepRecord {
    const obj = assertObject(value, what);
    rejectUnknownFields(obj, STEP_RECORD_FIELDS, what);
    const step = assertEnum(obj["step"], WIZARD_STEP_SET, `${what}.step`);
    const status = assertEnum(
      obj["status"],
      WIZARD_STEP_STATUS_SET,
      `${what}.status`,
    );
    const lastTransition = assertNonNegativeInt(
      obj["last_transition_at_unix_s"],
      `${what}.last_transition_at_unix_s`,
    );
    const verificationRaw = obj["verification"];
    if (verificationRaw !== undefined && verificationRaw !== null) {
      const verification = parseRemoteVerification(
        verificationRaw,
        `${what}.verification`,
      );
      if (status !== "VERIFIED") {
        throw new Spec006Error(
          ErrorCode.Validation,
          `${what} carries a verification record but status is ${status}; only VERIFIED steps may carry one`,
        );
      }
      return new WizardStepRecord(step, status, lastTransition, verification);
    }
    if (status === "VERIFIED") {
      throw new Spec006Error(
        ErrorCode.Verification,
        `${what} is VERIFIED but has no verification record`,
      );
    }
    return new WizardStepRecord(step, status, lastTransition, undefined);
  }

  toJSON(): WizardStepRecordShape {
    return {
      step: this.step,
      status: this.status,
      last_transition_at_unix_s: this.last_transition_at_unix_s,
      ...(this.verification === undefined
        ? {}
        : { verification: this.verification }),
    };
  }
}

export function isValidWizardStateTransition(
  from: WizardState,
  to: WizardState,
): boolean {
  const allowed = WIZARD_TRANSITIONS[from];
  return allowed !== undefined && allowed.has(to);
}

export function isValidStepStatusTransition(
  from: WizardStepStatus,
  to: WizardStepStatus,
): boolean {
  const allowed = STEP_STATUS_TRANSITIONS[from];
  return allowed !== undefined && allowed.has(to);
}

const WIZARD_STATE_FIELDS = new Set<string>([
  "state",
  "current_step",
  "steps",
  "correlation_id",
  "updated_at_unix_s",
]);

export class SetupWizardState {
  readonly state: WizardState;
  readonly current_step: WizardStep;
  readonly steps: ReadonlyArray<WizardStepRecord>;
  readonly correlation_id: string;
  readonly updated_at_unix_s: number;

  private constructor(
    state: WizardState,
    currentStep: WizardStep,
    steps: ReadonlyArray<WizardStepRecord>,
    correlationId: string,
    updatedAtUnixS: number,
  ) {
    this.state = state;
    this.current_step = currentStep;
    this.steps = steps;
    this.correlation_id = correlationId;
    this.updated_at_unix_s = updatedAtUnixS;
  }

  static parse(value: unknown, what = "setup wizard state"): SetupWizardState {
    const obj = assertObject(value, what);
    rejectUnknownFields(obj, WIZARD_STATE_FIELDS, what);
    const state = assertEnum(obj["state"], WIZARD_STATE_SET, `${what}.state`);
    const currentStep = assertEnum(
      obj["current_step"],
      WIZARD_STEP_SET,
      `${what}.current_step`,
    );
    const stepsRaw = obj["steps"];
    if (!Array.isArray(stepsRaw)) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `${what}.steps must be an array`,
      );
    }
    const steps = stepsRaw.map((entry) =>
      WizardStepRecord.parse(entry, `${what}.steps entry`),
    );
    const seen = new Set<WizardStep>();
    for (const step of steps) {
      if (seen.has(step.step)) {
        throw new Spec006Error(
          ErrorCode.Validation,
          `${what}.steps contains duplicate step '${step.step}'`,
        );
      }
      seen.add(step.step);
    }
    for (const stepId of WIZARD_STEPS) {
      if (!seen.has(stepId)) {
        throw new Spec006Error(
          ErrorCode.Validation,
          `${what}.steps is missing step '${stepId}'`,
        );
      }
    }
    return new SetupWizardState(
      state,
      currentStep,
      steps,
      assertUuid(obj["correlation_id"], `${what}.correlation_id`),
      assertNonNegativeInt(
        obj["updated_at_unix_s"],
        `${what}.updated_at_unix_s`,
      ),
    );
  }

  /** Create the canonical NOT_STARTED wizard with every step PENDING. */
  static notStarted(correlationId: string, atUnixS: number): SetupWizardState {
    const steps = WIZARD_STEPS.map(
      (step) => new WizardStepRecord(step, "PENDING", atUnixS, undefined),
    );
    return new SetupWizardState(
      "NOT_STARTED",
      WIZARD_STEPS[0],
      steps,
      correlationId,
      atUnixS,
    );
  }

  stepRecord(step: WizardStep): WizardStepRecord {
    const record = this.steps.find((entry) => entry.step === step);
    if (record === undefined) {
      throw new Spec006Error(
        ErrorCode.Internal,
        `wizard has no record for step '${step}'`,
      );
    }
    return record;
  }

  /**
   * Advance the whole wizard state. COMPLETED additionally requires
   * every step to be VERIFIED: a wizard with unverified steps can never
   * be marked complete.
   */
  advance(toState: WizardState, atUnixS: number): SetupWizardState {
    if (!isValidWizardStateTransition(this.state, toState)) {
      throw new Spec006Error(
        ErrorCode.Policy,
        `invalid wizard transition ${this.state} -> ${toState}`,
        this.correlation_id,
      );
    }
    if (toState === "COMPLETED") {
      const unverified = this.steps.filter(
        (entry) => entry.status !== "VERIFIED",
      );
      if (unverified.length > 0) {
        throw new Spec006Error(
          ErrorCode.Policy,
          `wizard cannot complete with unverified steps: ${unverified
            .map((entry) => entry.step)
            .join(",")}`,
          this.correlation_id,
        );
      }
    }
    return new SetupWizardState(
      toState,
      this.current_step,
      this.steps,
      this.correlation_id,
      atUnixS,
    );
  }

  /** Advance one step's status (typed transition; VERIFIED needs a record). */
  advanceStep(
    step: WizardStep,
    toStatus: WizardStepStatus,
    atUnixS: number,
    verification?: RemoteVerificationShape,
  ): SetupWizardState {
    const record = this.stepRecord(step);
    if (toStatus !== "VERIFIED" && verification !== undefined) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `step ${step} cannot carry a verification record with status ${toStatus}`,
        this.correlation_id,
      );
    }
    if (!isValidStepStatusTransition(record.status, toStatus)) {
      throw new Spec006Error(
        ErrorCode.Policy,
        `invalid step transition ${step}: ${record.status} -> ${toStatus}`,
        this.correlation_id,
      );
    }
    if (toStatus === "VERIFIED" && verification === undefined) {
      throw new Spec006Error(
        ErrorCode.Verification,
        `step ${step} cannot become VERIFIED without a verification record`,
        this.correlation_id,
      );
    }
    const updated: Array<WizardStepRecord> = this.steps.map((entry) => {
      if (entry.step !== step) {
        return entry;
      }
      return new WizardStepRecord(step, toStatus, atUnixS, verification);
    });
    return new SetupWizardState(
      this.state,
      step,
      updated,
      this.correlation_id,
      atUnixS,
    );
  }

  toJSON(): SetupWizardStateShape {
    return {
      state: this.state,
      current_step: this.current_step,
      steps: this.steps.map((entry) => entry.toJSON()),
      correlation_id: this.correlation_id,
      updated_at_unix_s: this.updated_at_unix_s,
    };
  }
}

const WIZARD_BEGIN_FIELDS = new Set<string>(["correlation_id"]);
const WIZARD_ADVANCE_FIELDS = new Set<string>([
  "correlation_id",
  "step",
  "to_status",
]);
const WIZARD_VERIFY_FIELDS = new Set<string>([
  "correlation_id",
  "step",
  "verification",
]);

export class WizardAdvanceRequest {
  readonly correlation_id: string;
  readonly step: WizardStep;
  readonly to_status: WizardStepStatus;

  private constructor(
    correlationId: string,
    step: WizardStep,
    toStatus: WizardStepStatus,
  ) {
    this.correlation_id = correlationId;
    this.step = step;
    this.to_status = toStatus;
  }

  static parse(value: unknown): WizardAdvanceRequest {
    const obj = assertObject(value, "wizard advance request");
    rejectUnknownFields(obj, WIZARD_ADVANCE_FIELDS, "wizard advance request");
    return new WizardAdvanceRequest(
      assertUuid(
        obj["correlation_id"],
        "wizard advance request.correlation_id",
      ),
      assertEnum(obj["step"], WIZARD_STEP_SET, "wizard advance request.step"),
      assertEnum(
        obj["to_status"],
        WIZARD_STEP_STATUS_SET,
        "wizard advance request.to_status",
      ),
    );
  }
}

export class WizardVerifyRequest {
  readonly correlation_id: string;
  readonly step: WizardStep;
  readonly verification: RemoteVerificationShape;

  private constructor(
    correlationId: string,
    step: WizardStep,
    verification: RemoteVerificationShape,
  ) {
    this.correlation_id = correlationId;
    this.step = step;
    this.verification = verification;
  }

  static parse(value: unknown): WizardVerifyRequest {
    const obj = assertObject(value, "wizard verify request");
    rejectUnknownFields(obj, WIZARD_VERIFY_FIELDS, "wizard verify request");
    return new WizardVerifyRequest(
      assertUuid(obj["correlation_id"], "wizard verify request.correlation_id"),
      assertEnum(obj["step"], WIZARD_STEP_SET, "wizard verify request.step"),
      parseRemoteVerification(
        obj["verification"],
        "wizard verify request.verification",
      ),
    );
  }
}

/**
 * Provider-neutral SetupWizard port. M1 declares the boundary; the real
 * durable implementation is owned by M2+.
 */
export interface SetupWizardPort {
  begin(request: WizardBeginRequestShape): SetupWizardState;
  advance(request: WizardAdvanceRequestShape): SetupWizardState;
  verifyRemote(request: WizardVerifyRequestShape): SetupWizardState;
}
