/**
 * EP-035 M1 IntegrationCard contract (SPEC-004).
 *
 * An IntegrationCard is TRUTHFUL about configuration versus health.
 * The status vocabulary is deliberately fine-grained:
 *
 *   UNCONFIGURED != CONFIGURED != AUTHENTICATED != REACHABLE
 *   REACHABLE != HEALTHY, with DEGRADED and ERROR distinct
 *
 * "credential exists" never becomes HEALTHY; "endpoint entered" never
 * becomes CONNECTED; "component installed" never becomes WORKING.
 * Advertised capabilities are never derived from a provider name: an
 * IntegrationCard("Home Assistant") with no capability data advertises
 * nothing.
 */

import {
  assertEnum,
  assertNonEmptyString,
  assertNonNegativeInt,
  assertObject,
  assertOptionalString,
  assertStringArray,
  assertUuid,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const INTEGRATION_STATUSES = [
  "UNCONFIGURED",
  "CONFIGURED",
  "AUTHENTICATED",
  "REACHABLE",
  "DEGRADED",
  "HEALTHY",
  "ERROR",
] as const;
export type IntegrationStatus = (typeof INTEGRATION_STATUSES)[number];
const INTEGRATION_STATUS_SET: ReadonlySet<IntegrationStatus> = new Set(
  INTEGRATION_STATUSES,
);

export interface IntegrationCardShape {
  integration_id: string;
  provider_name: string;
  status: IntegrationStatus;
  advertised_capabilities: Array<string>;
  configured_at_unix_s?: number;
  last_verified_at_unix_s?: number;
  correlation_id: string;
}

const INTEGRATION_CARD_FIELDS = new Set<string>([
  "integration_id",
  "provider_name",
  "status",
  "advertised_capabilities",
  "configured_at_unix_s",
  "last_verified_at_unix_s",
  "correlation_id",
]);

const INTEGRATION_TRANSITIONS: Readonly<
  Record<IntegrationStatus, ReadonlySet<IntegrationStatus>>
> = {
  UNCONFIGURED: new Set<IntegrationStatus>(["CONFIGURED", "ERROR"]),
  CONFIGURED: new Set<IntegrationStatus>([
    "AUTHENTICATED",
    "ERROR",
    "DEGRADED",
  ]),
  AUTHENTICATED: new Set<IntegrationStatus>(["REACHABLE", "ERROR", "DEGRADED"]),
  REACHABLE: new Set<IntegrationStatus>(["HEALTHY", "DEGRADED", "ERROR"]),
  DEGRADED: new Set<IntegrationStatus>(["REACHABLE", "ERROR"]),
  HEALTHY: new Set<IntegrationStatus>(["DEGRADED", "ERROR"]),
  ERROR: new Set<IntegrationStatus>(["CONFIGURED"]),
};

export function isValidIntegrationStatusTransition(
  from: IntegrationStatus,
  to: IntegrationStatus,
): boolean {
  const allowed = INTEGRATION_TRANSITIONS[from];
  return allowed !== undefined && allowed.has(to);
}

export class IntegrationCardData {
  readonly integration_id: string;
  readonly provider_name: string;
  readonly status: IntegrationStatus;
  readonly advertised_capabilities: ReadonlyArray<string>;
  readonly configured_at_unix_s: number | undefined;
  readonly last_verified_at_unix_s: number | undefined;
  readonly correlation_id: string;

  private constructor(
    integrationId: string,
    providerName: string,
    status: IntegrationStatus,
    advertisedCapabilities: ReadonlyArray<string>,
    configuredAtUnixS: number | undefined,
    lastVerifiedAtUnixS: number | undefined,
    correlationId: string,
  ) {
    this.integration_id = integrationId;
    this.provider_name = providerName;
    this.status = status;
    this.advertised_capabilities = advertisedCapabilities;
    this.configured_at_unix_s = configuredAtUnixS;
    this.last_verified_at_unix_s = lastVerifiedAtUnixS;
    this.correlation_id = correlationId;
  }

  static parse(value: unknown, what = "integration card"): IntegrationCardData {
    const obj = assertObject(value, what);
    rejectUnknownFields(obj, INTEGRATION_CARD_FIELDS, what);
    const status = assertEnum(
      obj["status"],
      INTEGRATION_STATUS_SET,
      `${what}.status`,
    );
    const configuredRaw = obj["configured_at_unix_s"];
    const configured =
      configuredRaw === undefined
        ? undefined
        : assertNonNegativeInt(configuredRaw, `${what}.configured_at_unix_s`);
    const verifiedRaw = obj["last_verified_at_unix_s"];
    const verified =
      verifiedRaw === undefined
        ? undefined
        : assertNonNegativeInt(verifiedRaw, `${what}.last_verified_at_unix_s`);

    // Truthfulness invariants.
    if (status === "UNCONFIGURED" && configured !== undefined) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `${what} is UNCONFIGURED but carries a configured_at_unix_s`,
      );
    }
    if (status !== "UNCONFIGURED" && configured === undefined) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `${what} status ${status} requires configured_at_unix_s`,
      );
    }
    if (
      (status === "REACHABLE" ||
        status === "HEALTHY" ||
        status === "DEGRADED") &&
      verified === undefined
    ) {
      throw new Spec006Error(
        ErrorCode.Verification,
        `${what} status ${status} requires last_verified_at_unix_s`,
      );
    }
    if (status === "UNCONFIGURED" && verified !== undefined) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `${what} is UNCONFIGURED but carries a last_verified_at_unix_s`,
      );
    }

    return new IntegrationCardData(
      assertUuid(obj["integration_id"], `${what}.integration_id`),
      assertNonEmptyString(obj["provider_name"], `${what}.provider_name`),
      status,
      assertStringArray(
        obj["advertised_capabilities"],
        `${what}.advertised_capabilities`,
        256,
      ),
      configured,
      verified,
      assertUuid(obj["correlation_id"], `${what}.correlation_id`),
    );
  }

  /**
   * Truthful transition: HEALTHY can only be reached from REACHABLE and
   * requires a verification timestamp.
   */
  transition(
    toStatus: IntegrationStatus,
    atUnixS: number,
  ): IntegrationCardData {
    if (!isValidIntegrationStatusTransition(this.status, toStatus)) {
      throw new Spec006Error(
        ErrorCode.Policy,
        `invalid integration status transition ${this.status} -> ${toStatus}`,
        this.correlation_id,
      );
    }
    if (
      (toStatus === "REACHABLE" ||
        toStatus === "HEALTHY" ||
        toStatus === "DEGRADED") &&
      this.last_verified_at_unix_s === undefined
    ) {
      throw new Spec006Error(
        ErrorCode.Verification,
        `integration cannot become ${toStatus} without a verification event`,
        this.correlation_id,
      );
    }
    return new IntegrationCardData(
      this.integration_id,
      this.provider_name,
      toStatus,
      this.advertised_capabilities,
      this.configured_at_unix_s,
      toStatus === "ERROR" ? this.last_verified_at_unix_s : atUnixS,
      this.correlation_id,
    );
  }

  toJSON(): IntegrationCardShape {
    return {
      integration_id: this.integration_id,
      provider_name: this.provider_name,
      status: this.status,
      advertised_capabilities: [...this.advertised_capabilities],
      ...(this.configured_at_unix_s === undefined
        ? {}
        : { configured_at_unix_s: this.configured_at_unix_s }),
      ...(this.last_verified_at_unix_s === undefined
        ? {}
        : { last_verified_at_unix_s: this.last_verified_at_unix_s }),
      correlation_id: this.correlation_id,
    };
  }
}

export interface IntegrationCardRequestShape {
  integration_id: string;
  provider_name: string;
  correlation_id: string;
  advertised_capabilities?: Array<string>;
}

const INTEGRATION_CARD_REQUEST_FIELDS = new Set<string>([
  "integration_id",
  "provider_name",
  "correlation_id",
  "advertised_capabilities",
]);

export class IntegrationCardRequest {
  readonly integration_id: string;
  readonly provider_name: string;
  readonly correlation_id: string;
  readonly advertised_capabilities: ReadonlyArray<string>;

  private constructor(
    integrationId: string,
    providerName: string,
    correlationId: string,
    advertisedCapabilities: ReadonlyArray<string>,
  ) {
    this.integration_id = integrationId;
    this.provider_name = providerName;
    this.correlation_id = correlationId;
    this.advertised_capabilities = advertisedCapabilities;
  }

  static parse(value: unknown): IntegrationCardRequest {
    const obj = assertObject(value, "integration card request");
    rejectUnknownFields(
      obj,
      INTEGRATION_CARD_REQUEST_FIELDS,
      "integration card request",
    );
    return new IntegrationCardRequest(
      assertUuid(
        obj["integration_id"],
        "integration card request.integration_id",
      ),
      assertNonEmptyString(
        obj["provider_name"],
        "integration card request.provider_name",
      ),
      assertUuid(
        obj["correlation_id"],
        "integration card request.correlation_id",
      ),
      assertStringArray(
        obj["advertised_capabilities"] ?? [],
        "integration card request.advertised_capabilities",
        256,
      ),
    );
  }
}

/** Provider-neutral IntegrationCard port. M1 declares the boundary. */
export interface IntegrationCardPort {
  create(request: IntegrationCardRequestShape): IntegrationCardShape;
  transition(
    integrationId: string,
    toStatus: IntegrationStatus,
    atUnixS: number,
  ): IntegrationCardShape;
}
