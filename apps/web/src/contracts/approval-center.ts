/**
 * EP-033 M1 approval center contract (directive L/M, SPEC-017
 * behavior 4).
 *
 * Approval classes are canonical and remain distinct: HUMAN,
 * STRONG_HUMAN, FOUR_EYES, POLICY are never collapsed into a generic
 * "Approve" boolean. The approval card displays the exact action,
 * target, risk, external effects, cost, reversibility, requester, and
 * expiration.
 *
 * FOUR_EYES requires two DISTINCT principals: one session/principal
 * can never satisfy both approvals (directive M).
 */

import {
  assertEnum,
  assertInt,
  assertObject,
  assertString,
  assertUuid,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";
import { APPROVAL_CLASSES, type ApprovalClass } from "./command";

export const APPROVAL_STATES = [
  "PENDING",
  "APPROVED",
  "DENIED",
  "EXPIRED",
  "REVOKED",
] as const;
export type ApprovalState = (typeof APPROVAL_STATES)[number];

export const APPROVAL_ACTIONS = ["APPROVE", "DENY"] as const;
export type ApprovalActionKind = (typeof APPROVAL_ACTIONS)[number];

const APPROVAL_CARD_FIELDS = new Set<string>([
  "approval_id",
  "action_id",
  "capability_id",
  "approval_class",
  "risk",
  "action_label",
  "target",
  "external_effects",
  "cost",
  "reversibility",
  "requester_id",
  "expires_at_unix_s",
  "correlation",
]);

export interface ApprovalCardShape {
  approval_id: string;
  action_id: string;
  capability_id: string;
  approval_class: ApprovalClass;
  risk: string;
  action_label: string;
  target: string;
  external_effects: string;
  cost: string;
  reversibility: string;
  requester_id: string;
  expires_at_unix_s: number;
  correlation: string;
}

/**
 * An approval card as presented. `approval_class` is preserved
 * verbatim: the UI renders the required class semantics; it never
 * flattens them.
 */
export class ApprovalCard {
  readonly approval_id: string;
  readonly action_id: string;
  readonly capability_id: string;
  readonly approval_class: ApprovalClass;
  readonly risk: string;
  readonly action_label: string;
  readonly target: string;
  readonly external_effects: string;
  readonly cost: string;
  readonly reversibility: string;
  readonly requester_id: string;
  readonly expires_at_unix_s: number;
  readonly correlation: string;

  private constructor(shape: ApprovalCardShape) {
    this.approval_id = shape.approval_id;
    this.action_id = shape.action_id;
    this.capability_id = shape.capability_id;
    this.approval_class = shape.approval_class;
    this.risk = shape.risk;
    this.action_label = shape.action_label;
    this.target = shape.target;
    this.external_effects = shape.external_effects;
    this.cost = shape.cost;
    this.reversibility = shape.reversibility;
    this.requester_id = shape.requester_id;
    this.expires_at_unix_s = shape.expires_at_unix_s;
    this.correlation = shape.correlation;
  }

  static fromWire(value: unknown): ApprovalCard {
    const obj = assertObject(value, "ApprovalCard");
    rejectUnknownFields(obj, APPROVAL_CARD_FIELDS, "ApprovalCard");
    return new ApprovalCard({
      approval_id: assertUuid(obj.approval_id, "approval_id"),
      action_id: assertUuid(obj.action_id, "action_id"),
      capability_id: assertString(obj.capability_id, "capability_id"),
      approval_class: assertEnum(
        obj.approval_class,
        new Set<ApprovalClass>(APPROVAL_CLASSES),
        "approval_class",
      ),
      risk: assertString(obj.risk, "risk"),
      action_label: assertString(obj.action_label, "action_label"),
      target: assertString(obj.target, "target"),
      external_effects: assertString(obj.external_effects, "external_effects"),
      cost: assertString(obj.cost, "cost"),
      reversibility: assertString(obj.reversibility, "reversibility"),
      requester_id: assertUuid(obj.requester_id, "requester_id"),
      expires_at_unix_s: assertInt(obj.expires_at_unix_s, "expires_at_unix_s"),
      correlation: assertUuid(obj.correlation, "correlation"),
    });
  }

  isExpired(nowUnixS: number): boolean {
    return nowUnixS >= this.expires_at_unix_s;
  }

  /**
   * The class-specific approval requirement. FOUR_EYES demands two
   * distinct principals; the UI must present that requirement instead
   * of a single approve button.
   */
  requiresTwoPrincipals(): boolean {
    return this.approval_class === "FOUR_EYES";
  }
}

/**
 * A recorded approval action. Carries the acting principal and the
 * approval class it satisfies; used by the four-eyes matcher.
 */
export class ApprovalAction {
  readonly approval_id: string;
  readonly action: ApprovalActionKind;
  readonly principal_id: string;
  readonly recorded_at_unix_s: number;

  private constructor(
    approvalId: string,
    action: ApprovalActionKind,
    principalId: string,
    recordedAt: number,
  ) {
    this.approval_id = approvalId;
    this.action = action;
    this.principal_id = principalId;
    this.recorded_at_unix_s = recordedAt;
  }

  static record(
    approvalId: string,
    action: ApprovalActionKind,
    principalId: string,
    recordedAt: number,
  ): ApprovalAction {
    if (typeof approvalId !== "string" || typeof principalId !== "string") {
      throw new Spec006Error(ErrorCode.Validation, "approval id and principal id are required");
    }
    return new ApprovalAction(approvalId, action, principalId, recordedAt);
  }
}

/**
 * FOUR_EYES satisfaction (directive M). Requires two APPROVE records
 * from two DISTINCT principal ids. A single principal approving twice
 * can never satisfy the requirement; a principal approving their own
 * request is also refused (requester exclusion).
 */
export class FourEyesRecord {
  readonly approval_id: string;
  readonly #approves: Map<string, number> = new Map();

  constructor(approvalId: string) {
    this.approval_id = approvalId;
  }

  apply(action: ApprovalAction): void {
    if (action.approval_id !== this.approval_id) {
      throw new Spec006Error(
        ErrorCode.Conflict,
        "Approval action targets a different approval",
      );
    }
    if (action.action === "APPROVE") {
      const existing = this.#approves.get(action.principal_id);
      this.#approves.set(action.principal_id, (existing ?? 0) + 1);
    }
  }

  /** Distinct approving principals observed so far. */
  distinctApprovers(): ReadonlyArray<string> {
    return [...this.#approves.keys()];
  }

  /**
   * Satisfied only when at least two distinct principals approved.
   * Two clicks from one account never satisfy FOUR_EYES.
   */
  isSatisfied(requesterId: string): boolean {
    const approvers = [...this.#approves.keys()].filter((p) => p !== requesterId);
    return approvers.length >= 2;
  }

  /**
   * Fail-closed check for adding an approval: refuses when the
   * principal already approved (a duplicate is a conflict, never a
   * second distinct approver).
   */
  requireNewPrincipal(principalId: string): void {
    if (this.#approves.has(principalId)) {
      throw new Spec006Error(
        ErrorCode.Conflict,
        "A principal may approve a four-eyes action only once",
      );
    }
  }
}
