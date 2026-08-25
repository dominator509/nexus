/**
 * EP-042 M2 deterministic compatibility evaluation (SPEC-016).
 *
 * Pure and deterministic: the same input always yields the same
 * decision, with reasons sorted canonically so filesystem or
 * serialization order cannot change the result. Fail-closed on unknown
 * components, mismatched versions, unparseable versions, and missing
 * entries.
 *
 * Permanent invariant:
 * - COMPATIBILITY MATRIX EXISTS != UPDATE SAFE
 * - COMPATIBILITY METADATA EXISTS != UPDATE SAFE
 */

import { ReleaseError, ReleaseErrorCode } from "./errors";
import type {
  CompatibilityMatrix,
  DeploymentProfileMode,
  SignedComponent,
} from "./types";

export interface CompatibleVerdict {
  compatible: boolean;
  reasons: ReadonlyArray<string>;
}

export function compatibleOk(): CompatibleVerdict {
  return { compatible: true, reasons: [] };
}

export function compatibleDeny(
  reasons: ReadonlyArray<string>,
): CompatibleVerdict {
  return { compatible: false, reasons };
}

/**
 * Three-part numeric version comparison (major.minor.patch). Any
 * non-numeric component fails closed (returns undefined), never guessed.
 */
export function compareVersions(a: string, b: string): number | undefined {
  const parse = (v: string): [number, number, number] | undefined => {
    const core = v.split(/[-+]/)[0] ?? "";
    const parts = core.split(".");
    if (parts.length === 0 || parts.length > 3) {
      return undefined;
    }
    const out: [number, number, number] = [0, 0, 0];
    for (let i = 0; i < parts.length; i += 1) {
      const part = parts[i] ?? "";
      if (part === "" || !/^\d+$/.test(part)) {
        return undefined;
      }
      out[i] = Number(part);
      if (!Number.isSafeInteger(out[i])) {
        return undefined;
      }
    }
    return out;
  };
  const pa = parse(a);
  const pb = parse(b);
  if (pa === undefined || pb === undefined) {
    return undefined;
  }
  for (let i = 0; i < 3; i += 1) {
    const da = pa[i] ?? 0;
    const db = pb[i] ?? 0;
    if (da < db) {
      return -1;
    }
    if (da > db) {
      return 1;
    }
  }
  return 0;
}

/**
 * Fail-closed compatibility check over a component set. Every component
 * must appear in the matrix with an exact version match and a version
 * within the declared bounds; any unknown or unparseable entry denies.
 */
export function evaluateCompatibility(
  matrix: CompatibilityMatrix,
  components: ReadonlyArray<SignedComponent>,
): CompatibleVerdict {
  const reasons: Array<string> = [];
  for (const component of components) {
    const entry = matrix.entries.find(
      (candidate) => candidate.component_id === component.component_id,
    );
    if (entry === undefined) {
      reasons.push(
        `component ${component.component_id} is not present in the compatibility matrix`,
      );
      continue;
    }
    if (entry.version !== component.version) {
      reasons.push(
        `component ${component.component_id} version ${component.version} does not match matrix version ${entry.version}`,
      );
    }
    const belowMin = compareVersions(component.version, entry.min_version);
    if (belowMin === undefined || belowMin < 0) {
      reasons.push(
        `component ${component.component_id} version ${component.version} is below matrix minimum ${entry.min_version}`,
      );
    }
    const aboveMax = compareVersions(component.version, entry.max_version);
    if (aboveMax === undefined || aboveMax > 0) {
      reasons.push(
        `component ${component.component_id} version ${component.version} exceeds matrix maximum ${entry.max_version}`,
      );
    }
  }
  if (reasons.length === 0) {
    return compatibleOk();
  }
  // Deterministic: reasons sorted canonically regardless of input order.
  reasons.sort((x, y) => (x < y ? -1 : x > y ? 1 : 0));
  return compatibleDeny(reasons);
}

/** True when every matrix entry declares support for the profile. */
export function matrixSupportsProfile(
  matrix: CompatibilityMatrix,
  profile: DeploymentProfileMode,
): boolean {
  return matrix.entries.every((entry) =>
    entry.supported_profiles.includes(profile),
  );
}

/** One signed distribution supports every canonical profile. */
export function matrixSupportsAllProfiles(
  matrix: CompatibilityMatrix,
): boolean {
  const profiles: ReadonlyArray<DeploymentProfileMode> = [
    "MANAGED",
    "BYOC",
    "EXISTING_SSH",
    "HYBRID",
    "FULLY_LOCAL",
  ];
  return profiles.every((profile) => matrixSupportsProfile(matrix, profile));
}

/**
 * Assert the component set is compatible with the matrix and the target
 * profile. Throws ReleaseError on any denial; used by the planner.
 */
export function assertCompatibleForProfile(
  matrix: CompatibilityMatrix,
  components: ReadonlyArray<SignedComponent>,
  profile: DeploymentProfileMode,
): void {
  const verdict = evaluateCompatibility(matrix, components);
  if (!verdict.compatible) {
    throw new ReleaseError(
      ReleaseErrorCode.Incompatible,
      `component set is not compatible: ${verdict.reasons.join("; ")}`,
      { field: "compatibility" },
    );
  }
  if (!matrixSupportsProfile(matrix, profile)) {
    throw new ReleaseError(
      ReleaseErrorCode.Incompatible,
      `compatibility matrix does not support deployment profile ${profile}`,
      { field: "compatibility.supported_profiles" },
    );
  }
}
