/**
 * EP-035 M1 RecoveryFlow contract (SPEC-004 / SPEC-016).
 *
 * Recovery is a real contract, not "catch an error and restart the
 * wizard". The flow distinguishes retryable, non-retryable, resume
 * checkpoint, reconcile, rollback, reauthenticate, reset, and manual
 * intervention outcomes. The no-blind-replay invariant is locked:
 *
 *   UNKNOWN WHETHER EXTERNAL MUTATION OCCURRED
 *     -> RECONCILE FIRST
 *     -> RETRY ONLY IF SAFE
 *
 * The RecoveryKit value object binds the canonical recovery-kit schema
 * (schemas/auth/recovery-kit.schema.json) verbatim.
 */

import {
  assertEnum,
  assertNonEmptyString,
  assertNonNegativeInt,
  assertObject,
  assertUuid,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const RECOVERY_MATERIAL_KINDS = [
  "RECOVERY_CODES",
  "OFFLINE_PASSPHRASE",
  "DEVICE_BACKUP",
] as const;
export type RecoveryMaterialKind = (typeof RECOVERY_MATERIAL_KINDS)[number];
const RECOVERY_MATERIAL_KIND_SET: ReadonlySet<RecoveryMaterialKind> = new Set(
  RECOVERY_MATERIAL_KINDS,
);

export const RECOVERY_FAILURE_CLASSES = [
  "AMBIGUOUS",
  "VALIDATION",
  "AUTHORIZATION",
  "UNAVAILABLE",
  "TIMEOUT",
  "CONFLICT",
  "INTERNAL",
] as const;
export type RecoveryFailureClass = (typeof RECOVERY_FAILURE_CLASSES)[number];
const RECOVERY_FAILURE_CLASS_SET: ReadonlySet<RecoveryFailureClass> = new Set(
  RECOVERY_FAILURE_CLASSES,
);

export const RECOVERY_MUTATION_STATES = ["UNKNOWN", "RECONCILED"] as const;
export type RecoveryMutationState = (typeof RECOVERY_MUTATION_STATES)[number];
const RECOVERY_MUTATION_STATE_SET: ReadonlySet<RecoveryMutationState> = new Set(
  RECOVERY_MUTATION_STATES,
);

export const RECOVERY_OUTCOMES = [
  "RETRYABLE",
  "NON_RETRYABLE",
  "RESUME_CHECKPOINT",
  "RECONCILE",
  "ROLLBACK",
  "REAUTHENTICATE",
  "RESET",
  "MANUAL_INTERVENTION",
] as const;
export type RecoveryOutcome = (typeof RECOVERY_OUTCOMES)[number];
const RECOVERY_OUTCOME_SET: ReadonlySet<RecoveryOutcome> = new Set(
  RECOVERY_OUTCOMES,
);

/**
 * Canonical RecoveryKit value object. Field names and enums are the
 * canonical snake_case wire names from schemas/auth/recovery-kit.schema.json
 * verbatim; parity is enforced by ep035_unit_schema_parity tests.
 */
export interface RecoveryKitShape {
  kit_id: string;
  principal_id: string;
  tenant_id: string;
  material_kind: RecoveryMaterialKind;
  created_at_unix_s: number;
  expires_at_unix_s: number;
  correlation: string;
}

const RECOVERY_KIT_FIELDS = new Set<string>([
  "kit_id",
  "principal_id",
  "tenant_id",
  "material_kind",
  "created_at_unix_s",
  "expires_at_unix_s",
  "correlation",
]);

export class RecoveryKit {
  readonly kit_id: string;
  readonly principal_id: string;
  readonly tenant_id: string;
  readonly material_kind: RecoveryMaterialKind;
  readonly created_at_unix_s: number;
  readonly expires_at_unix_s: number;
  readonly correlation: string;

  private constructor(
    kitId: string,
    principalId: string,
    tenantId: string,
    materialKind: RecoveryMaterialKind,
    createdAtUnixS: number,
    expiresAtUnixS: number,
    correlation: string,
  ) {
    this.kit_id = kitId;
    this.principal_id = principalId;
    this.tenant_id = tenantId;
    this.material_kind = materialKind;
    this.created_at_unix_s = createdAtUnixS;
    this.expires_at_unix_s = expiresAtUnixS;
    this.correlation = correlation;
  }

  static parse(value: unknown, what = "recovery kit"): RecoveryKit {
    const obj = assertObject(value, what);
    rejectUnknownFields(obj, RECOVERY_KIT_FIELDS, what);
    const created = assertNonNegativeInt(
      obj["created_at_unix_s"],
      `${what}.created_at_unix_s`,
    );
    const expires = assertNonNegativeInt(
      obj["expires_at_unix_s"],
      `${what}.expires_at_unix_s`,
    );
    if (expires <= created) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `${what} expires_at_unix_s must be after created_at_unix_s`,
      );
    }
    return new RecoveryKit(
      assertUuid(obj["kit_id"], `${what}.kit_id`),
      assertUuid(obj["principal_id"], `${what}.principal_id`),
      assertUuid(obj["tenant_id"], `${what}.tenant_id`),
      assertEnum(
        obj["material_kind"],
        RECOVERY_MATERIAL_KIND_SET,
        `${what}.material_kind`,
      ),
      created,
      expires,
      assertUuid(obj["correlation"], `${what}.correlation`),
    );
  }

  isExpired(nowUnixS: number): boolean {
    return nowUnixS > this.expires_at_unix_s;
  }

  toJSON(): RecoveryKitShape {
    return {
      kit_id: this.kit_id,
      principal_id: this.principal_id,
      tenant_id: this.tenant_id,
      material_kind: this.material_kind,
      created_at_unix_s: this.created_at_unix_s,
      expires_at_unix_s: this.expires_at_unix_s,
      correlation: this.correlation,
    };
  }
}

export interface RecoveryEvidenceShape {
  failure_class: RecoveryFailureClass;
  mutation_known: boolean;
  mutation_occurred?: boolean;
  mutation_state?: RecoveryMutationState;
  correlation_id?: string;
}

const RECOVERY_EVIDENCE_FIELDS = new Set<string>([
  "failure_class",
  "mutation_known",
  "mutation_occurred",
  "mutation_state",
  "correlation_id",
]);

export class RecoveryEvidence {
  readonly failure_class: RecoveryFailureClass;
  readonly mutation_known: boolean;
  readonly mutation_occurred: boolean | undefined;
  readonly mutation_state: RecoveryMutationState | undefined;
  readonly correlation_id: string | undefined;

  private constructor(
    failureClass: RecoveryFailureClass,
    mutationKnown: boolean,
    mutationOccurred: boolean | undefined,
    mutationState: RecoveryMutationState | undefined,
    correlationId: string | undefined,
  ) {
    this.failure_class = failureClass;
    this.mutation_known = mutationKnown;
    this.mutation_occurred = mutationOccurred;
    this.mutation_state = mutationState;
    this.correlation_id = correlationId;
  }

  static parse(value: unknown): RecoveryEvidence {
    const obj = assertObject(value, "recovery evidence");
    rejectUnknownFields(obj, RECOVERY_EVIDENCE_FIELDS, "recovery evidence");
    const mutationKnown =
      typeof obj["mutation_known"] === "boolean"
        ? obj["mutation_known"]
        : undefined;
    if (mutationKnown === undefined) {
      throw new Spec006Error(
        ErrorCode.Validation,
        "recovery evidence.mutation_known must be a boolean",
      );
    }
    const occurredRaw = obj["mutation_occurred"];
    const occurred =
      occurredRaw === undefined ? undefined : Boolean(occurredRaw);
    const mutationStateRaw = obj["mutation_state"];
    const mutationState =
      mutationStateRaw === undefined
        ? undefined
        : assertEnum(
            mutationStateRaw,
            RECOVERY_MUTATION_STATE_SET,
            "recovery evidence.mutation_state",
          );
    const correlationRaw = obj["correlation_id"];
    const correlation =
      correlationRaw === undefined
        ? undefined
        : assertUuid(correlationRaw, "recovery evidence.correlation_id");
    return new RecoveryEvidence(
      assertEnum(
        obj["failure_class"],
        RECOVERY_FAILURE_CLASS_SET,
        "recovery evidence.failure_class",
      ),
      mutationKnown,
      occurred,
      mutationState,
      correlation,
    );
  }
}

export interface RecoveryDecisionShape {
  outcome: RecoveryOutcome;
  mutation_state: RecoveryMutationState;
  retry_safe: boolean;
  detail: string;
}

export class RecoveryDecision {
  readonly outcome: RecoveryOutcome;
  readonly mutation_state: RecoveryMutationState;
  readonly retry_safe: boolean;
  readonly detail: string;

  /** Constructed only by decideRecovery; exposed for deterministic tests. */
  constructor(
    outcome: RecoveryOutcome,
    mutationState: RecoveryMutationState,
    retrySafe: boolean,
    detail: string,
  ) {
    this.outcome = outcome;
    this.mutation_state = mutationState;
    this.retry_safe = retrySafe;
    this.detail = detail;
  }

  toJSON(): RecoveryDecisionShape {
    return {
      outcome: this.outcome,
      mutation_state: this.mutation_state,
      retry_safe: this.retry_safe,
      detail: this.detail,
    };
  }
}

/**
 * Deterministic no-blind-replay recovery decision.
 *
 * - AMBIGUOUS (mutation outcome unknown) -> RECONCILE; retry is safe
 *   only after the mutation state is RECONCILED.
 * - UNAVAILABLE/TIMEOUT with known no-mutation -> RETRYABLE and safe.
 * - UNAVAILABLE/TIMEOUT with unknown mutation -> RECONCILE (never
 *   blind retry).
 * - VALIDATION -> NON_RETRYABLE (correct the input first).
 * - AUTHORIZATION -> REAUTHENTICATE (never retry-safe).
 * - CONFLICT -> RESUME_CHECKPOINT (reconcile local state, do not replay
 *   the mutation).
 * - INTERNAL -> MANUAL_INTERVENTION.
 */
export function decideRecovery(evidence: RecoveryEvidence): RecoveryDecision {
  const unknown =
    evidence.mutation_state === undefined ? "UNKNOWN" : evidence.mutation_state;
  switch (evidence.failure_class) {
    case "AMBIGUOUS":
      // AUD-045: AMBIGUOUS + RECONCILED is NOT retry-safe by itself.
      // Retrying after an ambiguous provider outcome can duplicate a
      // consequential effect unless there is an EXPLICIT negative
      // mutation observation (mutation_occurred === false) and the
      // mutation is known. A reconciled state without that observation
      // is still unsafe to retry.
      if (
        evidence.mutation_state === "RECONCILED" &&
        evidence.mutation_known &&
        evidence.mutation_occurred === false
      ) {
        return new RecoveryDecision(
          "RETRYABLE",
          "RECONCILED",
          true,
          "mutation reconciled with explicit negative observation; retry is safe",
        );
      }
      return new RecoveryDecision(
        "RECONCILE",
        "UNKNOWN",
        false,
        "external mutation outcome unknown; reconcile before retry",
      );
    case "UNAVAILABLE":
    case "TIMEOUT":
      if (evidence.mutation_known && evidence.mutation_occurred === false) {
        return new RecoveryDecision(
          "RETRYABLE",
          unknown,
          true,
          "no mutation occurred; retry is safe",
        );
      }
      return new RecoveryDecision(
        "RECONCILE",
        "UNKNOWN",
        false,
        "mutation outcome unknown; reconcile before retry",
      );
    case "VALIDATION":
      return new RecoveryDecision(
        "NON_RETRYABLE",
        unknown,
        false,
        "input must be corrected; retry not safe",
      );
    case "AUTHORIZATION":
      return new RecoveryDecision(
        "REAUTHENTICATE",
        unknown,
        false,
        "authorization required again",
      );
    case "CONFLICT":
      return new RecoveryDecision(
        "RESUME_CHECKPOINT",
        unknown,
        false,
        "conflicting state; resume from last checkpoint",
      );
    case "INTERNAL":
      return new RecoveryDecision(
        "MANUAL_INTERVENTION",
        unknown,
        false,
        "internal failure; manual intervention required",
      );
  }
}

/** Provider-neutral RecoveryFlow port. M1 declares the boundary. */
export interface RecoveryFlowPort {
  decide(request: RecoveryEvidenceShape): RecoveryDecisionShape;
}
