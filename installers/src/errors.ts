/**
 * EP-042 M4 installer typed failure classification (SPEC-006 codes;
 * SPEC-016 / SPEC-024 error states; fence N vocabulary).
 *
 * Every installer failure carries a typed class so the gate can assert
 * fail-closed behavior: a manifest error, a digest mismatch, a denied
 * policy decision, a backup failure, a staging failure, an interrupted
 * update, a rollback failure, a timeout, an unavailable dependency, an
 * authorization denial, a resource exhaustion, a path escape, a foreign
 * resource request, or a recovery failure are all distinct outcomes.
 * Messages never contain secrets, tokens, private payloads, or signing
 * key material.
 */

export const INSTALLER_FAILURE_CLASSES = [
  "MANIFEST_INVALID",
  "SIGNATURE_UNVERIFIED",
  "DIGEST_MISMATCH",
  "COMPATIBILITY_DENIED",
  "BACKUP_FAILED",
  "STAGING_FAILED",
  "INSTALL_FAILED",
  "VALIDATION_FAILED",
  "ROLLBACK_REQUIRED",
  "ROLLBACK_FAILED",
  "TIMEOUT",
  "UNAVAILABLE",
  "RESOURCE_EXHAUSTION",
  "AUTHORIZATION_DENIED",
  "PATH_ESCAPE",
  "FOREIGN_RESOURCE",
  "RECOVERY_FAILED",
] as const;

export type InstallerFailureClass = (typeof INSTALLER_FAILURE_CLASSES)[number];

const INSTALLER_FAILURE_CLASS_SET: ReadonlySet<string> = new Set(
  INSTALLER_FAILURE_CLASSES,
);

export function isInstallerFailureClass(
  value: unknown,
): value is InstallerFailureClass {
  return typeof value === "string" && INSTALLER_FAILURE_CLASS_SET.has(value);
}

export interface InstallerErrorShape {
  failure_class: InstallerFailureClass;
  message: string;
  install_id?: string;
  release_id?: string;
  component_id?: string;
  correlation_id?: string;
}

/**
 * Typed installer error carrying failure class, correlation, and
 * redaction-safe detail. Never constructed with secret-shaped content.
 */
export class InstallerError extends Error {
  readonly failureClass: InstallerFailureClass;
  readonly installId?: string;
  readonly releaseId?: string;
  readonly componentId?: string;
  readonly correlationId?: string;

  constructor(
    failureClass: InstallerFailureClass,
    message: string,
    options: {
      installId?: string;
      releaseId?: string;
      componentId?: string;
      correlationId?: string;
    } = {},
  ) {
    super(`installer ${failureClass}: ${message}`);
    this.name = "InstallerError";
    this.failureClass = failureClass;
    if (options.installId !== undefined) this.installId = options.installId;
    if (options.releaseId !== undefined) this.releaseId = options.releaseId;
    if (options.componentId !== undefined) {
      this.componentId = options.componentId;
    }
    if (options.correlationId !== undefined) {
      this.correlationId = options.correlationId;
    }
  }

  toShape(): InstallerErrorShape {
    const shape: InstallerErrorShape = {
      failure_class: this.failureClass,
      message: this.message,
    };
    if (this.installId !== undefined) shape.install_id = this.installId;
    if (this.releaseId !== undefined) shape.release_id = this.releaseId;
    if (this.componentId !== undefined) {
      shape.component_id = this.componentId;
    }
    if (this.correlationId !== undefined) {
      shape.correlation_id = this.correlationId;
    }
    return shape;
  }
}

export function isInstallerError(value: unknown): value is InstallerError {
  return value instanceof InstallerError;
}
