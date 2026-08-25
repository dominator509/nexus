/**
 * EP-042 M2 dependency-direction and honesty proofs.
 *
 * The update core must remain pure: no node builtins, no filesystem, no
 * network, no process, no docker, no provider SDK imports in
 * apps/setup/src/update. It adapts canonical contracts only.
 */

import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const UPDATE_DIR = join(
  __dirname,
  "..",
  "..",
  "..",
  "..",
  "apps",
  "setup",
  "src",
  "update",
);

const FORBIDDEN_IMPORTS: ReadonlyArray<RegExp> = [
  /from ["']node:fs["']/,
  /from ["']node:child_process["']/,
  /from ["']node:net["']/,
  /from ["']node:http["']/,
  /from ["']node:https["']/,
  /from ["']node:process["']/,
  /require\(["']fs["']\)/,
  /require\(["']child_process["']\)/,
  /require\(["']net["']\)/,
  /require\(["']http["']\)/,
];

const FORBIDDEN_KEYWORDS: ReadonlyArray<RegExp> = [
  /\bdocker\b/i,
  /\bkubectl\b/i,
  /\bminio\b/,
  /\baws-sdk\b/,
  /\bazure\b/i,
  /\bgoogle-cloud\b/,
  /\bcloudflare\b/i,
  /\bopentelemetry\b/i,
  /\bprometheus\b/i,
];

function updateSourceFiles(): Array<string> {
  const files: Array<string> = [];
  const walk = (dir: string): void => {
    for (const entry of readdirSync(dir)) {
      const full = join(dir, entry);
      if (statSync(full).isDirectory()) {
        walk(full);
      } else if (full.endsWith(".ts")) {
        files.push(full);
      }
    }
  };
  walk(UPDATE_DIR);
  return files;
}

describe("ep042_unit dependency direction", () => {
  it("ep042_unit_update_core_has_no_node_builtin_imports", () => {
    const files = updateSourceFiles();
    expect(files.length).toBeGreaterThanOrEqual(10);
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      for (const pattern of FORBIDDEN_IMPORTS) {
        expect(
          source.match(pattern),
          `${file} contains forbidden import ${pattern}`,
        ).toBeNull();
      }
    }
  });

  it("ep042_unit_update_core_has_no_provider_keywords", () => {
    const files = updateSourceFiles();
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      for (const pattern of FORBIDDEN_KEYWORDS) {
        expect(
          source.match(pattern),
          `${file} contains forbidden provider keyword ${pattern}`,
        ).toBeNull();
      }
    }
  });

  it("ep042_unit_update_core_files_present", () => {
    const files = updateSourceFiles();
    const names = files.map((file) => file.split("/").pop() ?? "");
    for (const required of [
      "errors.ts",
      "types.ts",
      "validate.ts",
      "digest.ts",
      "manifest.ts",
      "compatibility.ts",
      "planner.ts",
      "backup.ts",
      "rollback.ts",
      "canary.ts",
      "evidence.ts",
      "index.ts",
    ]) {
      expect(names).toContain(required);
    }
  });

  it("ep042_unit_update_core_no_placeholder_content", () => {
    const files = updateSourceFiles();
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      expect(
        /placeholder|TODO|FIXME|not implemented|unimplemented!/i.test(source),
        `${file} contains placeholder content`,
      ).toBe(false);
    }
  });
});
