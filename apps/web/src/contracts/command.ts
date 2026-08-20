/**
 * EP-033 M1 typed command dispatch (directive K/N, SPEC-006).
 *
 * The UI sends consequential actions to Nexus as typed command
 * requests bound to the canonical ActionRequest schema
 * (schemas/action-request.schema.json): capability_id comes from the
 * known vocabulary (never an arbitrary string), risk is a canonical
 * R0-R4 class, approval_class is canonical, idempotency_key is
 * validated, and invocation carries request/correlation/origin actor
 * context. The UI cannot fabricate capability names.
 *
 * Lifecycle is displayed, never transitioned, by the UI:
 * REQUESTED != EXECUTED != VERIFIED (directive D). Expired sessions
 * fail closed and sensitive actions are never queued for blind replay.
 */

import type { ActionRequest, InvocationContext } from "@nexus/contracts";
import {
  assertObject,
  assertString,
  assertUuid,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";
import type { AuthenticatedSession } from "./session";
import type { KnownCapabilityVocabulary } from "./capability";

export const RISK_CLASSES = ["R0", "R1", "R2", "R3", "R4"] as const;
export type RiskClass = (typeof RISK_CLASSES)[number];

export const APPROVAL_CLASSES = [
  "NONE",
  "POLICY",
  "HUMAN",
  "STRONG_HUMAN",
  "FOUR_EYES",
] as const;
export type ApprovalClass = (typeof APPROVAL_CLASSES)[number];

/**
 * Canonical SPEC-006 action lifecycle. The UI may display any stage;
 * it never advances the lifecycle itself. A backend ActionReceipt or
 * VerificationResult is the only authority for EXECUTED/VERIFIED.
 */
export const ACTION_LIFECYCLE = [
  "REQUESTED",
  "EVALUATED",
  "AWAITING_APPROVAL",
  "APPROVED",
  "EXECUTING",
  "VERIFYING",
  "SUCCEEDED",
  "FAILED",
  "COMPENSATING",
  "COMPENSATED",
  "REJECTED",
] as const;
export type ActionLifecycleStage = (typeof ACTION_LIFECYCLE)[number];

const COMMAND_REQUEST_FIELDS = new Set<string>([
  "action_id",
  "tenant_id",
  "principal_id",
  "capability_id",
  "idempotency_key",
  "risk",
  "approval_class",
  "reversal",
  "arguments",
  "expected_state",
  "invocation",
]);

export interface CommandRequestShape {
  action_id: string;
  tenant_id: string;
  principal_id: string;
  capability_id: string;
  idempotency_key: string;
  risk: RiskClass;
  approval_class: ApprovalClass;
  reversal: string;
  arguments: Record<string, unknown>;
  expected_state: Record<string, unknown>;
  invocation: InvocationContext;
}

/**
 * A validated command request. Constructed only from wire-shaped input
 * after vocabulary and shape validation: unknown capability ids are
 * rejected before any dispatch can occur.
 */
export class TypedCommandRequest {
  readonly action_id: string;
  readonly tenant_id: string;
  readonly principal_id: string;
  readonly capability_id: string;
  readonly idempotency_key: string;
  readonly risk: RiskClass;
  readonly approval_class: ApprovalClass;
  readonly reversal: string;
  readonly arguments: Record<string, unknown>;
  readonly expected_state: Record<string, unknown>;
  readonly invocation: InvocationContext;

  private constructor(shape: CommandRequestShape) {
    this.action_id = shape.action_id;
    this.tenant_id = shape.tenant_id;
    this.principal_id = shape.principal_id;
    this.capability_id = shape.capability_id;
    this.idempotency_key = shape.idempotency_key;
    this.risk = shape.risk;
    this.approval_class = shape.approval_class;
    this.reversal = shape.reversal;
    this.arguments = shape.arguments;
    this.expected_state = shape.expected_state;
    this.invocation = shape.invocation;
  }

  static fromWire(
    value: unknown,
    vocabulary: KnownCapabilityVocabulary,
    session: AuthenticatedSession,
  ): TypedCommandRequest {
    const obj = assertObject(value, "ActionRequest");
    rejectUnknownFields(obj, COMMAND_REQUEST_FIELDS, "ActionRequest");

    const capabilityId = assertString(obj.capability_id, "capability_id");
    if (!vocabulary.isKnown(capabilityId)) {
      throw new Spec006Error(
        ErrorCode.Vocabulary,
        `Unknown capability '${capabilityId}'; command refused`,
        session.correlation,
      );
    }

    const actionId = assertUuid(obj.action_id, "action_id");
    const tenantId = assertUuid(obj.tenant_id, "tenant_id");
    const principalId = assertUuid(obj.principal_id, "principal_id");

    if (tenantId !== session.tenant_id || principalId !== session.principal_id) {
      throw new Spec006Error(
        ErrorCode.Authorization,
        "Command tenant/principal does not match session",
        session.correlation,
      );
    }

    const idempotencyKey = assertString(obj.idempotency_key, "idempotency_key");
    if (idempotencyKey.length < 16 || idempotencyKey.length > 200) {
      throw new Spec006Error(
        ErrorCode.Validation,
        "idempotency_key must be between 16 and 200 characters",
        session.correlation,
      );
    }

    if (typeof obj.arguments !== "object" || obj.arguments === null || Array.isArray(obj.arguments)) {
      throw new Spec006Error(ErrorCode.Validation, "arguments must be an object", session.correlation);
    }
    if (
      typeof obj.expected_state !== "object" ||
      obj.expected_state === null ||
      Array.isArray(obj.expected_state)
    ) {
      throw new Spec006Error(
        ErrorCode.Validation,
        "expected_state must be an object",
        session.correlation,
      );
    }
    if (typeof obj.invocation !== "object" || obj.invocation === null) {
      throw new Spec006Error(ErrorCode.Validation, "invocation must be an object", session.correlation);
    }
    const invocation = obj.invocation as InvocationContext;
    if (
      typeof invocation.request_id !== "string" ||
      typeof invocation.correlation_id !== "string" ||
      typeof invocation.origin_system !== "string" ||
      typeof invocation.external_actor_id !== "string"
    ) {
      throw new Spec006Error(
        ErrorCode.Validation,
        "invocation requires request_id, correlation_id, origin_system, external_actor_id",
        session.correlation,
      );
    }

    const risk = obj.risk;
    if (typeof risk !== "string" || !(RISK_CLASSES as readonly string[]).includes(risk)) {
      throw new Spec006Error(ErrorCode.Vocabulary, `Unsupported risk '${String(risk)}'`, session.correlation);
    }
    const approvalClass = obj.approval_class;
    if (
      typeof approvalClass !== "string" ||
      !(APPROVAL_CLASSES as readonly string[]).includes(approvalClass)
    ) {
      throw new Spec006Error(
        ErrorCode.Vocabulary,
        `Unsupported approval_class '${String(approvalClass)}'`,
        session.correlation,
      );
    }

    return new TypedCommandRequest({
      action_id: actionId,
      tenant_id: tenantId,
      principal_id: principalId,
      capability_id: capabilityId,
      idempotency_key: idempotencyKey,
      risk: risk as RiskClass,
      approval_class: approvalClass as ApprovalClass,
      reversal: assertString(obj.reversal, "reversal"),
      arguments: obj.arguments as Record<string, unknown>,
      expected_state: obj.expected_state as Record<string, unknown>,
      invocation,
    });
  }

  /** Canonical ActionRequest wire shape (snake_case verbatim). */
  toWire(): ActionRequest {
    return {
      action_id: this.action_id,
      tenant_id: this.tenant_id,
      principal_id: this.principal_id,
      capability_id: this.capability_id,
      idempotency_key: this.idempotency_key,
      risk: this.risk,
      approval_class: this.approval_class,
      reversal: this.reversal,
      arguments: this.arguments,
      expected_state: this.expected_state,
      invocation: this.invocation,
    };
  }
}

/**
 * Auth-expiry gate (directive N). A consequential dispatch under an
 * expired or revoked session is refused, and the refusal is terminal
 * for that attempt: the action is never queued for blind replay.
 * The caller must re-authenticate and re-issue a fresh request.
 */
export class DispatchGate {
  /**
   * @returns the request unchanged when the session is active.
   * @throws Authentication/Authorization when the session is not
   *         active; the caller must not queue the request.
   */
  authorize(request: TypedCommandRequest, session: AuthenticatedSession, nowUnixS: number): TypedCommandRequest {
    session.requireActive(nowUnixS, request.invocation.correlation_id ?? session.correlation);
    return request;
  }
}

export interface MutationRefusal {
  code: "AUTH_EXPIRED" | "AUTH_REVOKED" | "SESSION_ACTIVE";
  correlation: string;
}

/**
 * Explicit refusal record: proves the gate refuses rather than queues.
 * A caller that respects the contract must treat AUTH_EXPIRED /
 * AUTH_REVOKED as terminal (re-authenticate, then re-issue manually).
 */
export function refuseOrPass(
  request: TypedCommandRequest,
  session: AuthenticatedSession,
  nowUnixS: number,
): { request: TypedCommandRequest; refusal: MutationRefusal } {
  const status = session.statusAt(nowUnixS);
  if (status === "ACTIVE") {
    return { request, refusal: { code: "SESSION_ACTIVE", correlation: session.correlation } };
  }
  return {
    request,
    refusal: {
      code: status === "EXPIRED" ? "AUTH_EXPIRED" : "AUTH_REVOKED",
      correlation: session.correlation,
    },
  };
}

/**
 * Lifecycle presentation helper: maps a lifecycle stage to what the UI
 * may truthfully claim. The UI may show the stage; it may never assert
 * a later stage without backend authority.
 */
export function lifecycleClaims(stage: ActionLifecycleStage): {
  requested: boolean;
  executed: boolean;
  verified: boolean;
} {
  return {
    requested: true,
    executed:
      stage === "EXECUTING" ||
      stage === "VERIFYING" ||
      stage === "SUCCEEDED" ||
      stage === "COMPENSATING" ||
      stage === "COMPENSATED",
    verified: stage === "VERIFYING" || stage === "SUCCEEDED" || stage === "COMPENSATED",
  };
}

export function riskToNumber(risk: RiskClass): number {
  return RISK_CLASSES.indexOf(risk);
}
