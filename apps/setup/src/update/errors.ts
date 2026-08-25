/**
 * EP-042 M2 release/update error surface (SPEC-006 codes; SPEC-016 /
 * SPEC-024 error states).
 *
 * Mirrors the canonical M1 error vocabulary in crates/nexus-release
 * (error.rs) at the TypeScript boundary. Every failure distinguishes
 * validation, authentication, authorization, policy, unavailable,
 * timeout, conflict, rate limit, external provider, verification,
 * compensation, internal invariant, and the release-specific
 * signature-invalid, digest-mismatch, incompatible, backup-required,
 * unsafe-rollback, promotion-not-authorized, and channel-mismatch
 * failures. Messages never contain secrets, tokens, private payloads,
 * or signature key material.
 */

/** Canonical release error codes (SPEC-006; SPEC-016/024 error states). */
export const ReleaseErrorCode = {
  Validation: "VALIDATION",
  Authentication: "AUTHENTICATION",
  Authorization: "AUTHORIZATION",
  Policy: "POLICY",
  Unavailable: "UNAVAILABLE",
  Timeout: "TIMEOUT",
  Conflict: "CONFLICT",
  NotFound: "NOT_FOUND",
  RateLimit: "RATE_LIMIT",
  ExternalProvider: "EXTERNAL_PROVIDER",
  Verification: "VERIFICATION",
  Compensation: "COMPENSATION",
  Vocabulary: "VOCABULARY",
  SignatureInvalid: "SIGNATURE_INVALID",
  DigestMismatch: "DIGEST_MISMATCH",
  Incompatible: "INCOMPATIBLE",
  BackupRequired: "BACKUP_REQUIRED",
  UnsafeRollback: "UNSAFE_ROLLBACK",
  PromotionNotAuthorized: "PROMOTION_NOT_AUTHORIZED",
  ChannelMismatch: "CHANNEL_MISMATCH",
  InternalInvariant: "INTERNAL_INVARIANT",
} as const;

export type ReleaseErrorCode =
  (typeof ReleaseErrorCode)[keyof typeof ReleaseErrorCode];

export const RELEASE_ERROR_CODES: ReadonlyArray<string> =
  Object.values(ReleaseErrorCode);

const RELEASE_ERROR_CODE_SET: ReadonlySet<string> = new Set(
  RELEASE_ERROR_CODES,
);

export function isReleaseErrorCode(value: unknown): value is ReleaseErrorCode {
  return typeof value === "string" && RELEASE_ERROR_CODE_SET.has(value);
}

export interface ReleaseErrorShape {
  code: ReleaseErrorCode;
  message: string;
  correlationId?: string;
  field?: string;
}

/**
 * Canonical release error carrying correlation and redaction-safe detail.
 * Never constructed with secret-shaped content.
 */
export class ReleaseError extends Error {
  readonly code: ReleaseErrorCode;
  readonly correlationId?: string;
  readonly field?: string;

  constructor(
    code: ReleaseErrorCode,
    message: string,
    options: { correlationId?: string; field?: string } = {},
  ) {
    super(`release ${code}: ${message}`);
    this.name = "ReleaseError";
    this.code = code;
    if (options.correlationId !== undefined) {
      this.correlationId = options.correlationId;
    }
    if (options.field !== undefined) {
      this.field = options.field;
    }
  }

  toShape(): ReleaseErrorShape {
    const shape: ReleaseErrorShape = {
      code: this.code,
      message: this.message,
    };
    if (this.correlationId !== undefined) {
      shape.correlationId = this.correlationId;
    }
    if (this.field !== undefined) {
      shape.field = this.field;
    }
    return shape;
  }
}
