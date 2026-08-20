/**
 * EP-033 M2 desktop dependency-direction test.
 *
 * The desktop shell imports the shared @nexus/web contracts and the
 * generated canonical bindings. It must never import React/DOM,
 * backend clients, or re-implement vocabulary (acceptance obligation
 * 2: PWA and Tauri share contracts without duplicating business
 * logic).
 */

import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const SRC_DIR = join(process.cwd(), "src");

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

describe("ep033_unit_desktop_dependency_direction", () => {
  const files = collectTsFiles(SRC_DIR).filter((f) => !f.includes("__tests__"));

  it("finds the desktop production sources (vacuity guard)", () => {
    expect(files.length).toBeGreaterThan(0);
  });

  it("desktop sources import only @nexus/web, @nexus/contracts, or relative modules", () => {
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      for (const line of source.split("\n")) {
        const match = /^\s*import\s+(?:type\s+)?.*?from\s+["']([^"']+)["']/.exec(line);
        if (!match || match[1] === undefined) {
          continue;
        }
        const specifier = match[1] as string;
        if (specifier.startsWith(".")) {
          continue;
        }
        expect(
          specifier,
          `${file} imports disallowed package '${specifier}'`,
        ).toMatch(/^@nexus\/(web|contracts)$/);
      }
    }
  });

  it("desktop never imports React, DOM, or backend clients", () => {
    const forbidden = [
      "react",
      "react-dom",
      "react/jsx-runtime",
      "next",
      "vite",
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
          source.includes(`from "${specifier}"`) || source.includes(`from '${specifier}'`),
          `${file} must not import '${specifier}'`,
        ).toBe(false);
      }
    }
  });
});
