/**
 * EP-043 M1 typed error surface (SPEC-006 codes, SPEC-008 ship domain).
 *
 * Fail-closed contract errors for the production readiness and ship
 * boundary. Every error carries a canonical SPEC-006 error code, a
 * machine-readable class, and a redacted message; serialization never
 * emits secret-shaped content.
 */

export const SHIP_ERROR_CODES = [
  "VALIDATION_FAILED",
  "NOT_FOUND",
  "CONFLICT",
  "POLICY_DENIED",
  "UNAVAILABLE",
  "TIMEOUT",
  "EXTERNAL_PROVIDER",
  "VERIFICATION_FAILED",
  "COMPENSATION_FAILED",
  "INTERNAL_INVARIANT",
] as const;

export type ShipErrorCode = (typeof SHIP_ERROR_CODES)[number];

export interface ShipErrorShape {
  code: ShipErrorCode;
  class: string;
  message: string;
  redacted: boolean;
}

const REDACTION_PATTERNS: RegExp[] = [
  /sk-[A-Za-z0-9._-]{6,}/g,
  /ghp_[A-Za-z0-9._-]{6,}/g,
  /AKIA[0-9A-Z._-]{6,}/g,
  /Bearer\s+[A-Za-z0-9._-]{16,}/gi,
  /xoxb-[0-9-]{10,}/g,
  /glpat-[A-Za-z0-9_-]{16,}/g,
  /token=([^\s&]+)/gi,
  /password=([^\s&]+)/gi,
  /secret=([^\s&]+)/gi,
  /-----BEGIN [A-Z ]*PRIVATE KEY-----/g,
];

/** Redact secret-shaped content from a message. Deterministic per input. */
export function redactShipMessage(message: string): string {
  let out = message;
  for (const pattern of REDACTION_PATTERNS) {
    out = out.replace(pattern, "[REDACTED]");
  }
  return out;
}

export class ShipError extends Error {
  readonly code: ShipErrorCode;
  readonly redacted: boolean;

  constructor(code: ShipErrorCode, message: string, redacted = true) {
    super(redacted ? redactShipMessage(message) : message);
    this.name = "ShipError";
    this.code = code;
    this.redacted = redacted;
  }

  toShape(): ShipErrorShape {
    return {
      code: this.code,
      class: this.constructor.name,
      message: this.message,
      redacted: this.redacted,
    };
  }

  toRedactedJson(): string {
    return JSON.stringify(this.toShape());
  }
}

export function isShipError(value: unknown): value is ShipError {
  return value instanceof ShipError;
}

export function assertKnownShipErrorCode(value: string): ShipErrorCode {
  if (!(SHIP_ERROR_CODES as readonly string[]).includes(value)) {
    throw new ShipError(
      "VALIDATION_FAILED",
      `unknown ship error code: ${value}`,
    );
  }
  return value as ShipErrorCode;
}
