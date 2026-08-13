/**
 * Workflow contract versioning strategy (SPEC-023 behavior 8; ADR-010).
 *
 * SPEC-023 requires: "Workflow and event schema upgrades preserve in-flight
 * compatibility." This module pins the version constants and the
 * compatibility rules every workflow contract must follow. The full
 * strategy (Temporal patched/version APIs, task-queue-per-version,
 * compatible sets, replay semantics) is documented in docs/versioning.md
 * and enforced here where it can be checked statically.
 */

import { WorkflowContractError } from "./errors.js";
import { workflowKind } from "./vocabulary.js";
import type { WorkflowKind } from "./vocabulary.js";

/** Contract surface version of the whole workflows vocabulary. */
export const WORKFLOW_CONTRACT_VERSION = "1.0.0";
export const SIGNAL_SCHEMA_VERSION = "1.0.0";
export const QUERY_SCHEMA_VERSION = "1.0.0";
export const ACTIVITY_CONTRACT_VERSION = "1.0.0";

export interface VersionCompatibility {
  readonly workflowKind: WorkflowKind;
  /** Lowest signal schema version this workflow version understands. */
  readonly minSupportedSignalVersion: string;
  /** Lowest query schema version this workflow version answers. */
  readonly minSupportedQueryVersion: string;
}

export const WORKFLOW_COMPATIBILITY: readonly VersionCompatibility[] = [
  {
    workflowKind: "OBJECTIVE",
    minSupportedSignalVersion: "1.0.0",
    minSupportedQueryVersion: "1.0.0",
  },
  {
    workflowKind: "APPROVAL",
    minSupportedSignalVersion: "1.0.0",
    minSupportedQueryVersion: "1.0.0",
  },
  {
    workflowKind: "CONNECTOR_CERTIFICATION",
    minSupportedSignalVersion: "1.0.0",
    minSupportedQueryVersion: "1.0.0",
  },
  {
    workflowKind: "INCIDENT_REMEDIATION",
    minSupportedSignalVersion: "1.0.0",
    minSupportedQueryVersion: "1.0.0",
  },
  {
    workflowKind: "DEPLOYMENT",
    minSupportedSignalVersion: "1.0.0",
    minSupportedQueryVersion: "1.0.0",
  },
];

export interface Semver {
  readonly major: number;
  readonly minor: number;
  readonly patch: number;
}

const SEMVER_RE = /^(\d+)\.(\d+)\.(\d+)$/;

export function parseSemver(value: string, context = "version"): Semver {
  const match = SEMVER_RE.exec(value);
  if (match === null) {
    throw new WorkflowContractError(
      `${context} must be semver MAJOR.MINOR.PATCH, got ${JSON.stringify(value)}`,
    );
  }
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
  };
}

/**
 * True when a peer schema version is accepted by a workflow contract.
 * Compatibility is major-scoped: same major is compatible; a different
 * major is rejected fail-closed (never silently reinterpreted). Minor and
 * patch additions within a major are additive by contract rule (see
 * docs/versioning.md), so a newer peer minor on the same major is still
 * accepted by an older contract.
 */
export function isCompatibleSchemaVersion(
  peerVersion: string,
  contractVersion: string,
): boolean {
  const peer = parseSemver(peerVersion, "peerVersion");
  const contract = parseSemver(contractVersion, "contractVersion");
  return peer.major === contract.major;
}

/**
 * Signal compatibility: a signal is accepted only when its schema version
 * is in the same major as, and not above, the workflow's declared support.
 */
export function isCompatibleSignalVersion(
  signalVersion: string,
  workflow: VersionCompatibility,
): boolean {
  const peer = parseSemver(signalVersion, "signalVersion");
  const min = parseSemver(
    workflow.minSupportedSignalVersion,
    "minSupportedSignalVersion",
  );
  const current = parseSemver(SIGNAL_SCHEMA_VERSION, "SIGNAL_SCHEMA_VERSION");
  if (peer.major !== current.major) {
    return false;
  }
  if (peer.major < min.major) {
    return false;
  }
  if (peer.major === min.major && peer.minor < min.minor) {
    return false;
  }
  return true;
}

export function compatibilityFor(kind: WorkflowKind): VersionCompatibility {
  workflowKind.parse(kind, "workflowKind");
  const found = WORKFLOW_COMPATIBILITY.find((c) => c.workflowKind === kind);
  if (found === undefined) {
    throw new WorkflowContractError(
      `no version compatibility declared for workflow kind ${kind}`,
    );
  }
  return found;
}
