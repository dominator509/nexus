/**
 * EP-042 M2 validation primitives for the release/update boundary.
 *
 * Every public contract parse validates wire-shaped input with
 * deny-unknown-field semantics, mirroring the canonical Rust serde
 * deny_unknown_fields pattern in crates/nexus-release (M1). Validation
 * is the only entry point: a raw object can never become a typed
 * release/update contract without passing through these checks.
 *
 * The update core is pure: it never imports node builtins, filesystem,
 * network, or process modules (enforced by the M2 gate).
 */

import { ReleaseError, ReleaseErrorCode } from "./errors";

export function assertObject(
  value: unknown,
  what: string,
): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must be an object`,
      { field: what },
    );
  }
  return value as Record<string, unknown>;
}

export function rejectUnknownFields(
  value: Record<string, unknown>,
  allowed: ReadonlySet<string>,
  what: string,
): void {
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) {
      throw new ReleaseError(
        ReleaseErrorCode.Validation,
        `${what} has unknown field '${key}'`,
        { field: what },
      );
    }
  }
}

export function assertNonEmptyString(value: unknown, what: string): string {
  if (typeof value !== "string" || value.trim() === "") {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must be a non-empty string`,
      { field: what },
    );
  }
  return value;
}

export function assertString(value: unknown, what: string): string {
  if (typeof value !== "string") {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must be a string`,
      { field: what },
    );
  }
  return value;
}

export function assertStringArray(
  value: unknown,
  what: string,
  maxLength = 1024,
): ReadonlyArray<string> {
  if (!Array.isArray(value)) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must be an array`,
      { field: what },
    );
  }
  if (value.length > maxLength) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} exceeds maximum length ${maxLength}`,
      { field: what },
    );
  }
  return value.map((entry, index) =>
    assertNonEmptyString(entry, `${what}[${index}]`),
  );
}

export function assertNonNegativeInt(value: unknown, what: string): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value < 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must be a non-negative integer`,
      { field: what },
    );
  }
  return value;
}

export function assertU32(value: unknown, what: string): number {
  if (
    typeof value !== "number" ||
    !Number.isInteger(value) ||
    value < 0 ||
    value > 0xffff_ffff
  ) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must be a u32 integer`,
      { field: what },
    );
  }
  return value;
}

export function assertEnum<T extends string>(
  value: unknown,
  allowed: ReadonlySet<T>,
  what: string,
): T {
  if (typeof value !== "string" || !allowed.has(value as T)) {
    throw new ReleaseError(
      ReleaseErrorCode.Vocabulary,
      `unknown ${what} value`,
      { field: what },
    );
  }
  return value as T;
}

export function assertIso8601Timestamp(value: unknown, what: string): string {
  const s = assertNonEmptyString(value, what);
  // Light structural check: YYYY-MM-DD prefix (RFC3339 per repository
  // canonical event contracts).
  if (!/^\d{4}-\d{2}-\d{2}/.test(s)) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must be an ISO-8601/RFC3339 timestamp`,
      { field: what },
    );
  }
  return s;
}

export function assertOptionalString(
  value: unknown,
  what: string,
): string | undefined {
  if (value === undefined) {
    return undefined;
  }
  return assertNonEmptyString(value, what);
}

export function assertOptionalObject(
  value: unknown,
  what: string,
): Record<string, unknown> | undefined {
  if (value === undefined) {
    return undefined;
  }
  return assertObject(value, what);
}

export function assertOptionalStringArray(
  value: unknown,
  what: string,
): ReadonlyArray<string> | undefined {
  if (value === undefined) {
    return undefined;
  }
  return assertStringArray(value, what);
}

export function isHex(s: string): boolean {
  return s.length > 0 && /^[0-9a-fA-F]+$/.test(s);
}

export function isBase64(s: string): boolean {
  if (s.length === 0 || s.length % 4 !== 0) {
    return false;
  }
  const padding = s.length - s.replace(/=+$/, "").length;
  if (padding > 2) {
    return false;
  }
  const body = s.slice(0, s.length - padding);
  return /^[A-Za-z0-9+/]+$/.test(body);
}
