/**
 * EP-035 M1 interface surface test.
 *
 * The public barrel must expose all eight Setup Wizard and Onboarding
 * interfaces and their canonical vocabulary. This is the anti-masking
 * surface proof: a partial barrel or a collapsed generic service cannot
 * satisfy the milestone.
 */

import { describe, expect, it } from "vitest";
import * as setup from "../index";

describe("ep035_unit_surfaces", () => {
  // The eight interface PORTS (SetupWizardPort, DeploymentChoicePort,
  // HardwareProfilerPort, OwnerBootstrapPort, EdgeEnrollmentPort,
  // DiscoveryWizardPort, IntegrationCardPort, RecoveryFlowPort) are
  // TypeScript interfaces: type-level only, verified by the tsc --noEmit
  // gate through the barrel exports. Runtime surface proofs below assert
  // the value objects and canonical vocabulary every interface carries.

  it("exposes the setup state value objects", () => {
    expect(setup.SetupWizardState).toBeDefined();
    expect(setup.WizardStepRecord).toBeDefined();
    expect(setup.DeploymentProfile).toBeDefined();
    expect(setup.DeploymentIntentRecord).toBeDefined();
    expect(setup.HardwareProfile).toBeDefined();
    expect(setup.HardwareFact).toBeDefined();
    expect(setup.OwnerBootstrapRequest).toBeDefined();
    expect(setup.OwnerBootstrapStateRecord).toBeDefined();
    expect(setup.EnrollmentCredential).toBeDefined();
    expect(setup.EdgeEnrollmentRequest).toBeDefined();
    expect(setup.DiscoveryObservation).toBeDefined();
    expect(setup.DiscoveryReport).toBeDefined();
    expect(setup.IntegrationSelection).toBeDefined();
    expect(setup.IntegrationCardData).toBeDefined();
    expect(setup.RecoveryKit).toBeDefined();
    expect(setup.RecoveryDecision).toBeDefined();
    expect(setup.RecoveryEvidence).toBeDefined();
  });

  it("exposes the canonical vocabulary enums", () => {
    expect(setup.WIZARD_STATES).toContain("COMPLETED");
    expect(setup.DEPLOYMENT_MODES).toContain("FULLY_LOCAL");
    expect(setup.RELEASE_CHANNELS).toContain("STABLE");
    expect(setup.HARDWARE_PROVENANCES).toContain("HARDWARE_CERTIFIED");
    expect(setup.OWNER_BOOTSTRAP_STATES).toContain("OWNER_AUTHORIZED");
    expect(setup.ENROLLMENT_TRUST_STATES).toContain("AUTHORIZED");
    expect(setup.INTEGRATION_STATUSES).toContain("HEALTHY");
    expect(setup.RECOVERY_OUTCOMES).toContain("MANUAL_INTERVENTION");
    expect(setup.RECOVERY_MATERIAL_KINDS).toContain("OFFLINE_PASSPHRASE");
  });

  it("exposes the SPEC-006 error vocabulary", () => {
    expect(setup.ErrorCode.Policy).toBe("POLICY");
    expect(setup.Spec006Error).toBeDefined();
    expect(setup.classifyError).toBeDefined();
  });
});
