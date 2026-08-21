/**
 * EP-035 M1 IntegrationCard truthfulness tests.
 *
 * Credential-exists never becomes HEALTHY; endpoint-entered never
 * becomes CONNECTED; component-installed never becomes WORKING.
 * Advertised capabilities are never derived from the provider name.
 */

import { describe, expect, it } from "vitest";
import {
  IntegrationCardData,
  IntegrationCardRequest,
  isValidIntegrationStatusTransition,
} from "../contracts/integration";
import { ErrorCode, Spec006Error } from "../contracts/errors";

const CORRELATION = "00000000-0000-4000-8000-000000000001";
const INTEGRATION_ID = "00000000-0000-4000-8000-000000000002";

describe("ep035_unit_integration", () => {
  it("an unconfigured card is not healthy and carries no verification", () => {
    const card = IntegrationCardData.parse({
      integration_id: INTEGRATION_ID,
      provider_name: "Home Assistant",
      status: "UNCONFIGURED",
      advertised_capabilities: [],
      correlation_id: CORRELATION,
    });
    expect(card.status).toBe("UNCONFIGURED");
    expect(card.configured_at_unix_s).toBeUndefined();
    expect(card.last_verified_at_unix_s).toBeUndefined();
  });

  it("capabilities are never derived from the provider name", () => {
    const card = IntegrationCardData.parse({
      integration_id: INTEGRATION_ID,
      provider_name: "Home Assistant",
      status: "CONFIGURED",
      advertised_capabilities: [],
      configured_at_unix_s: 1000,
      correlation_id: CORRELATION,
    });
    expect(card.advertised_capabilities).toEqual([]);
    // No lights/locks/HVAC/cameras appear from the name alone.
    expect(card.advertised_capabilities).not.toContain("lights");
    expect(card.advertised_capabilities).not.toContain("cameras");
  });

  it("CONFIGURED requires a configured timestamp", () => {
    expect(() =>
      IntegrationCardData.parse({
        integration_id: INTEGRATION_ID,
        provider_name: "Home Assistant",
        status: "CONFIGURED",
        advertised_capabilities: [],
        correlation_id: CORRELATION,
      }),
    ).toThrowError(Spec006Error);
  });

  it("REACHABLE and HEALTHY require a verification event", () => {
    expect(() =>
      IntegrationCardData.parse({
        integration_id: INTEGRATION_ID,
        provider_name: "Home Assistant",
        status: "HEALTHY",
        advertised_capabilities: [],
        configured_at_unix_s: 1000,
        correlation_id: CORRELATION,
      }),
    ).toThrowError(Spec006Error);
    const healthy = IntegrationCardData.parse({
      integration_id: INTEGRATION_ID,
      provider_name: "Home Assistant",
      status: "HEALTHY",
      advertised_capabilities: [],
      configured_at_unix_s: 1000,
      last_verified_at_unix_s: 2000,
      correlation_id: CORRELATION,
    });
    expect(healthy.status).toBe("HEALTHY");
  });

  it("credential existence alone cannot mint HEALTHY", () => {
    // A card that only "has a credential" (CONFIGURED) cannot jump to
    // HEALTHY: the transition table rejects the leap.
    const configured = IntegrationCardData.parse({
      integration_id: INTEGRATION_ID,
      provider_name: "Home Assistant",
      status: "CONFIGURED",
      advertised_capabilities: [],
      configured_at_unix_s: 1000,
      correlation_id: CORRELATION,
    });
    expect(() => configured.transition("HEALTHY", 1001)).toThrowError(
      Spec006Error,
    );
  });

  it("the status ladder is strictly ordered", () => {
    expect(
      isValidIntegrationStatusTransition("UNCONFIGURED", "CONFIGURED"),
    ).toBe(true);
    expect(isValidIntegrationStatusTransition("UNCONFIGURED", "HEALTHY")).toBe(
      false,
    );
    expect(
      isValidIntegrationStatusTransition("AUTHENTICATED", "REACHABLE"),
    ).toBe(true);
    expect(isValidIntegrationStatusTransition("REACHABLE", "HEALTHY")).toBe(
      true,
    );
    expect(isValidIntegrationStatusTransition("HEALTHY", "REACHABLE")).toBe(
      false,
    );
  });

  it("round-trips serialization and rejects unknown fields", () => {
    const card = IntegrationCardData.parse({
      integration_id: INTEGRATION_ID,
      provider_name: "Home Assistant",
      status: "DEGRADED",
      advertised_capabilities: [],
      configured_at_unix_s: 1000,
      last_verified_at_unix_s: 1500,
      correlation_id: CORRELATION,
    });
    const parsed = IntegrationCardData.parse(JSON.parse(JSON.stringify(card)));
    expect(parsed.status).toBe("DEGRADED");
    expect(() =>
      IntegrationCardData.parse({
        ...JSON.parse(JSON.stringify(card)),
        forged: true,
      }),
    ).toThrowError(Spec006Error);
  });

  it("parses card requests with deny-unknown", () => {
    const request = IntegrationCardRequest.parse({
      integration_id: INTEGRATION_ID,
      provider_name: "Home Assistant",
      correlation_id: CORRELATION,
    });
    expect(request.provider_name).toBe("Home Assistant");
    expect(() =>
      IntegrationCardRequest.parse({
        integration_id: INTEGRATION_ID,
        provider_name: "Home Assistant",
        correlation_id: CORRELATION,
        forged: true,
      }),
    ).toThrowError(Spec006Error);
  });
});
