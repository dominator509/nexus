/**
 * EP-035 M1 DiscoveryWizard observation-only tests.
 *
 * Discovery returns observations, not authority. Hostile discovery
 * content ("ADMIN", "TRUSTED", "AUTO-APPROVE", "OWNER DEVICE") is
 * inert data. Integration selection is an explicit governed step that
 * records the selecting principal.
 */

import { describe, expect, it } from "vitest";
import {
  DiscoveryObservation,
  DiscoveryReport,
  IntegrationSelection,
} from "../contracts/discovery";
import { ErrorCode, Spec006Error } from "../contracts/errors";

const CORRELATION = "00000000-0000-4000-8000-000000000001";
const PRINCIPAL = "00000000-0000-4000-8000-000000000002";

function hostileObservation(): Record<string, unknown> {
  return {
    id: "00000000-0000-4000-8000-000000000003",
    kind: "DEVICE",
    name: "ADMIN",
    endpoint: "mdns://trusted-device.local",
    advertised_capabilities: ["AUTO-APPROVE", "OWNER_DEVICE"],
    metadata: { vendor: "hostile" },
    observed_at_unix_s: 1000,
  };
}

describe("ep035_unit_discovery", () => {
  it("parses observations with deny-unknown", () => {
    const observation = DiscoveryObservation.parse(hostileObservation());
    expect(observation.name).toBe("ADMIN");
    expect(() =>
      DiscoveryObservation.parse({ ...hostileObservation(), forged: true }),
    ).toThrowError(Spec006Error);
  });

  it("hostile discovery content is detected as data, never authority", () => {
    const observation = DiscoveryObservation.parse(hostileObservation());
    expect(observation.containsHostileAuthorityToken()).toBe(true);
    // Detection is informational: parsing never changes any authority.
    const report = DiscoveryReport.parse({
      observations: [hostileObservation()],
      generated_at_unix_s: 1001,
      correlation_id: CORRELATION,
    });
    expect(report.observations.length).toBe(1);
    // The report itself carries no trust/enrollment/authorization state.
    const wire = JSON.parse(JSON.stringify(report));
    expect(Object.keys(wire)).not.toContain("authorized");
    expect(Object.keys(wire)).not.toContain("enrolled");
    expect(Object.keys(wire)).not.toContain("trusted");
  });

  it("benign observations do not trip the hostile detector", () => {
    const observation = DiscoveryObservation.parse({
      id: "00000000-0000-4000-8000-000000000004",
      kind: "SERVICE",
      name: "kitchen-speaker",
      endpoint: "http://10.0.0.9:8080",
      advertised_capabilities: ["audio"],
      metadata: {},
      observed_at_unix_s: 1000,
    });
    expect(observation.containsHostileAuthorityToken()).toBe(false);
  });

  it("selection is an explicit governed step recording the principal", () => {
    const selection = IntegrationSelection.parse({
      observation_id: "00000000-0000-4000-8000-000000000003",
      selected_by: PRINCIPAL,
      selected_at_unix_s: 1002,
      correlation_id: CORRELATION,
    });
    expect(selection.selected_by).toBe(PRINCIPAL);
    expect(() =>
      IntegrationSelection.parse({
        observation_id: "00000000-0000-4000-8000-000000000003",
        selected_by: PRINCIPAL,
        selected_at_unix_s: 1002,
        correlation_id: CORRELATION,
        forged: true,
      }),
    ).toThrowError(Spec006Error);
  });

  it("rejects unknown discovery kinds", () => {
    expect(() =>
      DiscoveryObservation.parse({ ...hostileObservation(), kind: "GOD_MODE" }),
    ).toThrowError(Spec006Error);
  });

  it("round-trips report serialization", () => {
    const report = DiscoveryReport.parse({
      observations: [hostileObservation()],
      generated_at_unix_s: 1001,
      correlation_id: CORRELATION,
    });
    const parsed = DiscoveryReport.parse(JSON.parse(JSON.stringify(report)));
    expect(parsed.observations[0]?.name).toBe("ADMIN");
  });
});
