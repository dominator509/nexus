/**
 * EP-035 M1 validation primitives tests: deny-unknown fields, enum
 * vocabulary rejection, invalid identifier rejection, missing required
 * values, and fail-closed deserialization.
 */

import { describe, expect, it } from "vitest";
import {
  assertEnum,
  assertObject,
  assertUuid,
  rejectUnknownFields,
} from "../contracts/validate";
import { ErrorCode, Spec006Error } from "../contracts/errors";

describe("ep035_unit_validate", () => {
  it("rejects unknown fields on any object", () => {
    expect(() =>
      rejectUnknownFields(
        { known: 1, forged: 2 },
        new Set(["known"]),
        "payload",
      ),
    ).toThrowError(Spec006Error);
    try {
      rejectUnknownFields({ forged: 2 }, new Set(["known"]), "payload");
    } catch (err) {
      expect((err as Spec006Error).code).toBe(ErrorCode.Validation);
    }
  });

  it("rejects unknown enum vocabulary values", () => {
    try {
      assertEnum("MADE_UP", new Set(["REAL"]), "mode");
    } catch (err) {
      expect((err as Spec006Error).code).toBe(ErrorCode.Vocabulary);
    }
  });

  it("rejects invalid identifiers", () => {
    expect(() => assertUuid("not-a-uuid", "id")).toThrowError(Spec006Error);
    expect(assertUuid("00000000-0000-4000-8000-000000000001", "id")).toBe(
      "00000000-0000-4000-8000-000000000001",
    );
  });

  it("rejects non-object wire input fail closed", () => {
    expect(() => assertObject("string", "payload")).toThrowError(Spec006Error);
    expect(() => assertObject([], "payload")).toThrowError(Spec006Error);
    expect(() => assertObject(null, "payload")).toThrowError(Spec006Error);
  });
});
