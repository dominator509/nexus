/**
 * EP-035 M1 DiscoveryWizard contract (SPEC-004).
 *
 * Discovery returns OBSERVATIONS, not authority. A report may carry
 * services/devices found, endpoints, kinds, advertised capabilities,
 * and metadata - but discovery alone never authenticates, connects
 * permanently, trusts, enrolls, authorizes, or mutates configuration.
 * Discovery data is UNTRUSTED: hostile content such as "ADMIN",
 * "TRUSTED", "AUTO-APPROVE", or "OWNER DEVICE" is inert data and never
 * alters setup authority. An explicit governed transition (selection by
 * a principal) is required before any integration work begins.
 */

import {
  assertEnum,
  assertNonEmptyString,
  assertNonNegativeInt,
  assertObject,
  assertStringArray,
  assertUuid,
  rejectUnknownFields,
} from "./validate";

export const DISCOVERY_KINDS = ["SERVICE", "DEVICE", "EDGE"] as const;
export type DiscoveryKind = (typeof DISCOVERY_KINDS)[number];
const DISCOVERY_KIND_SET: ReadonlySet<DiscoveryKind> = new Set(DISCOVERY_KINDS);

export interface DiscoveryObservationShape {
  id: string;
  kind: DiscoveryKind;
  name: string;
  endpoint: string;
  advertised_capabilities: Array<string>;
  metadata: Record<string, unknown>;
  observed_at_unix_s: number;
}

const DISCOVERY_OBSERVATION_FIELDS = new Set<string>([
  "id",
  "kind",
  "name",
  "endpoint",
  "advertised_capabilities",
  "metadata",
  "observed_at_unix_s",
]);

/** Names that are hostile as authority claims but harmless as data. */
const HOSTILE_AUTHORITY_TOKENS = new Set<string>([
  "ADMIN",
  "TRUSTED",
  "AUTO-APPROVE",
  "AUTO_APPROVE",
  "OWNER DEVICE",
  "OWNER_DEVICE",
  "ROOT",
  "SUPERUSER",
]);

export class DiscoveryObservation {
  readonly id: string;
  readonly kind: DiscoveryKind;
  readonly name: string;
  readonly endpoint: string;
  readonly advertised_capabilities: ReadonlyArray<string>;
  readonly metadata: Record<string, unknown>;
  readonly observed_at_unix_s: number;

  private constructor(
    id: string,
    kind: DiscoveryKind,
    name: string,
    endpoint: string,
    advertisedCapabilities: ReadonlyArray<string>,
    metadata: Record<string, unknown>,
    observedAtUnixS: number,
  ) {
    this.id = id;
    this.kind = kind;
    this.name = name;
    this.endpoint = endpoint;
    this.advertised_capabilities = advertisedCapabilities;
    this.metadata = metadata;
    this.observed_at_unix_s = observedAtUnixS;
  }

  static parse(
    value: unknown,
    what = "discovery observation",
  ): DiscoveryObservation {
    const obj = assertObject(value, what);
    rejectUnknownFields(obj, DISCOVERY_OBSERVATION_FIELDS, what);
    const metadataRaw = obj["metadata"];
    if (
      typeof metadataRaw !== "object" ||
      metadataRaw === null ||
      Array.isArray(metadataRaw)
    ) {
      throw new Error(`${what}.metadata must be an object`);
    }
    return new DiscoveryObservation(
      assertUuid(obj["id"], `${what}.id`),
      assertEnum(obj["kind"], DISCOVERY_KIND_SET, `${what}.kind`),
      assertNonEmptyString(obj["name"], `${what}.name`),
      assertNonEmptyString(obj["endpoint"], `${what}.endpoint`),
      assertStringArray(
        obj["advertised_capabilities"],
        `${what}.advertised_capabilities`,
        128,
      ),
      metadataRaw as Record<string, unknown>,
      assertNonNegativeInt(
        obj["observed_at_unix_s"],
        `${what}.observed_at_unix_s`,
      ),
    );
  }

  /** Hostile discovery content is data, never authority. Informational only. */
  containsHostileAuthorityToken(): boolean {
    const haystack =
      `${this.name} ${this.endpoint} ${this.advertised_capabilities.join(" ")}`.toUpperCase();
    for (const token of HOSTILE_AUTHORITY_TOKENS) {
      if (haystack.includes(token)) {
        return true;
      }
    }
    return false;
  }

  toJSON(): DiscoveryObservationShape {
    return {
      id: this.id,
      kind: this.kind,
      name: this.name,
      endpoint: this.endpoint,
      advertised_capabilities: [...this.advertised_capabilities],
      metadata: { ...this.metadata },
      observed_at_unix_s: this.observed_at_unix_s,
    };
  }
}

export interface DiscoveryReportShape {
  observations: Array<DiscoveryObservationShape>;
  generated_at_unix_s: number;
  correlation_id: string;
}

const DISCOVERY_REPORT_FIELDS = new Set<string>([
  "observations",
  "generated_at_unix_s",
  "correlation_id",
]);

export class DiscoveryReport {
  readonly observations: ReadonlyArray<DiscoveryObservation>;
  readonly generated_at_unix_s: number;
  readonly correlation_id: string;

  private constructor(
    observations: ReadonlyArray<DiscoveryObservation>,
    generatedAtUnixS: number,
    correlationId: string,
  ) {
    this.observations = observations;
    this.generated_at_unix_s = generatedAtUnixS;
    this.correlation_id = correlationId;
  }

  static parse(value: unknown): DiscoveryReport {
    const obj = assertObject(value, "discovery report");
    rejectUnknownFields(obj, DISCOVERY_REPORT_FIELDS, "discovery report");
    const observationsRaw = obj["observations"];
    if (!Array.isArray(observationsRaw)) {
      throw new Error("discovery report.observations must be an array");
    }
    return new DiscoveryReport(
      observationsRaw.map((entry) =>
        DiscoveryObservation.parse(
          entry,
          "discovery report.observations entry",
        ),
      ),
      assertNonNegativeInt(
        obj["generated_at_unix_s"],
        "discovery report.generated_at_unix_s",
      ),
      assertUuid(obj["correlation_id"], "discovery report.correlation_id"),
    );
  }

  toJSON(): DiscoveryReportShape {
    return {
      observations: this.observations.map((entry) => entry.toJSON()),
      generated_at_unix_s: this.generated_at_unix_s,
      correlation_id: this.correlation_id,
    };
  }
}

export interface IntegrationSelectionShape {
  observation_id: string;
  selected_by: string;
  selected_at_unix_s: number;
  correlation_id: string;
}

const INTEGRATION_SELECTION_FIELDS = new Set<string>([
  "observation_id",
  "selected_by",
  "selected_at_unix_s",
  "correlation_id",
]);

/**
 * The explicit governed transition from discovery to integration work:
 * a principal selects an observation. Selection is still NOT
 * enrollment/authorization; it is the one legitimate bridge from
 * observation to configuration, and it always records the actor.
 */
export class IntegrationSelection {
  readonly observation_id: string;
  readonly selected_by: string;
  readonly selected_at_unix_s: number;
  readonly correlation_id: string;

  private constructor(
    observationId: string,
    selectedBy: string,
    selectedAtUnixS: number,
    correlationId: string,
  ) {
    this.observation_id = observationId;
    this.selected_by = selectedBy;
    this.selected_at_unix_s = selectedAtUnixS;
    this.correlation_id = correlationId;
  }

  static parse(value: unknown): IntegrationSelection {
    const obj = assertObject(value, "integration selection");
    rejectUnknownFields(
      obj,
      INTEGRATION_SELECTION_FIELDS,
      "integration selection",
    );
    return new IntegrationSelection(
      assertUuid(obj["observation_id"], "integration selection.observation_id"),
      assertUuid(obj["selected_by"], "integration selection.selected_by"),
      assertNonNegativeInt(
        obj["selected_at_unix_s"],
        "integration selection.selected_at_unix_s",
      ),
      assertUuid(obj["correlation_id"], "integration selection.correlation_id"),
    );
  }
}

/** Provider-neutral DiscoveryWizard port. M1 declares the boundary. */
export interface DiscoveryWizardPort {
  observe(correlationId: string): DiscoveryReportShape;
  select(selection: IntegrationSelectionShape): IntegrationSelectionShape;
}
