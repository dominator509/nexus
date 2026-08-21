/**
 * EP-035 M1 SPEC-006 error vocabulary tests.
 */

import { describe, expect, it } from "vitest";
import { ErrorCode, Spec006Error, classifyError } from "../contracts/errors";

describe("ep035_unit_errors", () => {
  it("exposes the canonical SPEC-006 codes without collapsing classes", () => {
    expect(ErrorCode.Validation).toBe("VALIDATION");
    expect(ErrorCode.Authorization).toBe("AUTHORIZATION");
    expect(ErrorCode.Policy).toBe("POLICY");
    expect(ErrorCode.Conflict).toBe("CONFLICT");
    expect(ErrorCode.Unavailable).toBe("UNAVAILABLE");
    expect(ErrorCode.Timeout).toBe("TIMEOUT");
    expect(ErrorCode.External).toBe("EXTERNAL");
    expect(ErrorCode.Verification).toBe("VERIFICATION");
    expect(ErrorCode.Vocabulary).toBe("VOCABULARY");
  });

  it("maps stable HTTP statuses per class", () => {
    expect(
      new Spec006Error(ErrorCode.Policy, "denied").toProblemDetails().status,
    ).toBe(403);
    expect(
      new Spec006Error(ErrorCode.Conflict, "already").toProblemDetails().status,
    ).toBe(409);
    expect(
      new Spec006Error(ErrorCode.Validation, "bad").toProblemDetails().status,
    ).toBe(400);
    expect(
      new Spec006Error(ErrorCode.Vocabulary, "unknown").toProblemDetails()
        .status,
    ).toBe(422);
  });

  it("classifies unknown thrown values as internal, never fabricated classes", () => {
    const classified = classifyError(new Error("boom"), "corr-1");
    expect(classified.code).toBe(ErrorCode.Internal);
    expect(classified.correlationId).toBe("corr-1");
  });

  it("keeps problem details structured and never invents content", () => {
    const err = new Spec006Error(
      ErrorCode.Authorization,
      "credential refused",
      "corr-9",
    );
    const details = err.toProblemDetails();
    expect(details.code).toBe(ErrorCode.Authorization);
    expect(details.detail).toBe("credential refused");
    expect(details.correlationId).toBe("corr-9");
    expect(details.type).toContain("authorization");
    // The class never adds fields beyond what was provided.
    expect(Object.keys(details).sort()).toEqual(
      ["code", "correlationId", "detail", "status", "type"].sort(),
    );
  });
});
