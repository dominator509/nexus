/**
 * EP-033 M1 capability presentation (directive D/E).
 *
 * The dashboard may DISPLAY capabilities (from canonical
 * CapabilityDescriptor), but rendering a button or panel never creates
 * authority. VISIBLE != AUTHORIZED: a capability can be rendered yet
 * not authorized, and no UI action can mint authority the backend did
 * not grant.
 *
 * Unknown capabilities fail closed: a capability id outside the known
 * vocabulary is HIDDEN or UNSUPPORTED, never a best-effort fabricated
 * panel. The UI never invents backend features.
 */

import {
  assertEnum,
  assertObject,
  assertString,
  rejectUnknownFields,
} from "./validate";
import type { CapabilityDescriptor } from "@nexus/contracts";

export const CAPABILITY_CLASSES = [
  "QUERY",
  "COMMAND",
  "WORKFLOW",
  "STREAM",
  "ADMINISTRATIVE",
] as const;
export type CapabilityClass = (typeof CAPABILITY_CLASSES)[number];

export const CAPABILITY_AVAILABILITY = [
  "AVAILABLE",
  "DEGRADED",
  "UNAVAILABLE",
  "UNCERTIFIED",
] as const;
export type CapabilityAvailability = (typeof CAPABILITY_AVAILABILITY)[number];

export enum CapabilityPresentation {
  /** Known vocabulary and available for rendering. */
  RENDER = "RENDER",
  /** Known vocabulary but deliberately hidden (e.g. not authorized). */
  HIDDEN = "HIDDEN",
  /** Unknown vocabulary: fail closed, never fabricated. */
  UNSUPPORTED = "UNSUPPORTED",
}

const PRESENTED_CAPABILITY_FIELDS = new Set<string>([
  "capability_id",
  "class",
  "availability",
  "visible",
  "authorized",
  "required_approval",
]);

export interface PresentedCapabilityShape {
  capability_id: string;
  class: CapabilityClass;
  availability: CapabilityAvailability;
  visible: boolean;
  authorized: boolean;
  required_approval: string;
}

/**
 * A capability as presented by the UI. `visible` (may be rendered) and
 * `authorized` (backend grants the action) are independent fields;
 * invoking an action requires authorized=true regardless of visibility.
 */
export class PresentedCapability {
  readonly capability_id: string;
  readonly class: CapabilityClass;
  readonly availability: CapabilityAvailability;
  readonly visible: boolean;
  readonly authorized: boolean;
  readonly required_approval: string;

  private constructor(shape: PresentedCapabilityShape) {
    this.capability_id = shape.capability_id;
    this.class = shape.class;
    this.availability = shape.availability;
    this.visible = shape.visible;
    this.authorized = shape.authorized;
    this.required_approval = shape.required_approval;
  }

  static fromWire(value: unknown): PresentedCapability {
    const obj = assertObject(value, "PresentedCapability");
    rejectUnknownFields(
      obj,
      PRESENTED_CAPABILITY_FIELDS,
      "PresentedCapability",
    );
    return new PresentedCapability({
      capability_id: assertString(obj.capability_id, "capability_id"),
      class: assertEnum(
        obj.class,
        new Set<CapabilityClass>(CAPABILITY_CLASSES),
        "class",
      ),
      availability: assertEnum(
        obj.availability,
        new Set<CapabilityAvailability>(CAPABILITY_AVAILABILITY),
        "availability",
      ),
      visible: obj.visible === true,
      authorized: obj.authorized === true,
      required_approval: assertString(
        obj.required_approval,
        "required_approval",
      ),
    });
  }

  /** The UI may invoke only when the backend authorizes the capability. */
  get invocable(): boolean {
    return this.authorized;
  }

  /** An unavailable or uncertified capability is never presented as live. */
  get operational(): boolean {
    return (
      this.availability === "AVAILABLE" || this.availability === "DEGRADED"
    );
  }
}

/**
 * Known capability vocabulary. A capability id outside this set is
 * UNSUPPORTED and must fail closed (directive E). The set is seeded
 * from the canonical repository capability namespace style
 * (e.g. "home.lights.query").
 */
export class KnownCapabilityVocabulary {
  readonly #known: ReadonlySet<string>;

  constructor(known: Iterable<string>) {
    const set = new Set<string>();
    for (const id of known) {
      if (typeof id !== "string" || id.length === 0) {
        throw new Error("capability id must be a non-empty string");
      }
      set.add(id);
    }
    this.#known = set;
  }

  isKnown(capabilityId: string): boolean {
    return this.#known.has(capabilityId);
  }

  /**
   * Resolve a canonical descriptor against the known vocabulary.
   * Unknown ids resolve to UNSUPPORTED regardless of the descriptor's
   * own claims (fail closed, never fabricated).
   */
  resolve(descriptor: CapabilityDescriptor): CapabilityPresentation {
    if (!this.isKnown(descriptor.id)) {
      return CapabilityPresentation.UNSUPPORTED;
    }
    if (descriptor.availability === "UNAVAILABLE") {
      return CapabilityPresentation.HIDDEN;
    }
    return CapabilityPresentation.RENDER;
  }

  resolveId(capabilityId: string): CapabilityPresentation {
    return this.isKnown(capabilityId)
      ? CapabilityPresentation.RENDER
      : CapabilityPresentation.UNSUPPORTED;
  }
}
