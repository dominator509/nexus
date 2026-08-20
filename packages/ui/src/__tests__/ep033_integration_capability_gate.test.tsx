/**
 * EP-033 M3 integration: CapabilityButton through REAL React rendering.
 *
 * react-dom/server renderToString executes the real React component
 * tree and produces real HTML markup - the transport boundary between
 * the shared UI package and the DOM surface. These tests prove
 * directive D/E semantics in rendered output, not in memory.
 */

import { describe, expect, it } from "vitest";
import { renderToString } from "react-dom/server";
import { PresentedCapability } from "@nexus/web";
import { CapabilityButton } from "../components/capability-button";

function capability(overrides: Record<string, unknown> = {}): PresentedCapability {
  return PresentedCapability.fromWire({
    capability_id: "home.lights.set",
    class: "COMMAND",
    availability: "AVAILABLE",
    visible: true,
    authorized: true,
    required_approval: "NONE",
    ...overrides,
  });
}

describe("ep033_integration_capability_gate", () => {
  it("renders a known, visible, authorized capability as an enabled button", () => {
    const html = renderToString(
      <CapabilityButton capability={capability()} label="Set lights" />,
    );
    expect(html).toContain('data-capability="home.lights.set"');
    expect(html).toContain("Set lights");
    expect(html).not.toContain("disabled");
  });

  it("renders a visible-but-unauthorized capability disabled (VISIBLE != AUTHORIZED)", () => {
    const html = renderToString(
      <CapabilityButton
        capability={capability({ authorized: false })}
        label="Set lights"
        disabledReason="Missing scope"
      />,
    );
    expect(html).toContain('disabled=""');
    expect(html).toContain('aria-disabled="true"');
    expect(html).toContain("Missing scope");
  });

  it("renders NOTHING for an invisible capability (fail closed)", () => {
    const html = renderToString(
      <CapabilityButton capability={capability({ visible: false })} label="Hidden" />,
    );
    expect(html).toBe("");
  });

  it("renders NOTHING for an unavailable capability (never presented as live)", () => {
    const html = renderToString(
      <CapabilityButton
        capability={capability({ availability: "UNCERTIFIED" })}
        label="Uncertified"
      />,
    );
    expect(html).toBe("");
  });

  it("exposes the label as the accessible name", () => {
    const html = renderToString(
      <CapabilityButton capability={capability()} label="Set lights" />,
    );
    expect(html).toContain('aria-label="Set lights"');
  });
});
