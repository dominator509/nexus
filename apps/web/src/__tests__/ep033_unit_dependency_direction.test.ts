/**
 * EP-033 M1 dependency-direction test (directive W).
 *
 * The web contract layer depends only on the generated canonical
 * bindings (@nexus/contracts) and the TypeScript standard library.
 * It must never depend on React/DOM libraries, framework shells, or
 * concrete backend clients: the same contracts are shared by the
 * Tauri desktop shell (EP-033 M2), so concrete UI/transport code must
 * never leak into the contract layer.
 */

import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const CONTRACTS_DIR = join(process.cwd(), "src", "contracts");

function collectTsFiles(dir: string): Array<string> {
  const out: Array<string> = [];
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      out.push(...collectTsFiles(full));
    } else if (name.endsWith(".ts")) {
      out.push(full);
    }
  }
  return out;
}

describe("ep033_unit_dependency_direction", () => {
  const files = collectTsFiles(CONTRACTS_DIR);

  it("finds the contract source files (vacuity guard)", () => {
    expect(files.length).toBeGreaterThan(0);
  });

  it("contract layer imports only @nexus/contracts and relative modules", () => {
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      for (const line of source.split("\n")) {
        const match =
          /^\s*import\s+(?:type\s+)?.*?from\s+["']([^"']+)["']/.exec(line);
        if (!match || match[1] === undefined) {
          continue;
        }
        const specifier = match[1] as string;
        if (specifier.startsWith(".")) {
          continue; // relative import inside the contract layer
        }
        // The ONLY external package the contract layer may import is
        // the generated canonical binding package.
        expect(
          specifier,
          `${file} imports disallowed external package '${specifier}'`,
        ).toBe("@nexus/contracts");
      }
    }
  });

  it("contract layer never imports React, DOM, or backend clients", () => {
    const forbidden = [
      "react",
      "react-dom",
      "react/jsx-runtime",
      "next",
      "vite",
      "@tanstack",
      "axios",
      "fetch",
      "ws",
      "socket.io",
      "@nexus/control-plane",
    ];
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      for (const specifier of forbidden) {
        expect(
          source.includes(`from "${specifier}"`) ||
            source.includes(`from '${specifier}'`),
          `${file} must not import '${specifier}'`,
        ).toBe(false);
      }
    }
  });

  it("index re-exports the contract surface without adding dependencies", () => {
    const index = readFileSync(join(process.cwd(), "src", "index.ts"), "utf8");
    // The index re-exports only relative contract modules and never
    // imports framework or backend-client packages itself.
    expect(index).toContain('from "./contracts/');
    for (const specifier of [
      "react",
      "react-dom",
      "axios",
      "ws",
      "socket.io",
    ]) {
      expect(index).not.toContain(`from "${specifier}"`);
    }
  });
});
