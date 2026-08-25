/**
 * EP-042 M3 release transport typed errors (SPEC-006 codes).
 *
 * Transport failures are typed so the gate can assert fail-closed
 * behavior: an unreachable provider, a timeout, a cancelled request,
 * a digest mismatch, or a config error are all distinct outcomes with
 * redacted messages. Secret-shaped values never appear in messages.
 */

export const TRANSPORT_ERROR_CODES = [
  "CONFIG_MISSING",
  "CONFIG_INVALID",
  "UNREACHABLE",
  "TIMEOUT",
  "CANCELLED",
  "HTTP_ERROR",
  "MALFORMED_RESPONSE",
  "DIGEST_MISMATCH",
  "MISSING_OBJECT",
  "BUCKET_ERROR",
  "AUTH_DENIED",
  "INTERNAL",
] as const;

export type TransportErrorCode = (typeof TRANSPORT_ERROR_CODES)[number];

export class ReleaseTransportError extends Error {
  readonly code: TransportErrorCode;
  readonly status?: number;
  readonly requestId?: string;
  constructor(
    code: TransportErrorCode,
    message: string,
    opts?: { status?: number; requestId?: string; cause?: unknown },
  ) {
    super(message);
    this.name = "ReleaseTransportError";
    this.code = code;
    if (opts?.status !== undefined) this.status = opts.status;
    if (opts?.requestId !== undefined) this.requestId = opts.requestId;
    if (opts?.cause !== undefined) {
      (this as { cause?: unknown }).cause = opts.cause;
    }
  }
}

export function isTransportError(
  value: unknown,
): value is ReleaseTransportError {
  return value instanceof ReleaseTransportError;
}
