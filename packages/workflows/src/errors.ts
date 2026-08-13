/**
 * Canonical workflow error contracts (SPEC-006).
 *
 * Every boundary failure carries a stable machine code from the SPEC-006
 * ladder, preserves correlation, and distinguishes validation,
 * authentication, authorization, policy, unavailable, timeout, conflict,
 * rate limit, external provider, verification, compensation, and internal
 * invariant failures. These codes are vocabulary locked; a new code
 * requires an ADR and schema update.
 */

export const NexusErrorCode = {
  VALIDATION: "VALIDATION",
  AUTHENTICATION: "AUTHENTICATION",
  AUTHORIZATION: "AUTHORIZATION",
  POLICY: "POLICY",
  UNAVAILABLE: "UNAVAILABLE",
  TIMEOUT: "TIMEOUT",
  CONFLICT: "CONFLICT",
  RATE_LIMIT: "RATE_LIMIT",
  EXTERNAL_PROVIDER: "EXTERNAL_PROVIDER",
  VERIFICATION: "VERIFICATION",
  COMPENSATION: "COMPENSATION",
  INTERNAL_INVARIANT: "INTERNAL_INVARIANT",
} as const;

export type NexusErrorCode =
  (typeof NexusErrorCode)[keyof typeof NexusErrorCode];

export const NEXUS_ERROR_CODES: readonly NexusErrorCode[] =
  Object.values(NexusErrorCode);

export interface WorkflowErrorOptions {
  readonly correlationId?: string;
  readonly workflowId?: string;
  readonly activityId?: string;
  readonly cause?: unknown;
}

/**
 * Typed workflow failure carrying a SPEC-006 code. Retryability is a
 * property of the code plus the owning retry policy, never of the message.
 */
export class NexusWorkflowError extends Error {
  readonly code: NexusErrorCode;
  readonly correlationId: string | undefined;
  readonly workflowId: string | undefined;
  readonly activityId: string | undefined;
  readonly cause: unknown;

  constructor(
    code: NexusErrorCode,
    message: string,
    options: WorkflowErrorOptions = {},
  ) {
    super(message);
    this.name = "NexusWorkflowError";
    this.code = code;
    this.correlationId = options.correlationId;
    this.workflowId = options.workflowId;
    this.activityId = options.activityId;
    this.cause = options.cause;
  }

  isRetryable(): boolean {
    switch (this.code) {
      case "UNAVAILABLE":
      case "TIMEOUT":
      case "RATE_LIMIT":
      case "CONFLICT":
        return true;
      default:
        return false;
    }
  }

  toProblemDetails(): Record<string, unknown> {
    return {
      type: `urn:nexus:error:${this.code.toLowerCase()}`,
      title: this.name,
      status: this.code === "VALIDATION" ? 400 : 500,
      detail: this.message,
      code: this.code,
      correlation_id: this.correlationId ?? null,
      workflow_id: this.workflowId ?? null,
      activity_id: this.activityId ?? null,
    };
  }
}

/**
 * Contract-level validation failure (vocabulary rejection, malformed
 * signal, invalid policy). Always VALIDATION and never retryable.
 */
export class WorkflowContractError extends NexusWorkflowError {
  constructor(message: string, options: WorkflowErrorOptions = {}) {
    super("VALIDATION", message, options);
    this.name = "WorkflowContractError";
  }
}

/** Construct a typed workflow error with a SPEC-006 code. */
export function workflowError(
  code: NexusErrorCode,
  message: string,
  options: WorkflowErrorOptions = {},
): NexusWorkflowError {
  return new NexusWorkflowError(code, message, options);
}
