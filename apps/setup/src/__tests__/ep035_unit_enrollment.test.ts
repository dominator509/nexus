/**
 * EP-035 M1 EdgeEnrollment trust-layer and secret-redaction tests.
 *
 * DISCOVERED != ENROLLMENT_REQUESTED != IDENTITY_VERIFIED != ENROLLED
 * != TRUSTED != AUTHORIZED. Discovery metadata never advances trust by
 * itself. Enrollment credentials (BootstrapToken) are secrets: they
 * never appear in JSON, strings, or summaries, and used or expired
 * credentials are never valid again.
 */

import { describe, expect, it } from "vitest";
import {
  EdgeEnrollmentRequest,
  EnrollmentCredential,
  requiredTrustEvidence,
} from "../contracts/enrollment";
import { ErrorCode, Spec006Error } from "../contracts/errors";

const CREDENTIAL_ID = "00000000-0000-4000-8000-000000000001";
const SECRET_CANARY = "bootstrap-secret-canary-9f8e7d6c";

function sampleCredential(
  overrides: Record<string, unknown> = {},
): EnrollmentCredential {
  return EnrollmentCredential.parse({
    credential_id: CREDENTIAL_ID,
    kind: "BOOTSTRAP_TOKEN",
    issued_at_unix_s: 1000,
    expires_at_unix_s: 2000,
    state: "ISSUED",
    nonce: "nonce-canary",
    secret: SECRET_CANARY,
    ...overrides,
  });
}

describe("ep035_unit_enrollment", () => {
  it("the trust ladder is strictly ordered", () => {
    expect(
      requiredTrustEvidence("DISCOVERED", "ENROLLMENT_REQUESTED", undefined),
    ).toBeUndefined();
    // Identity verification requires evidence.
    try {
      requiredTrustEvidence(
        "ENROLLMENT_REQUESTED",
        "IDENTITY_VERIFIED",
        undefined,
      );
      throw new Error("expected rejection");
    } catch (err) {
      expect((err as Spec006Error).code).toBe(ErrorCode.Verification);
    }
    // A leap from DISCOVERED to AUTHORIZED is rejected outright.
    try {
      requiredTrustEvidence("DISCOVERED", "AUTHORIZED", "any-evidence");
      throw new Error("expected rejection");
    } catch (err) {
      expect((err as Spec006Error).code).toBe(ErrorCode.Policy);
    }
  });

  it("discovery metadata alone is never sufficient for trust", () => {
    // A record with only hostname/IP/QR/label cannot move beyond
    // ENROLLMENT_REQUESTED; every further step needs evidence.
    expect(() =>
      requiredTrustEvidence("DISCOVERED", "TRUSTED", "hostname=home-edge"),
    ).toThrowError(Spec006Error);
    expect(() =>
      requiredTrustEvidence("DISCOVERED", "ENROLLED", "ip=10.0.0.5"),
    ).toThrowError(Spec006Error);
  });

  it("credential secrets never appear in serialization", () => {
    const credential = sampleCredential();
    const json = JSON.stringify(credential);
    expect(json).not.toContain(SECRET_CANARY);
    expect(json).not.toContain("nonce-canary");
    expect(String(credential)).not.toContain(SECRET_CANARY);
    const redacted = credential.redacted();
    expect(Object.keys(redacted)).not.toContain("secret");
    expect(JSON.stringify(redacted)).not.toContain(SECRET_CANARY);
  });

  it("an ISSUED credential within its window is usable", () => {
    expect(sampleCredential().isUsable(1500)).toBe(true);
  });

  it("expired credentials are never usable", () => {
    const expired = sampleCredential({ expires_at_unix_s: 1499 });
    expect(expired.isUsable(1500)).toBe(false);
  });

  it("used and revoked credentials are never valid again, even if cached", () => {
    const used = sampleCredential({ state: "USED" });
    const revoked = sampleCredential({ state: "REVOKED" });
    const expired = sampleCredential({ state: "EXPIRED" });
    expect(used.isUsable(1500)).toBe(false);
    expect(revoked.isUsable(1500)).toBe(false);
    expect(expired.isUsable(1500)).toBe(false);
  });

  it("credential parse enforces deny-unknown and validity window", () => {
    expect(() =>
      EnrollmentCredential.parse({
        ...sampleCredential().toJSON(),
        forged: true,
      }),
    ).toThrowError(Spec006Error);
    expect(() =>
      EnrollmentCredential.parse({
        credential_id: CREDENTIAL_ID,
        kind: "BOOTSTRAP_TOKEN",
        issued_at_unix_s: 2000,
        expires_at_unix_s: 1000,
        state: "ISSUED",
        nonce: "n",
        secret: "s",
      }),
    ).toThrowError(Spec006Error);
    expect(() =>
      EnrollmentCredential.parse({
        credential_id: CREDENTIAL_ID,
        kind: "NOT_A_TOKEN",
        issued_at_unix_s: 1000,
        expires_at_unix_s: 2000,
        state: "ISSUED",
        nonce: "n",
        secret: "s",
      }),
    ).toThrowError(Spec006Error);
  });

  it("edge enrollment request is typed and deny-unknown", () => {
    const request = EdgeEnrollmentRequest.parse({
      device_label: "living-room-edge",
      endpoint: "https://edge.local",
      credential_id: CREDENTIAL_ID,
      correlation_id: "00000000-0000-4000-8000-000000000002",
    });
    expect(request.device_label).toBe("living-room-edge");
    expect(() =>
      EdgeEnrollmentRequest.parse({
        device_label: "x",
        endpoint: "https://edge.local",
        credential_id: CREDENTIAL_ID,
        correlation_id: "00000000-0000-4000-8000-000000000002",
        forged: true,
      }),
    ).toThrowError(Spec006Error);
  });
});
