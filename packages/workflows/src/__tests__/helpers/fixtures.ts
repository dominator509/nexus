/**
 * Test-zone fixtures and helpers for the ep006_unit_* suite.
 * TESTING.md test-double zone: src/__tests__/ is test code, never
 * production surface.
 */

import { fileURLToPath } from "node:url";
import path from "node:path";
import { readdirSync, readFileSync, statSync } from "node:fs";

import { findDeterminismViolations } from "../../determinism.js";
import type { DeterminismViolation } from "../../determinism.js";
import {
  parseActionDigest,
  parseActionId,
  parseSignalId,
  parseWorkflowId,
} from "../../ids.js";
import type {
  ApprovalSignal,
  AuthenticationContext,
  PrincipalRef,
} from "../../index.js";

/** Repo root, resolved from this file: helpers -> __tests__ -> src -> workflows -> packages -> root. */
export const repoRoot = fileURLToPath(
  new URL("../../../../../", import.meta.url),
);
export const workflowsPackageRoot = path.join(repoRoot, "packages/workflows");

// --- Valid fixtures -------------------------------------------------------

export const WID_A = "0193a1f2-0000-7000-8000-000000000001";
export const WID_B = "0193a1f2-0000-7000-8000-000000000002";
export const WID_C = "0193a1f2-0000-7000-8000-000000000003";

export const signalIdA = parseSignalId(WID_A);
export const signalIdB = parseSignalId(WID_B);
export const workflowIdA = parseWorkflowId(WID_A);
export const workflowIdB = parseWorkflowId(WID_B);
export const actionIdA = parseActionId(WID_C);

export const DIGEST_A = "a".repeat(64);
export const DIGEST_B = "b".repeat(64);
export const actionDigestA = parseActionDigest(DIGEST_A);
export const actionDigestB = parseActionDigest(DIGEST_B);

export const ISO_A = "2026-08-13T00:00:00Z";
export const ISO_OFFSET = "2026-08-13T00:00:00+02:00";
export const ISO_FRACTION = "2026-08-13T00:00:00.123456789Z";

export const PRINCIPAL_HUMAN: PrincipalRef = { id: "p-hob", type: "HUMAN" };
export const AUTH_STEP_UP: AuthenticationContext = {
  strength: "STEP_UP",
  method: "passkey",
  sessionId: "sess-1",
  verifiedAt: ISO_A,
};

export function makeApprovalSignal(
  overrides: Partial<ApprovalSignal> = {},
): ApprovalSignal {
  const base: ApprovalSignal = {
    signalType: "APPROVAL",
    signalId: signalIdA,
    workflowId: workflowIdA,
    actionId: actionIdA,
    actionDigest: actionDigestA,
    // Clone the shared fixtures so a test mutating one signal's
    // principal/authentication never leaks into another signal.
    principal: { ...PRINCIPAL_HUMAN },
    authentication: { ...AUTH_STEP_UP },
    decision: "APPROVE",
    decidedAt: ISO_A,
  };
  return { ...base, ...overrides };
}

// --- Source audit helpers (test zone) -------------------------------------

export function workflowSourceFiles(packageRoot: string): string[] {
  const srcDir = path.join(packageRoot, "src");
  const out: string[] = [];
  const walk = (dir: string): void => {
    for (const entry of readdirSync(dir)) {
      const full = path.join(dir, entry);
      if (statSync(full).isDirectory()) {
        if (entry === "__tests__") {
          continue;
        }
        walk(full);
      } else if (entry.endsWith(".ts")) {
        out.push(full);
      }
    }
  };
  walk(srcDir);
  return out;
}

export interface SourceAuditResult {
  readonly files: string[];
  readonly violations: DeterminismViolation[];
}

export function auditAllWorkflowSources(
  packageRoot: string,
): SourceAuditResult {
  const files = workflowSourceFiles(packageRoot);
  const violations: DeterminismViolation[] = [];
  for (const file of files) {
    const source = readFileSync(file, "utf8");
    violations.push(...findDeterminismViolations(source));
  }
  return { files, violations };
}

/** Extract import/export specifiers from a TS module. */
export function importSpecifiers(source: string): string[] {
  const specifiers: string[] = [];
  const re = /(?:import|export)\s+(?:[^'"]*?\s+from\s+)?['"]([^'"]+)['"]/g;
  let match: RegExpExecArray | null;
  while ((match = re.exec(source)) !== null) {
    const specifier = match[1];
    if (specifier !== undefined) {
      specifiers.push(specifier);
    }
  }
  return specifiers;
}
