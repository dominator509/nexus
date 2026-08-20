import { describe, expect, it } from "vitest";
import { RedactedLogger, RedactedLogEntry, redact } from "../contracts/logging";

describe("ep033_unit_logging_redaction", () => {
  it("records only safe diagnostic fields", () => {
    const logger = new RedactedLogger();
    const entry = logger.log({
      route: "/approvals",
      view: "approval-center",
      correlation_id: "corr-0001",
      error_class: "POLICY",
      backend_status: "403",
      duration_ms: 12,
    });
    expect(entry.error_class).toBe("POLICY");
    expect(entry.backend_status).toBe("403");
  });

  it("redacts bearer tokens from free text", () => {
    expect(redact("Authorization: Bearer abcdefghijklmnopqrstuvwxyz")).toContain("[REDACTED]");
  });

  it("redacts token/secret/password-shaped assignments", () => {
    expect(redact("token=abcdefghijklmnopqrstuvwxyz")).toContain("[REDACTED]");
    expect(redact("password=hunter2secret")).toContain("[REDACTED]");
    expect(redact("api_key=sk-abcdefghijklmnopqrstuvwxyz")).toContain("[REDACTED]");
  });

  it("redacts private keys and approval credentials", () => {
    expect(redact("private_key=-----BEGIN RSA PRIVATE KEY-----")).toContain("[REDACTED]");
    expect(redact("approval_credential=cred-abcdefghijklmnopqrstuvwxyz")).toContain("[REDACTED]");
  });

  it("redacts JWTs in free text", () => {
    const jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIn0.signaturepart123456";
    expect(redact(`token ${jwt}`)).not.toContain(jwt);
    expect(redact(`token ${jwt}`)).toContain("[REDACTED]");
  });

  it("canary: no secret-shaped content survives any logged entry", () => {
    const logger = new RedactedLogger();
    logger.log({
      route: "/approvals",
      view: "approval-center",
      correlation_id: "corr-0001",
      error_class: "AUTHORIZATION",
      backend_status: "401",
      duration_ms: 3,
    });
    // Attempt to smuggle secrets through every free-text field; the
    // boundary must redact before recording.
    logger.log({
      route: "Authorization: Bearer abcdefghijklmnopqrstuvwxyz012345",
      view: "token=abcdef1234567890",
      correlation_id: "corr-0002",
      error_class: "password=hunter2secret",
      backend_status: "api_key=sk-1234567890abcdef",
      duration_ms: 4,
    });
    logger.assertNoSecrets();
    const serialized = logger.entries().map((entry) => entry.serialize()).join("\n");
    expect(serialized).not.toMatch(/eyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}/);
    expect(serialized).not.toMatch(/password=/);
    expect(serialized).not.toMatch(/token=/);
  });

  it("redaction happens at construction by design", () => {
    const entry = RedactedLogEntry.fromShape({
      route: "Authorization: Bearer abcdefghijklmnopqrstuvwxyz012345",
      view: "approvals",
      correlation_id: "corr-0001",
      error_class: "POLICY",
      backend_status: "403",
      duration_ms: 1,
    });
    expect(entry.route).toContain("[REDACTED]");
  });
});
