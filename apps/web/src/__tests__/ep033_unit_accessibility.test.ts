import { describe, expect, it } from "vitest";
import {
  A11ySurface,
  FocusOrder,
  assertReducedMotionSafe,
  assertNonColorStatus,
  A11Y_ROLES,
} from "../contracts/accessibility";
import { ErrorCode, Spec006Error } from "../contracts/errors";

function surface(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    name: "approve-button",
    label: "Approve quarantine",
    role: "button",
    focusable: true,
    keyboard_operable: true,
    focus_order: 1,
    reduced_motion_safe: true,
    non_color_status: true,
    ...overrides,
  };
}

describe("ep033_unit_accessibility_contracts", () => {
  it("constructs an interactive surface with label, role, and keyboard operability", () => {
    const button = A11ySurface.fromWire(surface());
    expect(button.label).toBe("Approve quarantine");
    expect(button.role).toBe("button");
    expect(button.keyboard_operable).toBe(true);
    button.assertInteractive();
  });

  it("exposes the canonical semantic role vocabulary", () => {
    expect(A11Y_ROLES).toContain("button");
    expect(A11Y_ROLES).toContain("navigation");
    expect(A11Y_ROLES).toContain("dialog");
    expect(A11Y_ROLES).toContain("alert");
  });

  it("fails closed on surfaces without a label", () => {
    expect(() => A11ySurface.fromWire(surface({ label: "" }))).toThrowError(Spec006Error);
  });

  it("fails closed on interactive surfaces without keyboard operability", () => {
    const unkeyboardable = A11ySurface.fromWire(surface({ keyboard_operable: false }));
    expect(() => unkeyboardable.assertInteractive()).toThrowError(Spec006Error);
  });

  it("rejects unsupported roles", () => {
    expect(() => A11ySurface.fromWire(surface({ role: "magic-widget" }))).toThrowError(
      Spec006Error,
    );
  });

  it("enforces unique positive focus orders", () => {
    const focusOrder = new FocusOrder();
    focusOrder.register("a", 1);
    focusOrder.register("b", 2);
    expect(() => focusOrder.register("c", 1)).toThrowError(Spec006Error);
    expect(() => focusOrder.register("d", 0)).toThrowError(Spec006Error);
  });

  it("requires reduced-motion safety for motion surfaces", () => {
    const motionSurface = A11ySurface.fromWire(surface({ name: "carousel", reduced_motion_safe: false }));
    expect(() => assertReducedMotionSafe(motionSurface)).toThrowError(Spec006Error);
    const safe = A11ySurface.fromWire(surface());
    expect(() => assertReducedMotionSafe(safe)).not.toThrow();
  });

  it("requires non-color status signaling", () => {
    const colorOnly = A11ySurface.fromWire(surface({ name: "status-dot", non_color_status: false }));
    expect(() => assertNonColorStatus(colorOnly)).toThrowError(Spec006Error);
    const accessible = A11ySurface.fromWire(surface());
    expect(() => assertNonColorStatus(accessible)).not.toThrow();
  });

  it("rejects unknown fields", () => {
    expect(() => A11ySurface.fromWire(surface({ onclick: "evil()" }))).toThrowError(Spec006Error);
  });

  it("does not claim WCAG conformance (owned by later milestones)", () => {
    // The contract surface proves labels/roles/keyboard/focus/reduced
    // motion/non-color exist and validate. WCAG 2.2 AA scanning is an
    // M3/M5 Playwright+axe obligation, never claimed here.
    expect(A11Y_ROLES).not.toContain("wcag-conformant");
  });
});
