/**
 * EP-033 M1 ProviderSettings contract (SPEC-004 behavior 8).
 *
 * Before activation, the settings surface displays provider
 * certification, self-hosted or API route, cost, privacy, and data
 * egress. Activation is a backend-authority action: the settings UI
 * renders the disclosure and issues a typed command request; it never
 * activates a provider itself.
 */

import { assertEnum, assertObject, assertString, rejectUnknownFields } from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const PROVIDER_ROUTES = ["SELF_HOSTED", "API", "HYBRID"] as const;
export type ProviderRoute = (typeof PROVIDER_ROUTES)[number];

export const PROVIDER_CERTIFICATION = [
  "NOT_IMPLEMENTED",
  "IMPLEMENTED",
  "INTERNAL_CERTIFIED",
  "PROVIDER_CERTIFIED",
  "HARDWARE_CERTIFIED",
  "PRODUCTION_CERTIFIED",
] as const;
export type ProviderCertification = (typeof PROVIDER_CERTIFICATION)[number];

const PROVIDER_DISCLOSURE_FIELDS = new Set<string>([
  "provider_id",
  "display_name",
  "route",
  "certification",
  "cost_description",
  "privacy_class",
  "egress_description",
  "correlation",
]);

export interface ProviderDisclosureShape {
  provider_id: string;
  display_name: string;
  route: ProviderRoute;
  certification: ProviderCertification;
  cost_description: string;
  privacy_class: string;
  egress_description: string;
  correlation: string;
}

/**
 * The activation disclosure. `certification` is displayed verbatim
 * from backend certification records: an UNCERTIFIED provider is never
 * presented as operational.
 */
export class ProviderDisclosure {
  readonly provider_id: string;
  readonly display_name: string;
  readonly route: ProviderRoute;
  readonly certification: ProviderCertification;
  readonly cost_description: string;
  readonly privacy_class: string;
  readonly egress_description: string;
  readonly correlation: string;

  private constructor(shape: ProviderDisclosureShape) {
    this.provider_id = shape.provider_id;
    this.display_name = shape.display_name;
    this.route = shape.route;
    this.certification = shape.certification;
    this.cost_description = shape.cost_description;
    this.privacy_class = shape.privacy_class;
    this.egress_description = shape.egress_description;
    this.correlation = shape.correlation;
  }

  static fromWire(value: unknown): ProviderDisclosure {
    const obj = assertObject(value, "ProviderDisclosure");
    rejectUnknownFields(obj, PROVIDER_DISCLOSURE_FIELDS, "ProviderDisclosure");
    return new ProviderDisclosure({
      provider_id: assertString(obj.provider_id, "provider_id"),
      display_name: assertString(obj.display_name, "display_name"),
      route: assertEnum(obj.route, new Set<ProviderRoute>(PROVIDER_ROUTES), "route"),
      certification: assertEnum(
        obj.certification,
        new Set<ProviderCertification>(PROVIDER_CERTIFICATION),
        "certification",
      ),
      cost_description: assertString(obj.cost_description, "cost_description"),
      privacy_class: assertString(obj.privacy_class, "privacy_class"),
      egress_description: assertString(obj.egress_description, "egress_description"),
      correlation: assertString(obj.correlation, "correlation"),
    });
  }

  /**
   * Whether activation may even be offered. Fails closed: a provider
   * without at least internal certification is not activatable from
   * the UI.
   */
  get activatable(): boolean {
    return (
      this.certification === "INTERNAL_CERTIFIED" ||
      this.certification === "PROVIDER_CERTIFIED" ||
      this.certification === "HARDWARE_CERTIFIED" ||
      this.certification === "PRODUCTION_CERTIFIED"
    );
  }

  /** The disclosure must be acknowledged before any activation request. */
  static requiresAcknowledgment(_disclosure: ProviderDisclosure): boolean {
    return true;
  }
}

export class ProviderSettings {
  readonly providers: ReadonlyArray<ProviderDisclosure>;
  readonly correlation: string;

  constructor(providers: ReadonlyArray<ProviderDisclosure>, correlation: string) {
    const ids = new Set<string>();
    for (const provider of providers) {
      if (ids.has(provider.provider_id)) {
        throw new Spec006Error(ErrorCode.Conflict, `Duplicate provider '${provider.provider_id}'`);
      }
      ids.add(provider.provider_id);
    }
    this.providers = [...providers];
    this.correlation = correlation;
  }
}
