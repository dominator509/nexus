/**
 * EP-035 M1 dependency-direction test (anti-drift).
 *
 * The setup contract package must remain provider-neutral and reusable
 * across future UI/runtime implementations. It may depend only on
 * canonical contracts (@nexus/contracts) plus relative imports, and it
 * must never import concrete provider, UI framework, or backend-client
 * packages.
 */

import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const PACKAGE_ROOT = process.cwd();
const SRC_ROOT = join(PACKAGE_ROOT, "src");

function listSourceFiles(dir: string): Array<string> {
  const out: Array<string> = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      out.push(...listSourceFiles(full));
    } else if (entry.name.endsWith(".ts")) {
      out.push(full);
    }
  }
  return out;
}

const FORBIDDEN_IMPORT_PREFIXES = [
  "@nexus/web",
  "@nexus/desktop",
  "@nexus/ui",
  "@nexus/mobile",
  "@nexus/mobile-contracts",
  "react",
  "react-dom",
  "@tauri-apps",
  "axios",
  "node-fetch",
  "@temporalio",
];

describe("ep035_unit_dependency_direction", () => {
  it("declares only canonical workspace dependencies", () => {
    const pkg = JSON.parse(
      readFileSync(join(PACKAGE_ROOT, "package.json"), "utf8"),
    ) as { dependencies?: Record<string, string> };
    const deps = pkg.dependencies ?? {};
    expect(Object.keys(deps).sort()).toEqual(["@nexus/contracts"]);
  });

  it("src never imports provider, UI framework, or backend clients", () => {
    const files = listSourceFiles(SRC_ROOT);
    expect(files.length).toBeGreaterThan(0);
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      for (const line of source.split("\n")) {
        const trimmed = line.trim();
        if (!trimmed.startsWith("import ")) {
          continue;
        }
        for (const forbidden of FORBIDDEN_IMPORT_PREFIXES) {
          expect(
            trimmed,
            `${file} must not import '${forbidden}'`,
          ).not.toContain(`"${forbidden}`);
          expect(
            trimmed,
            `${file} must not import '${forbidden}'`,
          ).not.toContain(`'${forbidden}`);
        }
      }
    }
  });

  it("all contract imports are relative or @nexus/contracts", () => {
    // Scope: src/contracts only. Test files legitimately import vitest.
    const files = listSourceFiles(SRC_ROOT).filter((file) =>
      file.includes(`${join("src", "contracts")}`),
    );
    expect(files.length).toBeGreaterThan(0);
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      for (const line of source.split("\n")) {
        const trimmed = line.trim();
        if (!trimmed.startsWith("import ")) {
          continue;
        }
        const match = /from\s+["']([^"']+)["']/.exec(trimmed);
        if (match === null) {
          continue;
        }
        const specifier = match[1] ?? "";
        if (specifier.startsWith(".")) {
          continue;
        }
        if (specifier === "@nexus/contracts") {
          continue;
        }
        expect(
          `${file}: import '${specifier}' is not a relative import or @nexus/contracts`,
        ).toBe("");
      }
    }
  });
});
