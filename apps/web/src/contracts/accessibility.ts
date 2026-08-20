/**
 * EP-033 M1 accessibility contracts (directive Q, SPEC-004 behavior 7).
 *
 * Accessibility requirements are executable contracts, not prose:
 * every interactive surface declares a label, semantic role, keyboard
 * handlers, focus order, reduced-motion handling, and non-color status
 * signaling. This milestone proves the CONTRACTS exist and validate;
 * WCAG 2.2 AA conformance scanning is owned by later milestones
 * (Playwright/axe in EP-033 M3/M5) and is NOT claimed here.
 */

import {
  assertBool,
  assertEnum,
  assertObject,
  assertString,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const A11Y_ROLES = [
  "button",
  "link",
  "navigation",
  "main",
  "region",
  "dialog",
  "tablist",
  "tab",
  "menu",
  "menuitem",
  "heading",
  "textbox",
  "checkbox",
  "switch",
  "list",
  "listitem",
  "alert",
  "status",
] as const;
export type A11yRole = (typeof A11Y_ROLES)[number];

const A11Y_SURFACE_FIELDS = new Set<string>([
  "name",
  "label",
  "role",
  "focusable",
  "keyboard_operable",
  "focus_order",
  "reduced_motion_safe",
  "non_color_status",
]);

export interface A11ySurfaceShape {
  name: string;
  label: string;
  role: A11yRole;
  focusable: boolean;
  keyboard_operable: boolean;
  focus_order: number;
  reduced_motion_safe: boolean;
  non_color_status: boolean;
}

/**
 * An interactive surface's accessibility contract. Constructing a
 * surface without a label, role, or keyboard operability fails closed:
 * the surface cannot be rendered as interactive.
 */
export class A11ySurface {
  readonly name: string;
  readonly label: string;
  readonly role: A11yRole;
  readonly focusable: boolean;
  readonly keyboard_operable: boolean;
  readonly focus_order: number;
  readonly reduced_motion_safe: boolean;
  readonly non_color_status: boolean;

  private constructor(shape: A11ySurfaceShape) {
    this.name = shape.name;
    this.label = shape.label;
    this.role = shape.role;
    this.focusable = shape.focusable;
    this.keyboard_operable = shape.keyboard_operable;
    this.focus_order = shape.focus_order;
    this.reduced_motion_safe = shape.reduced_motion_safe;
    this.non_color_status = shape.non_color_status;
  }

  static fromWire(value: unknown): A11ySurface {
    const obj = assertObject(value, "A11ySurface");
    rejectUnknownFields(obj, A11Y_SURFACE_FIELDS, "A11ySurface");
    const name = assertString(obj.name, "name");
    const label = assertString(obj.label, "label");
    if (name.length === 0 || label.length === 0) {
      throw new Spec006Error(
        ErrorCode.Validation,
        "a11y surface requires a name and label",
      );
    }
    return new A11ySurface({
      name,
      label,
      role: assertEnum(obj.role, new Set<A11yRole>(A11Y_ROLES), "role"),
      focusable: assertBool(obj.focusable, "focusable"),
      keyboard_operable: assertBool(obj.keyboard_operable, "keyboard_operable"),
      focus_order: typeof obj.focus_order === "number" ? obj.focus_order : 0,
      reduced_motion_safe: assertBool(
        obj.reduced_motion_safe,
        "reduced_motion_safe",
      ),
      non_color_status: assertBool(obj.non_color_status, "non_color_status"),
    });
  }

  /** Interactive surfaces must be keyboard operable and labeled. */
  assertInteractive(): void {
    if (!this.keyboard_operable) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `Surface '${this.name}' is not keyboard operable`,
      );
    }
  }
}

/**
 * Focus order contract: focus_order must be a positive integer and
 * unique within a view. Duplicate or zero orders fail closed.
 */
export class FocusOrder {
  readonly #orders: Map<string, number> = new Map();

  register(name: string, order: number): void {
    if (!Number.isInteger(order) || order <= 0) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `Focus order for '${name}' must be a positive integer`,
      );
    }
    for (const existing of this.#orders.values()) {
      if (existing === order) {
        throw new Spec006Error(
          ErrorCode.Conflict,
          `Duplicate focus order ${order}`,
        );
      }
    }
    this.#orders.set(name, order);
  }

  orderOf(name: string): number | undefined {
    return this.#orders.get(name);
  }
}

/**
 * Reduced-motion contract: a motion surface is safe only when it
 * declares reduced_motion_safe=true or has an explicit reduced-motion
 * variant. Rendering motion without the flag is a contract violation.
 */
export function assertReducedMotionSafe(surface: A11ySurface): void {
  if (!surface.reduced_motion_safe) {
    throw new Spec006Error(
      ErrorCode.Validation,
      `Surface '${surface.name}' is not reduced-motion safe`,
    );
  }
}

/**
 * Non-color status: any state conveyed by color alone violates the
 * contract. A status with no non-color signaling fails closed.
 */
export function assertNonColorStatus(surface: A11ySurface): void {
  if (!surface.non_color_status) {
    throw new Spec006Error(
      ErrorCode.Validation,
      `Surface '${surface.name}' conveys state by color alone`,
    );
  }
}
