import { describe, expect, it } from "vitest";

import { readFileSync } from "node:fs";
import path from "node:path";

import {
  importSpecifiers,
  workflowSourceFiles,
  workflowsPackageRoot,
} from "./helpers/fixtures.js";

/**
 * Dependency-direction constraints (EP-006 M1 doctrine, ADR-010):
 * - @nexus/workflows is engine-neutral: it must never import a Temporal
 *   SDK or any provider engine.
 * - It may import only relative modules and the canonical generated
 *   contracts package @nexus/contracts.
 * - It must never reach into infra/ or tests/.
 */

function productionSources(): string[] {
  return workflowSourceFiles(workflowsPackageRoot);
}

describe("ep006_unit_dependency_direction", () => {
  it("ep006_unit_dependency_direction_no_engine_imports", () => {
    const files = productionSources();
    expect(files.length).toBeGreaterThan(0);
    const offenders: string[] = [];
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      for (const specifier of importSpecifiers(source)) {
        if (specifier.includes("@temporalio")) {
          offenders.push(`${path.basename(file)} -> ${specifier}`);
        }
      }
    }
    expect(offenders).toEqual([]);
  });

  it("ep006_unit_dependency_direction_no_node_builtins", () => {
    const files = productionSources();
    const offenders: string[] = [];
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      for (const specifier of importSpecifiers(source)) {
        if (/^node:/.test(specifier)) {
          offenders.push(`${path.basename(file)} -> ${specifier}`);
        }
      }
    }
    expect(offenders).toEqual([]);
  });

  it("ep006_unit_dependency_direction_relative_and_contracts_only", () => {
    const files = productionSources();
    const offenders: string[] = [];
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      for (const specifier of importSpecifiers(source)) {
        if (specifier.startsWith(".")) {
          continue;
        }
        if (specifier === "@nexus/contracts") {
          continue;
        }
        offenders.push(`${path.basename(file)} -> ${specifier}`);
      }
    }
    expect(offenders).toEqual([]);
  });

  it("ep006_unit_dependency_direction_never_imports_infra_or_tests", () => {
    const files = productionSources();
    const offenders: string[] = [];
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      for (const specifier of importSpecifiers(source)) {
        if (
          specifier.includes("infra/") ||
          specifier.includes("tests/") ||
          specifier.includes("../infra") ||
          specifier.includes("../tests")
        ) {
          offenders.push(`${path.basename(file)} -> ${specifier}`);
        }
      }
    }
    expect(offenders).toEqual([]);
  });
});
