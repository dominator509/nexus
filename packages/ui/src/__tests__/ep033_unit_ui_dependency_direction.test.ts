/**
 * EP-033 M3 UI package dependency-direction test (directive W).
 *
 * packages/ui is the shared UI contract package: the web app and the
 * desktop shell depend on it, and it must never depend on concrete
 * web/desktop implementations or backend clients. Its only external
 * imports are React (the selected open-source component), the
 * generated canonical bindings, and the @nexus/web contract package.
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
    } else if (name.endsWith(".ts") || name.endsWith(".tsx")) {
      out.push(full);
    }
  }
  return out;
}

describe("ep033_unit_ui_dependency_direction", () => {
  const files = collectTsFiles(SRC_DIR).filter((f) => !f.includes("__tests__"));

  it("finds the UI production sources (vacuity guard)", () => {
    expect(files.length).toBeGreaterThan(0);
  });

  it("UI sources import only react, react-dom, @nexus/web, @nexus/contracts, or relative modules", () => {
    const allowed = new Set(["react", "react-dom", "@nexus/web", "@nexus/contracts", "react/jsx-runtime"]);
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
          allowed.has(specifier),
          `${file} imports disallowed package '${specifier}'`,
        ).toBe(true);
      }
    }
  });

  it("UI package never imports web/desktop app code or backend clients", () => {
    const forbidden = [
      "@nexus/control-plane",
      "axios",
      "fetch",
      "ws",
      "socket.io",
      "next",
      "vite",
      "@tauri-apps",
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
