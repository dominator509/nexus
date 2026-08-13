/**
 * Durable workflow queries (ADR-010; SPEC-023 behavior 5).
 *
 * Queries are read-only, deterministic views into workflow state. They
 * never mutate state and never perform I/O; answers are derived from the
 * durable event history so a replay answers identically.
 */

import { parseActionId, parseActivityId, parseWorkflowId } from "./ids.js";
import type { ActionId, ActivityId, WorkflowId } from "./ids.js";
import { WorkflowContractError } from "./errors.js";
import { queryType, workflowState } from "./vocabulary.js";
import type { WorkflowOutcome, WorkflowState } from "./vocabulary.js";
import { validateApprovalSignal } from "./signals.js";
import type { ApprovalSignal } from "./signals.js";

export type WorkflowQuery =
  | { readonly queryType: "WORKFLOW_STATUS"; readonly workflowId: WorkflowId }
  | {
      readonly queryType: "PENDING_APPROVAL";
      readonly workflowId: WorkflowId;
    }
  | {
      readonly queryType: "ACTIVITY_STATE";
      readonly workflowId: WorkflowId;
      readonly activityId: ActivityId;
    }
  | {
      readonly queryType: "ACTION_RECEIPT";
      readonly workflowId: WorkflowId;
      readonly actionId: ActionId;
    };

export function validateQuery(value: unknown): WorkflowQuery {
  if (typeof value !== "object" || value === null) {
    throw new WorkflowContractError(
      `WorkflowQuery must be an object, got ${JSON.stringify(value)}`,
    );
  }
  const record = value as Record<string, unknown>;
  const kind = queryType.parse(record.queryType, "queryType");
  const workflowId = parseWorkflowId(record.workflowId);
  switch (kind) {
    case "WORKFLOW_STATUS":
      return { queryType: "WORKFLOW_STATUS", workflowId };
    case "PENDING_APPROVAL":
      return { queryType: "PENDING_APPROVAL", workflowId };
    case "ACTIVITY_STATE": {
      const activityId = parseActivityId(record.activityId);
      return { queryType: "ACTIVITY_STATE", workflowId, activityId };
    }
    case "ACTION_RECEIPT": {
      const actionId = parseActionId(record.actionId);
      return { queryType: "ACTION_RECEIPT", workflowId, actionId };
    }
  }
}

export interface WorkflowStatusQueryResponse {
  readonly queryType: "WORKFLOW_STATUS";
  readonly workflowId: WorkflowId;
  readonly state: WorkflowState;
  readonly outcome?: WorkflowOutcome;
  /** Engine-provided deterministic timestamp (ISO-8601). */
  readonly updatedAt: string;
}

export interface PendingApprovalQueryResponse {
  readonly queryType: "PENDING_APPROVAL";
  readonly workflowId: WorkflowId;
  /** Immutable approvals observed so far, in arrival order. */
  readonly approvals: readonly ApprovalSignal[];
}

export interface ActivityStateQueryResponse {
  readonly queryType: "ACTIVITY_STATE";
  readonly workflowId: WorkflowId;
  readonly activityId: ActivityId;
  readonly state: WorkflowState;
  readonly attempts: number;
}

export interface ActionReceiptQueryResponse {
  readonly queryType: "ACTION_RECEIPT";
  readonly workflowId: WorkflowId;
  readonly actionId: ActionId;
  readonly actionDigest: string;
  readonly decision: "APPROVE" | "REJECT" | "PENDING";
  readonly decidedAt?: string;
  readonly receiptId?: string;
  readonly verified: boolean;
}

export type WorkflowQueryResponse =
  | WorkflowStatusQueryResponse
  | PendingApprovalQueryResponse
  | ActivityStateQueryResponse
  | ActionReceiptQueryResponse;

export function validateQueryResponse(value: unknown): WorkflowQueryResponse {
  if (typeof value !== "object" || value === null) {
    throw new WorkflowContractError(
      `WorkflowQueryResponse must be an object, got ${JSON.stringify(value)}`,
    );
  }
  const record = value as Record<string, unknown>;
  const kind = queryType.parse(record.queryType, "queryType");
  const workflowId = parseWorkflowId(record.workflowId);
  switch (kind) {
    case "WORKFLOW_STATUS": {
      const state = workflowState.parse(record.state, "state");
      const outcome = record.outcome as WorkflowOutcome | undefined;
      return outcome === undefined
        ? {
            queryType: "WORKFLOW_STATUS",
            workflowId,
            state,
            updatedAt: String(record.updatedAt ?? ""),
          }
        : {
            queryType: "WORKFLOW_STATUS",
            workflowId,
            state,
            outcome,
            updatedAt: String(record.updatedAt ?? ""),
          };
    }
    case "PENDING_APPROVAL": {
      const approvals = Array.isArray(record.approvals) ? record.approvals : [];
      return {
        queryType: "PENDING_APPROVAL",
        workflowId,
        approvals: approvals.map((a) => {
          if (typeof a !== "object" || a === null) {
            throw new WorkflowContractError("approval entry must be an object");
          }
          return validateApprovalSignal(a);
        }),
      };
    }
    case "ACTIVITY_STATE": {
      const activityId = parseActivityId(record.activityId);
      return {
        queryType: "ACTIVITY_STATE",
        workflowId,
        activityId,
        state: workflowState.parse(record.state, "state"),
        attempts: Number(record.attempts ?? 0),
      };
    }
    case "ACTION_RECEIPT": {
      const actionId = parseActionId(record.actionId);
      const decidedAt = record.decidedAt as string | undefined;
      const receiptId = record.receiptId as string | undefined;
      const base: {
        queryType: "ACTION_RECEIPT";
        workflowId: WorkflowId;
        actionId: ActionId;
        actionDigest: string;
        decision: ActionReceiptQueryResponse["decision"];
        verified: boolean;
      } = {
        queryType: "ACTION_RECEIPT",
        workflowId,
        actionId,
        actionDigest: String(record.actionDigest ?? ""),
        decision: record.decision as ActionReceiptQueryResponse["decision"],
        verified: Boolean(record.verified),
      };
      return decidedAt === undefined && receiptId === undefined
        ? base
        : {
            ...base,
            ...(decidedAt === undefined ? {} : { decidedAt }),
            ...(receiptId === undefined ? {} : { receiptId }),
          };
    }
  }
}
