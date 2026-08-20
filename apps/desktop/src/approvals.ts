/**
 * EP-033 M2 desktop approval flow (deterministic core).
 *
 * Presents approval cards and tracks approval state transitions with
 * the canonical classes preserved. FOUR_EYES progression uses the
 * shared FourEyesRecord: two distinct principals are required and a
 * single account can never satisfy both approvals (directive M).
 */

import {
  ApprovalAction,
  ApprovalCard,
  FourEyesRecord,
  ErrorCode,
  Spec006Error,
  type ApprovalState,
} from "@nexus/web";

export interface ApprovalProgression {
  card: ApprovalCard;
  state: ApprovalState;
  distinctApprovers: ReadonlyArray<string>;
  satisfied: boolean;
}

/**
 * Deterministic approval flow for one card. The flow records actions;
 * it never mints authority - APPROVED state is derived only from the
 * canonical approval semantics of the card's class.
 */
export class DesktopApprovalFlow {
  readonly card: ApprovalCard;
  #state: ApprovalState = "PENDING";
  readonly #fourEyes: FourEyesRecord;
  readonly #requesterExcluded: boolean;

  constructor(card: ApprovalCard, requesterExcluded = true) {
    this.card = card;
    this.#fourEyes = new FourEyesRecord(card.approval_id);
    this.#requesterExcluded = requesterExcluded;
  }

  get state(): ApprovalState {
    return this.#state;
  }

  /**
   * Record an approval action. For FOUR_EYES, the flow requires two
   * distinct principals (a duplicate principal is a Conflict, never a
   * second approval).
   */
  apply(action: ApprovalAction, nowUnixS: number): ApprovalState {
    if (this.#state === "EXPIRED") {
      throw new Spec006Error(ErrorCode.Conflict, "Approval already expired");
    }
    if (
      this.#state === "APPROVED" ||
      this.#state === "DENIED" ||
      this.#state === "REVOKED"
    ) {
      throw new Spec006Error(
        ErrorCode.Conflict,
        `Approval already ${this.#state.toLowerCase()}`,
      );
    }
    if (this.card.isExpired(nowUnixS)) {
      this.#state = "EXPIRED";
      throw new Spec006Error(
        ErrorCode.Conflict,
        "Approval expired before action",
      );
    }
    if (action.approval_id !== this.card.approval_id) {
      throw new Spec006Error(
        ErrorCode.Conflict,
        "Approval action targets a different card",
      );
    }

    if (this.card.requiresTwoPrincipals()) {
      if (action.action === "APPROVE") {
        if (
          this.#requesterExcluded &&
          action.principal_id === this.card.requester_id
        ) {
          // Requester approval is recorded for visibility but can
          // never satisfy four-eyes; state stays PENDING.
          this.#fourEyes.apply(action);
          this.#state = "PENDING";
          return this.#state;
        }
        // Guard BEFORE recording: a duplicate principal is a
        // Conflict, never a second distinct approver.
        this.#fourEyes.requireNewPrincipal(action.principal_id);
        this.#fourEyes.apply(action);
        if (this.#fourEyes.isSatisfied(this.card.requester_id)) {
          this.#state = "APPROVED";
        }
      } else {
        this.#state = "DENIED";
      }
      return this.#state;
    }

    // Non-four-eyes classes: a single APPROVE satisfies the class.
    if (action.action === "APPROVE") {
      if (
        this.#requesterExcluded &&
        action.principal_id === this.card.requester_id
      ) {
        throw new Spec006Error(
          ErrorCode.Policy,
          "Requester cannot approve their own action",
          this.card.correlation,
        );
      }
      this.#state = "APPROVED";
    } else {
      this.#state = "DENIED";
    }
    return this.#state;
  }

  expire(): void {
    if (this.#state === "PENDING") {
      this.#state = "EXPIRED";
    }
  }

  revoke(): void {
    if (this.#state === "APPROVED" || this.#state === "PENDING") {
      this.#state = "REVOKED";
    }
  }

  progression(): ApprovalProgression {
    return {
      card: this.card,
      state: this.#state,
      distinctApprovers: this.#fourEyes.distinctApprovers(),
      satisfied: this.#fourEyes.isSatisfied(this.card.requester_id),
    };
  }
}
