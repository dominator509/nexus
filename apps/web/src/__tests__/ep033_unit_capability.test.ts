import { describe, expect, it } from "vitest";
import {
  CapabilityPresentation,
  KnownCapabilityVocabulary,
  PresentedCapability,
} from "../contracts/capability";
import { ErrorCode, Spec006Error } from "../contracts/errors";
import type { CapabilityDescriptor } from "@nexus/contracts";

function descriptor(overrides: Partial<CapabilityDescriptor> = {}): CapabilityDescriptor {
  return {
    id: "home.lights.query",
    version: "1.0.0",
    class: "QUERY",
    description: "Query light state",
    input_schema: "schemas/home/lights-query.input.json",
    output_schema: "schemas/home/lights-query.output.json",
    required_scopes: [],
    risk: "R0",
    approval: "NONE",
    reversal: "NONE",
    idempotency: "NOT_APPLICABLE",
    availability: "AVAILABLE",
    ...overrides,
  };
}

describe("ep033_unit_capability_presentation", () => {
  it("renders known capabilities", () => {
    const vocabulary = new KnownCapabilityVocabulary(["home.lights.query"]);
    expect(vocabulary.resolve(descriptor())).toBe(CapabilityPresentation.RENDER);
  });

  it("fails closed on unknown capabilities (never fabricated)", () => {
    const vocabulary = new KnownCapabilityVocabulary(["home.lights.query"]);
    const unknown = descriptor({ id: "home.lights.hack" });
    expect(vocabulary.resolve(unknown)).toBe(CapabilityPresentation.UNSUPPORTED);
  });

  it("hides known but unavailable capabilities", () => {
    const vocabulary = new KnownCapabilityVocabulary(["home.lights.query"]);
    const unavailable = descriptor({ availability: "UNAVAILABLE" });
    expect(vocabulary.resolve(unavailable)).toBe(CapabilityPresentation.HIDDEN);
  });

  it("keeps visible distinct from authorized (VISIBLE != AUTHORIZED)", () => {
    const visibleNotAuthorized = PresentedCapability.fromWire({
      capability_id: "home.lights.query",
      class: "QUERY",
      availability: "AVAILABLE",
      visible: true,
      authorized: false,
      required_approval: "NONE",
    });
    expect(visibleNotAuthorized.visible).toBe(true);
    expect(visibleNotAuthorized.invocable).toBe(false);

    const visibleAndAuthorized = PresentedCapability.fromWire({
      capability_id: "home.lights.query",
      class: "QUERY",
      availability: "AVAILABLE",
      visible: true,
      authorized: true,
      required_approval: "NONE",
    });
    expect(visibleAndAuthorized.invocable).toBe(true);
  });

  it("never presents an unavailable capability as operational", () => {
    const uncertified = PresentedCapability.fromWire({
      capability_id: "home.lights.query",
      class: "QUERY",
      availability: "UNCERTIFIED",
      visible: true,
      authorized: true,
      required_approval: "NONE",
    });
    expect(uncertified.operational).toBe(false);
  });

  it("rejects unknown fields in presented capability wire input", () => {
    expect(() =>
      PresentedCapability.fromWire({
        capability_id: "home.lights.query",
        class: "QUERY",
        availability: "AVAILABLE",
        visible: true,
        authorized: true,
        required_approval: "NONE",
        minted_authority: true,
      }),
    ).toThrowError(Spec006Error);
  });

  it("rejects unsupported capability classes", () => {
    expect(() =>
      PresentedCapability.fromWire({
        capability_id: "home.lights.query",
        class: "GOD_MODE",
        availability: "AVAILABLE",
        visible: true,
        authorized: true,
        required_approval: "NONE",
      }),
    ).toThrowError(Spec006Error);
  });
});
