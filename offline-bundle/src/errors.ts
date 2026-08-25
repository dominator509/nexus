/**
 * EP-042 M5 typed failure classification for offline bundle behavior
 * (SPEC-016 behavior 5, SPEC-024; ExecPlan M5, fence N).
 *
 * Every denial path carries a typed class from this vocabulary. A bundle
 * failure is never reduced to a generic exit code: verification and
 * install denials are classified so operators can distinguish a missing
 * file from a digest mismatch from a path escape from a wrong release.
 */

export const BUNDLE_FAILURE_CLASSES = [
  "BUNDLE_INVALID",
  "BUNDLE_MISSING_FILE",
  "BUNDLE_DIGEST_MISMATCH",
  "BUNDLE_MALFORMED_DIGEST",
  "BUNDLE_DUPLICATE_PATH",
  "BUNDLE_SELF_DIGEST_MISMATCH",
  "BUNDLE_REQUIRED_KIND_MISSING",
  "BUNDLE_NOT_VERIFIED",
  "PATH_ESCAPE",
  "WRONG_RELEASE_ID",
  "MANIFEST_INVALID",
  "EVIDENCE_INVALID",
  "INSTALL_FAILED",
  "ROLLBACK_FAILED",
] as const;

export type BundleFailureClass = (typeof BUNDLE_FAILURE_CLASSES)[number];

export function isBundleFailureClass(
  value: string,
): value is BundleFailureClass {
  return (BUNDLE_FAILURE_CLASSES as readonly string[]).includes(value);
}

export interface BundleErrorShape {
  code: BundleFailureClass;
  message: string;
  field?: string;
  context?: Record<string, string>;
}

export class BundleError extends Error {
  readonly code: BundleFailureClass;
  readonly field?: string;
  readonly context?: Record<string, string>;

  constructor(
    code: BundleFailureClass,
    message: string,
    context?: Record<string, string>,
    field?: string,
  ) {
    super(message);
    this.name = "BundleError";
    this.code = code;
    if (context !== undefined) this.context = context;
    if (field !== undefined) this.field = field;
  }

  toShape(): BundleErrorShape {
    const shape: BundleErrorShape = {
      code: this.code,
      message: this.message,
    };
    if (this.field !== undefined) shape.field = this.field;
    if (this.context !== undefined) shape.context = this.context;
    return shape;
  }
}

export function isBundleError(value: unknown): value is BundleError {
  return value instanceof BundleError;
}
