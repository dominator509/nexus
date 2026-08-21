/**
 * EP-035 M1 HardwareProfiler contract (SPEC-004 / SPEC-016).
 *
 * Hardware facts carry their actual provenance; "user says RTX GPU" is
 * a USER_DECLARED fact, never a detected GPU. Observed cores, RAM, GPU
 * model, and disk size never automatically become claims such as
 * "local LLM supported", "Whisper real-time capable", "Frigate capacity
 * sufficient", "vision acceleration active", or "latency target
 * achieved". Those need later measured proof. M1 exposes capability
 * declarations, never performance certification.
 */

import {
  assertEnum,
  assertNonNegativeInt,
  assertObject,
  assertNonEmptyString,
  assertString,
  assertUuid,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const HARDWARE_PROVENANCES = [
  "USER_DECLARED",
  "HOST_OBSERVED",
  "PLATFORM_REPORTED",
  "BENCHMARKED",
  "HARDWARE_CERTIFIED",
] as const;
export type HardwareProvenance = (typeof HARDWARE_PROVENANCES)[number];
const HARDWARE_PROVENANCE_SET: ReadonlySet<HardwareProvenance> = new Set(
  HARDWARE_PROVENANCES,
);

export const CAPABILITY_CERTIFICATION_STATES = [
  "NOT_CERTIFIED",
  "CERTIFIED",
] as const;
export type CapabilityCertificationState =
  (typeof CAPABILITY_CERTIFICATION_STATES)[number];
const CAPABILITY_CERTIFICATION_STATE_SET: ReadonlySet<CapabilityCertificationState> =
  new Set(CAPABILITY_CERTIFICATION_STATES);

export interface HardwareFactShape {
  key: string;
  value: string | number;
  provenance: HardwareProvenance;
  observed_at_unix_s?: number;
}

const HARDWARE_FACT_FIELDS = new Set<string>([
  "key",
  "value",
  "provenance",
  "observed_at_unix_s",
]);

export class HardwareFact {
  readonly key: string;
  readonly value: string | number;
  readonly provenance: HardwareProvenance;
  readonly observed_at_unix_s: number | undefined;

  private constructor(
    key: string,
    value: string | number,
    provenance: HardwareProvenance,
    observedAtUnixS: number | undefined,
  ) {
    this.key = key;
    this.value = value;
    this.provenance = provenance;
    this.observed_at_unix_s = observedAtUnixS;
  }

  static parse(value: unknown, what = "hardware fact"): HardwareFact {
    const obj = assertObject(value, what);
    rejectUnknownFields(obj, HARDWARE_FACT_FIELDS, what);
    const key = assertNonEmptyString(obj["key"], `${what}.key`);
    const rawValue = obj["value"];
    if (typeof rawValue !== "string" && typeof rawValue !== "number") {
      throw new Spec006Error(
        ErrorCode.Validation,
        `${what}.value must be a string or number`,
      );
    }
    if (typeof rawValue === "number" && !Number.isFinite(rawValue)) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `${what}.value must be finite`,
      );
    }
    const provenance = assertEnum(
      obj["provenance"],
      HARDWARE_PROVENANCE_SET,
      `${what}.provenance`,
    );
    const observedRaw = obj["observed_at_unix_s"];
    const observed =
      observedRaw === undefined
        ? undefined
        : assertNonNegativeInt(observedRaw, `${what}.observed_at_unix_s`);
    return new HardwareFact(key, rawValue, provenance, observed);
  }

  toJSON(): HardwareFactShape {
    return {
      key: this.key,
      value: this.value,
      provenance: this.provenance,
      ...(this.observed_at_unix_s === undefined
        ? {}
        : { observed_at_unix_s: this.observed_at_unix_s }),
    };
  }
}

export interface HardwareCapabilityDeclarationShape {
  capability_id: string;
  declaration_provenance: HardwareProvenance;
  certification: CapabilityCertificationState;
  measured_evidence_id?: string;
}

const HARDWARE_CAPABILITY_DECLARATION_FIELDS = new Set<string>([
  "capability_id",
  "declaration_provenance",
  "certification",
  "measured_evidence_id",
]);

/**
 * A capability declaration is a claim, never a certification.
 * CERTIFIED requires measured evidence AND a measured provenance
 * (BENCHMARKED / HARDWARE_CERTIFIED); any other combination fails
 * closed at parse time.
 */
export class HardwareCapabilityDeclaration {
  readonly capability_id: string;
  readonly declaration_provenance: HardwareProvenance;
  readonly certification: CapabilityCertificationState;
  readonly measured_evidence_id: string | undefined;

  private constructor(
    capabilityId: string,
    declarationProvenance: HardwareProvenance,
    certification: CapabilityCertificationState,
    measuredEvidenceId: string | undefined,
  ) {
    this.capability_id = capabilityId;
    this.declaration_provenance = declarationProvenance;
    this.certification = certification;
    this.measured_evidence_id = measuredEvidenceId;
  }

  static parse(
    value: unknown,
    what = "hardware capability declaration",
  ): HardwareCapabilityDeclaration {
    const obj = assertObject(value, what);
    rejectUnknownFields(obj, HARDWARE_CAPABILITY_DECLARATION_FIELDS, what);
    const provenance = assertEnum(
      obj["declaration_provenance"],
      HARDWARE_PROVENANCE_SET,
      `${what}.declaration_provenance`,
    );
    const certification = assertEnum(
      obj["certification"],
      CAPABILITY_CERTIFICATION_STATE_SET,
      `${what}.certification`,
    );
    const evidenceRaw = obj["measured_evidence_id"];
    const evidence =
      evidenceRaw === undefined
        ? undefined
        : assertNonEmptyString(evidenceRaw, `${what}.measured_evidence_id`);
    if (certification === "CERTIFIED") {
      if (evidence === undefined) {
        throw new Spec006Error(
          ErrorCode.Verification,
          `${what} claims CERTIFIED without measured evidence`,
        );
      }
      if (provenance !== "BENCHMARKED" && provenance !== "HARDWARE_CERTIFIED") {
        throw new Spec006Error(
          ErrorCode.Verification,
          `${what} claims CERTIFIED from provenance ${provenance}; measured provenance required`,
        );
      }
    }
    return new HardwareCapabilityDeclaration(
      assertNonEmptyString(obj["capability_id"], `${what}.capability_id`),
      provenance,
      certification,
      evidence,
    );
  }

  toJSON(): HardwareCapabilityDeclarationShape {
    return {
      capability_id: this.capability_id,
      declaration_provenance: this.declaration_provenance,
      certification: this.certification,
      ...(this.measured_evidence_id === undefined
        ? {}
        : { measured_evidence_id: this.measured_evidence_id }),
    };
  }
}

export interface HardwareProfileShape {
  facts: Array<HardwareFactShape>;
  capability_declarations: Array<HardwareCapabilityDeclarationShape>;
  profiled_at_unix_s: number;
  correlation_id: string;
}

const HARDWARE_PROFILE_FIELDS = new Set<string>([
  "facts",
  "capability_declarations",
  "profiled_at_unix_s",
  "correlation_id",
]);

export class HardwareProfile {
  readonly facts: ReadonlyArray<HardwareFact>;
  readonly capability_declarations: ReadonlyArray<HardwareCapabilityDeclaration>;
  readonly profiled_at_unix_s: number;
  readonly correlation_id: string;

  private constructor(
    facts: ReadonlyArray<HardwareFact>,
    capabilityDeclarations: ReadonlyArray<HardwareCapabilityDeclaration>,
    profiledAtUnixS: number,
    correlationId: string,
  ) {
    this.facts = facts;
    this.capability_declarations = capabilityDeclarations;
    this.profiled_at_unix_s = profiledAtUnixS;
    this.correlation_id = correlationId;
  }

  static parse(value: unknown, what = "hardware profile"): HardwareProfile {
    const obj = assertObject(value, what);
    rejectUnknownFields(obj, HARDWARE_PROFILE_FIELDS, what);
    const factsRaw = obj["facts"];
    if (!Array.isArray(factsRaw)) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `${what}.facts must be an array`,
      );
    }
    const facts = factsRaw.map((entry) =>
      HardwareFact.parse(entry, `${what}.facts entry`),
    );
    const declarationsRaw = obj["capability_declarations"];
    if (!Array.isArray(declarationsRaw)) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `${what}.capability_declarations must be an array`,
      );
    }
    const declarations = declarationsRaw.map((entry) =>
      HardwareCapabilityDeclaration.parse(
        entry,
        `${what}.capability_declarations entry`,
      ),
    );
    return new HardwareProfile(
      facts,
      declarations,
      assertNonNegativeInt(
        obj["profiled_at_unix_s"],
        `${what}.profiled_at_unix_s`,
      ),
      assertUuid(obj["correlation_id"], `${what}.correlation_id`),
    );
  }

  toJSON(): HardwareProfileShape {
    return {
      facts: this.facts.map((entry) => entry.toJSON()),
      capability_declarations: this.capability_declarations.map((entry) =>
        entry.toJSON(),
      ),
      profiled_at_unix_s: this.profiled_at_unix_s,
      correlation_id: this.correlation_id,
    };
  }
}

export interface HardwareProfileRequestShape {
  correlation_id: string;
}

/** Provider-neutral HardwareProfiler port. M1 declares the boundary. */
export interface HardwareProfilerPort {
  profile(request: HardwareProfileRequestShape): HardwareProfileShape;
}
