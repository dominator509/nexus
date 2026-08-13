import { describe, expect, it } from "vitest";

import {
  isUuidV7,
  parseWorkflowId,
  parseSignalId,
  parseActionDigest,
  isActionDigest,
} from "../ids.js";
import { WID_A, DIGEST_A, DIGEST_B } from "./helpers/fixtures.js";

describe("ep006_unit_ids", () => {
  it("ep006_unit_ids_uuidv7_accepts_canonical", () => {
    expect(isUuidV7(WID_A)).toBe(true);
    expect(parseWorkflowId(WID_A)).toBe(WID_A);
    expect(parseSignalId(WID_A)).toBe(WID_A);
  });

  it("ep006_unit_ids_uuidv7_rejects_wrong_version_nibble", () => {
    const bad = WID_A.replace("-7000-", "-6000-");
    expect(isUuidV7(bad)).toBe(false);
    expect(() => parseWorkflowId(bad)).toThrow(/UUIDv7/);
  });

  it("ep006_unit_ids_uuidv7_rejects_wrong_variant", () => {
    const bad = WID_A.replace("-8000-", "-0000-");
    expect(isUuidV7(bad)).toBe(false);
    expect(() => parseWorkflowId(bad)).toThrow(/UUIDv7/);
  });

  it("ep006_unit_ids_uuidv7_rejects_uppercase", () => {
    const bad = WID_A.toUpperCase();
    expect(isUuidV7(bad)).toBe(false);
  });

  it("ep006_unit_ids_uuidv7_rejects_malformed", () => {
    expect(isUuidV7("not-a-uuid")).toBe(false);
    expect(isUuidV7(WID_A.replaceAll("-", ""))).toBe(false);
    expect(isUuidV7(42)).toBe(false);
    expect(isUuidV7(null)).toBe(false);
    expect(() =>
      parseWorkflowId("0193a1f2-0000-7000-8000-00000000000z"),
    ).toThrow(/UUIDv7/);
  });

  it("ep006_unit_ids_action_digest_accepts_64_lowercase_hex", () => {
    expect(isActionDigest(DIGEST_A)).toBe(true);
    expect(isActionDigest(DIGEST_B)).toBe(true);
    expect(parseActionDigest(DIGEST_A)).toBe(DIGEST_A);
  });

  it("ep006_unit_ids_action_digest_rejects_short_and_uppercase", () => {
    expect(isActionDigest("abc")).toBe(false);
    expect(isActionDigest(DIGEST_A.toUpperCase())).toBe(false);
    expect(isActionDigest("")).toBe(false);
    expect(() => parseActionDigest("abc")).toThrow(/sha256/);
    expect(() => parseActionDigest(undefined)).toThrow(/sha256/);
  });
});
