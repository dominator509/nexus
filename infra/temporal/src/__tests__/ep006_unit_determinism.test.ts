import { describe, expect, it } from "vitest";

import { readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { findDeterminismViolations } from "@nexus/workflows";

/**
 * Determinism audit for the Temporal workflow code (SPEC-023 behavior 6;
 * EP-006 hard invariant: no host-clock, random-source, network, or
 * database calls inside workflow code).
 *
 * Scans infra/temporal/src/workflows and src/state (the code that runs
 * inside the workflow isolate). The engine bridge (src/context.ts) is the
 * one place that reads the isolate-patched clock and is intentionally not
 * workflow code.
 */

const packageRoot = fileURLToPath(new URL("../../", import.meta.url));

function workflowSourceFiles(): string[] {
  const roots = ["src/workflows", "src/state"];
  const out: string[] = [];
  for (const root of roots) {
    const dir = path.join(packageRoot, root);
    const walk = (d: string): void => {
      for (const entry of readdirRecursive(d)) {
        if (entry.endsWith(".ts")) {
          out.push(entry);
        }
      }
    };
    walk(dir);
  }
  return out;
}

function readdirRecursive(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = path.join(dir, entry);
    if (statSync(full).isDirectory()) {
      if (entry === "__tests__") {
        continue;
      }
      out.push(...readdirRecursive(full));
    } else {
      out.push(full);
    }
  }
  return out;
}

describe("ep006_unit_determinism", () => {
  it("ep006_unit_determinism_workflow_code_clean", () => {
    const files = workflowSourceFiles();
    expect(files.length).toBeGreaterThan(0);
    const violations: string[] = [];
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      const found = findDeterminismViolations(source);
      for (const v of found) {
        violations.push(
          `${path.relative(packageRoot, file)}:${v.line}: ${v.reason}`,
        );
      }
    }
    expect(violations).toEqual([]);
  });

  it("ep006_unit_determinism_scans_workflow_and_state_roots", () => {
    const files = workflowSourceFiles();
    expect(
      files.some((f) => f.includes(`${path.sep}workflows${path.sep}`)),
    ).toBe(true);
    expect(files.some((f) => f.includes(`${path.sep}state${path.sep}`))).toBe(
      true,
    );
  });
});
