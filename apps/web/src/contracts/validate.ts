/**
 * EP-033 M1 contract validation primitives.
 *
 * Every public contract constructor validates wire-shaped input with
 * deny-unknown-field semantics (mirroring the canonical schema
 * `additionalProperties: false` rule and the Rust serde
 * deny_unknown_fields pattern used by prior nodes). Validation is the
 * only entry point: a raw object can never become a typed contract
 * without passing through these checks, so the UI can never fabricate
 * vocabulary or authority from unvalidated input.
 */

import { Spec006Error, ErrorCode } from "./errors";

export function assertObject(value: unknown, what: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Spec006Error(ErrorCode.Validation, `${what} must be an object`);
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
      throw new Spec006Error(ErrorCode.Validation, `${what} has unknown field '${key}'`);
    }
  }
}

export function assertString(value: unknown, what: string): string {
  if (typeof value !== "string") {
    throw new Spec006Error(ErrorCode.Validation, `${what} must be a string`);
  }
  return value;
}

export function assertUuid(value: unknown, what: string): string {
  const s = assertString(value, what);
  if (!UUID_RE.test(s)) {
    throw new Spec006Error(ErrorCode.Validation, `${what} must be a UUID`);
  }
  return s;
}

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

export function assertEnum<T extends string>(
  value: unknown,
  allowed: ReadonlySet<T>,
  what: string,
): T {
  const s = assertString(value, what);
  if (!(allowed as ReadonlySet<string>).has(s)) {
    throw new Spec006Error(
      ErrorCode.Vocabulary,
      `${what} has unsupported value '${s}'`,
    );
  }
  return s as T;
}

export function assertInt(value: unknown, what: string): number {
  if (typeof value !== "number" || !Number.isInteger(value)) {
    throw new Spec006Error(ErrorCode.Validation, `${what} must be an integer`);
  }
  return value;
}

export function assertBool(value: unknown, what: string): boolean {
  if (typeof value !== "boolean") {
    throw new Spec006Error(ErrorCode.Validation, `${what} must be a boolean`);
  }
  return value;
}

export function assertStringSet(
  value: unknown,
  what: string,
  min: number,
  max: number,
): ReadonlySet<string> {
  if (!Array.isArray(value)) {
    throw new Spec006Error(ErrorCode.Validation, `${what} must be an array`);
  }
  if (value.length < min || value.length > max) {
    throw new Spec006Error(
      ErrorCode.Validation,
      `${what} must contain between ${min} and ${max} entries`,
    );
  }
  const out = new Set<string>();
  for (const entry of value) {
    const s = assertString(entry, `${what} entry`);
    if (s.length === 0) {
      throw new Spec006Error(ErrorCode.Validation, `${what} entry must not be empty`);
    }
    if (out.has(s)) {
      throw new Spec006Error(ErrorCode.Validation, `${what} contains duplicate '${s}'`);
    }
    out.add(s);
  }
  return out;
}
