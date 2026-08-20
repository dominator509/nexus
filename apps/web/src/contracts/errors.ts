/**
 * EP-033 M1 canonical error vocabulary (SPEC-006).
 *
 * Every UI-visible failure uses a stable machine code and safe human
 * explanation. The UI never collapses distinct failure classes into a
 * generic "Something went wrong": presentation may be friendly, state
 * must remain truthful.
 */

/**
 * Canonical SPEC-006 error codes used by the dashboard contracts.
 * The set is a strict subset of the repository canonical vocabulary
 * (crates/nexus-notifications NotificationErrorCode and SPEC-006
 * "Error states"): validation, authentication, authorization, policy,
 * unavailable, timeout, conflict, rate limit, external provider,
 * verification, compensation, internal invariant, plus the
 * vocabulary rejection code used by prior contract crates.
 */
export enum ErrorCode {
  Validation = "VALIDATION",
  Authentication = "AUTHENTICATION",
  Authorization = "AUTHORIZATION",
  Policy = "POLICY",
  NotFound = "NOT_FOUND",
  Conflict = "CONFLICT",
  Unavailable = "UNAVAILABLE",
  Timeout = "TIMEOUT",
  RateLimit = "RATE_LIMIT",
  External = "EXTERNAL",
  Verification = "VERIFICATION",
  Compensation = "COMPENSATION",
  Internal = "INTERNAL",
  Vocabulary = "VOCABULARY",
}

export interface ProblemDetails {
  /** Stable machine code, e.g. "POLICY". Never free-form prose. */
  code: ErrorCode;
  /** RFC 9457-compatible problem type URI fragment. */
  type: string;
  /** Safe human explanation. Never contains secrets or private content. */
  detail: string;
  /** Canonical correlation id when available. */
  correlationId?: string;
  /** HTTP status for the class when rendered over HTTP. */
  status: number;
}

const STATUS_BY_CODE: Readonly<Record<ErrorCode, number>> = {
  [ErrorCode.Validation]: 400,
  [ErrorCode.Authentication]: 401,
  [ErrorCode.Authorization]: 403,
  [ErrorCode.Policy]: 403,
  [ErrorCode.NotFound]: 404,
  [ErrorCode.Conflict]: 409,
  [ErrorCode.Unavailable]: 503,
  [ErrorCode.Timeout]: 504,
  [ErrorCode.RateLimit]: 429,
  [ErrorCode.External]: 502,
  [ErrorCode.Verification]: 409,
  [ErrorCode.Compensation]: 500,
  [ErrorCode.Internal]: 500,
  [ErrorCode.Vocabulary]: 422,
};

export class Spec006Error extends Error {
  readonly code: ErrorCode;
  readonly correlationId: string | undefined;

  constructor(code: ErrorCode, detail: string, correlationId?: string) {
    super(detail);
    this.name = "Spec006Error";
    this.code = code;
    this.correlationId = correlationId;
  }

  toProblemDetails(): ProblemDetails {
    return {
      code: this.code,
      type: `https://schemas.nexus.local/problems/${this.code.toLowerCase()}`,
      detail: this.message,
      ...(this.correlationId === undefined
        ? {}
        : { correlationId: this.correlationId }),
      status: STATUS_BY_CODE[this.code],
    };
  }
}

/** Categorize an unknown thrown value into a stable SPEC-006 class. */
export function classifyError(value: unknown, correlationId?: string): Spec006Error {
  if (value instanceof Spec006Error) {
    return value;
  }
  if (value instanceof Error) {
    return new Spec006Error(ErrorCode.Internal, "Unexpected internal failure", correlationId);
  }
  return new Spec006Error(ErrorCode.Internal, "Unexpected non-error failure", correlationId);
}
