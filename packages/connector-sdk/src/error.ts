// SDK typed error (SPEC-006) for the TypeScript binding.
//
// Mirrors the Rust `SdkError`: canonical failure class plus
// correlation/actor/tenant/resource context. Failures fail closed: an
// error is never converted into a success.

/** Canonical SDK failure class (SPEC-006). */
export type SdkErrorCode =
  | "VALIDATION"
  | "AUTHENTICATION"
  | "AUTHORIZATION"
  | "POLICY"
  | "UNAVAILABLE"
  | "TIMEOUT"
  | "CONFLICT"
  | "NOT_FOUND"
  | "RATE_LIMIT"
  | "EXTERNAL_PROVIDER"
  | "VERIFICATION"
  | "COMPENSATION"
  | "INTERNAL";

/** Typed SDK failure with SPEC-006 context. Field names are the
 * canonical snake_case wire names shared by the Rust, TypeScript, and
 * Python bindings (directive D: no language-specific wire aliases). */
export interface SdkError {
  readonly code: SdkErrorCode;
  readonly message: string;
  readonly correlation_id?: string;
  readonly actor?: string;
  readonly tenant?: string;
  readonly resource?: string;
}

/** Construct a typed SDK error. */
export function sdkError(
  code: SdkErrorCode,
  message: string,
  context?: {
    correlation_id?: string;
    actor?: string;
    tenant?: string;
    resource?: string;
  },
): SdkError {
  return {
    code,
    message,
    ...(context?.correlation_id !== undefined
      ? { correlation_id: context.correlation_id }
      : {}),
    ...(context?.actor !== undefined ? { actor: context.actor } : {}),
    ...(context?.tenant !== undefined ? { tenant: context.tenant } : {}),
    ...(context?.resource !== undefined ? { resource: context.resource } : {}),
  };
}

/** True when the failure is retryable at the transport layer. */
export function isTransient(err: SdkError): boolean {
  return (
    err.code === "UNAVAILABLE" ||
    err.code === "TIMEOUT" ||
    err.code === "RATE_LIMIT" ||
    err.code === "EXTERNAL_PROVIDER"
  );
}
