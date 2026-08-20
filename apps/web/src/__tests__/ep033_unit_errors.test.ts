import { describe, expect, it } from "vitest";
import { ErrorCode, Spec006Error, classifyError } from "../contracts/errors";

describe("ep033_unit_errors_vocabulary", () => {
  it("exposes the full canonical SPEC-006 error vocabulary", () => {
    expect([...Object.values(ErrorCode)]).toEqual([
      "VALIDATION",
      "AUTHENTICATION",
      "AUTHORIZATION",
      "POLICY",
      "NOT_FOUND",
      "CONFLICT",
      "UNAVAILABLE",
      "TIMEOUT",
      "RATE_LIMIT",
      "EXTERNAL",
      "VERIFICATION",
      "COMPENSATION",
      "INTERNAL",
      "VOCABULARY",
    ]);
  });

  it("distinguishes failure classes instead of collapsing to a generic message", () => {
    const authorization = new Spec006Error(ErrorCode.Authorization, "denied");
    const policy = new Spec006Error(ErrorCode.Policy, "policy rejected");
    const unavailable = new Spec006Error(ErrorCode.Unavailable, "backend down");
    const timeout = new Spec006Error(ErrorCode.Timeout, "timed out");
    const conflict = new Spec006Error(ErrorCode.Conflict, "duplicate");
    const verification = new Spec006Error(ErrorCode.Verification, "mismatch");

    expect(authorization.code).not.toBe(policy.code);
    expect(authorization.code).not.toBe(unavailable.code);
    expect(timeout.code).not.toBe(unavailable.code);
    expect(conflict.code).not.toBe(verification.code);
  });

  it("produces RFC 9457-compatible problem details with stable codes", () => {
    const error = new Spec006Error(ErrorCode.Policy, "policy rejected", "corr-0001");
    const details = error.toProblemDetails();
    expect(details.code).toBe(ErrorCode.Policy);
    expect(details.type).toBe("https://schemas.nexus.local/problems/policy");
    expect(details.correlationId).toBe("corr-0001");
    expect(details.status).toBe(403);
  });

  it("maps each error class to a stable HTTP status", () => {
    expect(new Spec006Error(ErrorCode.Authentication, "x").toProblemDetails().status).toBe(401);
    expect(new Spec006Error(ErrorCode.Authorization, "x").toProblemDetails().status).toBe(403);
    expect(new Spec006Error(ErrorCode.NotFound, "x").toProblemDetails().status).toBe(404);
    expect(new Spec006Error(ErrorCode.Unavailable, "x").toProblemDetails().status).toBe(503);
    expect(new Spec006Error(ErrorCode.Timeout, "x").toProblemDetails().status).toBe(504);
    expect(new Spec006Error(ErrorCode.RateLimit, "x").toProblemDetails().status).toBe(429);
  });

  it("classifies unknown thrown values into the internal class, never success", () => {
    const classified = classifyError(new Error("boom"), "corr-0002");
    expect(classified.code).toBe(ErrorCode.Internal);
    expect(classified.correlationId).toBe("corr-0002");
  });

  it("preserves already-classified errors", () => {
    const original = new Spec006Error(ErrorCode.External, "provider bad");
    expect(classifyError(original)).toBe(original);
  });

  it("never embeds secrets in problem details", () => {
    const error = new Spec006Error(
      ErrorCode.Authorization,
      "token invalid",
    );
    const details = error.toProblemDetails();
    expect(details.detail).not.toMatch(/eyJ[a-zA-Z0-9_-]{10,}/);
  });
});
