/**
 * EP-035 M1 DeploymentChoice intent-only tests.
 *
 * Selection is intent: it never proves host, container runtime, ports,
 * DNS, TLS, running Nexus, or health. Verification is a separate
 * explicit state reached only through evidence.
 */

import { describe, expect, it } from "vitest";
import {
  DeploymentIntentRecord,
  DeploymentProfile,
  DeploymentSelectionRequest,
  DeploymentVerificationRequest,
} from "../contracts/deployment";
import { ErrorCode, Spec006Error } from "../contracts/errors";

const CORRELATION = "00000000-0000-4000-8000-000000000001";

function sampleProfile(mode = "FULLY_LOCAL"): DeploymentProfile {
  return DeploymentProfile.parse({
    id: "profile-local",
    mode,
    release_channel: "STABLE",
    components: ["core", "edge"],
    nodes: [{ id: "home" }],
    backup: { enabled: true },
    remote_access: { enabled: false },
  });
}

describe("ep035_unit_deployment", () => {
  it("parses the canonical profile with deny-unknown", () => {
    const profile = sampleProfile();
    expect(profile.mode).toBe("FULLY_LOCAL");
    expect(profile.release_channel).toBe("STABLE");
    expect(() =>
      DeploymentProfile.parse({ ...profile.toJSON(), forged: true }),
    ).toThrowError(Spec006Error);
    expect(() =>
      DeploymentProfile.parse({
        ...profile.toJSON(),
        mode: "MADE_UP",
      }),
    ).toThrowError(Spec006Error);
  });

  it("rejects a profile missing required fields", () => {
    const profile = sampleProfile().toJSON();
    const { id: _id, ...missingId } = profile;
    expect(() => DeploymentProfile.parse(missingId)).toThrowError(Spec006Error);
  });

  it("selection creates intent with verification UNVERIFIED, always", () => {
    const record = DeploymentIntentRecord.select(
      sampleProfile(),
      CORRELATION,
      1000,
    );
    expect(record.verification.state).toBe("UNVERIFIED");
    expect(record.verification.evidence).toBeUndefined();
  });

  it("VERIFIED requires evidence; intent alone never verifies", () => {
    const record = DeploymentIntentRecord.select(
      sampleProfile(),
      CORRELATION,
      1000,
    );
    try {
      record.withVerification("VERIFIED", 1001);
      throw new Error("expected rejection");
    } catch (err) {
      expect((err as Spec006Error).code).toBe(ErrorCode.Verification);
    }
    const verified = record.withVerification("VERIFIED", 1001, {
      verified_at_unix_s: 1001,
      evidence_id: "ev-1",
      verifier: "probe",
    });
    expect(verified.verification.state).toBe("VERIFIED");
    expect(verified.verification.evidence?.evidence_id).toBe("ev-1");
  });

  it("selection never claims host/runtime/DNS/TLS/health facts", () => {
    const record = DeploymentIntentRecord.select(
      sampleProfile("HYBRID"),
      CORRELATION,
      1000,
    );
    const wire = JSON.parse(JSON.stringify(record));
    expect(wire.verification.state).toBe("UNVERIFIED");
    // The intent record carries no runtime/health claim fields at all.
    expect(Object.keys(wire)).not.toContain("healthy");
    expect(Object.keys(wire)).not.toContain("reachable");
    expect(Object.keys(wire)).not.toContain("running");
  });

  it("VERIFYING and FAILED are explicit distinct states", () => {
    const record = DeploymentIntentRecord.select(
      sampleProfile(),
      CORRELATION,
      1000,
    );
    expect(record.withVerification("VERIFYING", 1001).verification.state).toBe(
      "VERIFYING",
    );
    expect(record.withVerification("FAILED", 1001).verification.state).toBe(
      "FAILED",
    );
  });

  it("round-trips intent serialization", () => {
    const record = DeploymentIntentRecord.select(
      sampleProfile("MANAGED"),
      CORRELATION,
      1000,
    );
    const parsed = DeploymentIntentRecord.parse(
      JSON.parse(JSON.stringify(record)),
    );
    expect(parsed.profile.mode).toBe("MANAGED");
    expect(parsed.verification.state).toBe("UNVERIFIED");
  });

  it("parses selection and verification requests with deny-unknown", () => {
    const selection = DeploymentSelectionRequest.parse({
      profile: sampleProfile().toJSON(),
      correlation_id: CORRELATION,
    });
    expect(selection.profile.mode).toBe("FULLY_LOCAL");
    expect(() =>
      DeploymentSelectionRequest.parse({
        profile: sampleProfile().toJSON(),
        correlation_id: CORRELATION,
        forged: true,
      }),
    ).toThrowError(Spec006Error);
    const verification = DeploymentVerificationRequest.parse({
      correlation_id: CORRELATION,
      state: "VERIFIED",
      evidence: {
        verified_at_unix_s: 1001,
        evidence_id: "ev-1",
        verifier: "probe",
      },
    });
    expect(verification.state).toBe("VERIFIED");
    expect(() =>
      DeploymentVerificationRequest.parse({
        correlation_id: CORRELATION,
        state: "VERIFIED",
      }),
    ).toThrowError(Spec006Error);
  });
});
