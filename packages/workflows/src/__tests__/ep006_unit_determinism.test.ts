import { describe, expect, it } from "vitest";

import { findDeterminismViolations, formatViolations } from "../determinism.js";
import {
  auditAllWorkflowSources,
  workflowsPackageRoot,
} from "./helpers/fixtures.js";

describe("ep006_unit_determinism", () => {
  it("ep006_unit_determinism_guard_scans_real_sources", () => {
    // The guard must genuinely inspect the workflow contract source, never
    // pass vacuously on an empty tree.
    const { files, violations } = auditAllWorkflowSources(workflowsPackageRoot);
    expect(files.length).toBeGreaterThan(0);
    expect(violations).toEqual([]);
  });

  it("ep006_unit_determinism_detects_wall_clock", () => {
    const violations = findDeterminismViolations("const now = Date.now();");
    expect(violations.length).toBeGreaterThan(0);
    expect(violations[0]?.reason).toMatch(/wall clock/);
  });

  it("ep006_unit_determinism_detects_date_constructor", () => {
    expect(
      findDeterminismViolations("const d = new Date();").length,
    ).toBeGreaterThan(0);
  });

  it("ep006_unit_determinism_detects_random", () => {
    expect(
      findDeterminismViolations("const r = Math.random();").length,
    ).toBeGreaterThan(0);
    expect(
      findDeterminismViolations("const id = crypto.randomUUID();").length,
    ).toBeGreaterThan(0);
  });

  it("ep006_unit_determinism_detects_network", () => {
    expect(
      findDeterminismViolations("await fetch('/api');").length,
    ).toBeGreaterThan(0);
    expect(
      findDeterminismViolations("const ws = new WebSocket(url);").length,
    ).toBeGreaterThan(0);
  });

  it("ep006_unit_determinism_detects_database_and_env", () => {
    expect(
      findDeterminismViolations("import pg from 'pg';").length,
    ).toBeGreaterThan(0);
    expect(
      findDeterminismViolations("const p = new Pool();").length,
    ).toBeGreaterThan(0);
    expect(
      findDeterminismViolations("const x = process.env.X;").length,
    ).toBeGreaterThan(0);
    expect(
      findDeterminismViolations("const g = globalThis;").length,
    ).toBeGreaterThan(0);
  });

  it("ep006_unit_determinism_clean_source_passes", () => {
    const clean = `
      import { WorkflowContractError } from "../errors.js";
      export function pure(x: number): number {
        return x * 2;
      }
    `;
    expect(findDeterminismViolations(clean)).toEqual([]);
  });

  it("ep006_unit_determinism_format_violations", () => {
    const violations = findDeterminismViolations("Date.now()");
    const formatted = formatViolations(violations);
    expect(formatted).toContain("line 1");
    expect(formatViolations([])).toBe("no determinism violations");
  });
});
