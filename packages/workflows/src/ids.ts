/**
 * Typed workflow identifiers (SPEC-001/SPEC-022; ADR-010).
 *
 * All identifiers are opaque UUIDv7 values represented as lowercase
 * canonical strings (8-4-4-4-12 hex layout, version nibble `7`, variant
 * nibble `8/9/a/b`). Each kind is a distinct branded type and is not
 * interchangeable at compile time.
 */

import { WorkflowContractError } from "./errors.js";

declare const __workflowId: unique symbol;
declare const __workflowRunId: unique symbol;
declare const __activityId: unique symbol;
declare const __signalId: unique symbol;
declare const __actionId: unique symbol;
declare const __objectiveId: unique symbol;
declare const __receiptId: unique symbol;

export type WorkflowId = string & { readonly [__workflowId]: true };
export type WorkflowRunId = string & { readonly [__workflowRunId]: true };
export type ActivityId = string & { readonly [__activityId]: true };
export type SignalId = string & { readonly [__signalId]: true };
export type ActionId = string & { readonly [__actionId]: true };
export type ObjectiveId = string & { readonly [__objectiveId]: true };
export type ReceiptId = string & { readonly [__receiptId]: true };

/**
 * Canonical SHA-256 hex digest of the exact action payload being approved.
 * 64 lowercase hex characters. The approval binds to this digest, never to
 * free text.
 */
export type ActionDigest = string & {
  readonly __actionDigest: unique symbol;
};

const UUIDV7_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const ACTION_DIGEST_RE = /^[0-9a-f]{64}$/;

export function isUuidV7(value: unknown): value is string {
  return typeof value === "string" && UUIDV7_RE.test(value);
}

export function parseUuidV7(value: unknown, context = "identifier"): string {
  if (typeof value !== "string" || !UUIDV7_RE.test(value)) {
    throw new WorkflowContractError(
      `${context} must be a canonical lowercase UUIDv7 string, got ${JSON.stringify(value)}`,
    );
  }
  return value;
}

export function isActionDigest(value: unknown): value is ActionDigest {
  return typeof value === "string" && ACTION_DIGEST_RE.test(value);
}

export function parseActionDigest(value: unknown): ActionDigest {
  if (typeof value !== "string" || !ACTION_DIGEST_RE.test(value)) {
    throw new WorkflowContractError(
      `actionDigest must be a 64-char lowercase sha256 hex string, got ${JSON.stringify(value)}`,
    );
  }
  return value as ActionDigest;
}

export function parseWorkflowId(value: unknown): WorkflowId {
  return parseUuidV7(value, "workflowId") as WorkflowId;
}

export function parseWorkflowRunId(value: unknown): WorkflowRunId {
  return parseUuidV7(value, "workflowRunId") as WorkflowRunId;
}

export function parseActivityId(value: unknown): ActivityId {
  return parseUuidV7(value, "activityId") as ActivityId;
}

export function parseSignalId(value: unknown): SignalId {
  return parseUuidV7(value, "signalId") as SignalId;
}

export function parseActionId(value: unknown): ActionId {
  return parseUuidV7(value, "actionId") as ActionId;
}

export function parseObjectiveId(value: unknown): ObjectiveId {
  return parseUuidV7(value, "objectiveId") as ObjectiveId;
}

export function parseReceiptId(value: unknown): ReceiptId {
  return parseUuidV7(value, "receiptId") as ReceiptId;
}
