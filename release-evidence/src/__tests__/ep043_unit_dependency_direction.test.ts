/**
 * EP-043 dependency-direction proof (M1 pure contract + M2 adapter
 * boundary).
 *
 * release-evidence/ is a provider-neutral ship contract package. Pure
 * domain modules (errors, model, readiness, manifest, report) must not
 * import node builtins, provider SDKs, or any other workspace package.
 * I/O adapter modules (repo-state, cli) may import node builtins for
 * real repository I/O but must never import provider SDKs or other
 * workspace packages. The ship boundary stays provider-neutral
 * (SPEC-008).
 */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..", "src");

/** Pure domain modules: no node builtins, no provider SDKs. */
const PURE_MODULES = [
  "errors.ts",
  "model.ts",
  "readiness.ts",
  "manifest.ts",
  "report.ts",
];

/** Adapter modules: node allowed, provider SDKs forbidden. */
const ADAPTER_MODULES = ["repo-state.ts", "cli.ts", "evidence.ts", "index.ts"];

const FORBIDDEN_PROVIDER_IMPORTS = [
  "@nexus/",
  "aws-",
  "@aws-",
  "minio",
  "seaweedfs",
  "openai",
  "anthropic",
  "temporal",
  "keycloak",
  "pg",
  "redis",
  "docker",
];

function collectTsFiles(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      out.push(...collectTsFiles(full));
    } else if (entry.endsWith(".ts") && !entry.endsWith(".test.ts")) {
      out.push(full);
    }
  }
  return out;
}

function importLines(content: string): string[] {
  return content
    .split("\n")
    .filter((line) => line.includes('from "') || line.includes("from '"));
}

describe("EP-043 dependency direction", () => {
  it("ep043_unit_dependency_direction_pure_modules_have_no_foreign_imports", () => {
    for (const moduleName of PURE_MODULES) {
      const file = join(ROOT, moduleName);
      const content = readFileSync(file, "utf8");
      for (const line of importLines(content)) {
        expect(line, `${moduleName}: ${line}`).not.toMatch(
          /from "(node:|@nexus\/)/,
        );
        for (const forbidden of FORBIDDEN_PROVIDER_IMPORTS) {
          expect(line, `${moduleName}: ${line}`).not.toContain(forbidden);
        }
      }
    }
  });

  it("ep043_unit_dependency_direction_adapters_have_no_provider_imports", () => {
    for (const moduleName of ADAPTER_MODULES) {
      const file = join(ROOT, moduleName);
      const content = readFileSync(file, "utf8");
      for (const line of importLines(content)) {
        for (const forbidden of FORBIDDEN_PROVIDER_IMPORTS) {
          expect(line, `${moduleName}: ${line}`).not.toContain(forbidden);
        }
      }
    }
  });

  it("ep043_unit_dependency_direction_every_module_classified", () => {
    const files = collectTsFiles(ROOT);
    expect(files.length).toBeGreaterThan(0);
    const known = new Set([...PURE_MODULES, ...ADAPTER_MODULES]);
    for (const file of files) {
      const name = file.split("/").pop()!;
      expect(known.has(name), `${file} not classified`).toBe(true);
    }
  });

  it("ep043_unit_dependency_direction_no_placeholder", () => {
    const files = collectTsFiles(ROOT);
    const placeholders = [
      "TODO",
      "FIXME",
      "XXX placeholder",
      "not implemented",
      "demo mode",
      "sample success",
    ];
    for (const file of files) {
      const content = readFileSync(file, "utf8");
      for (const placeholder of placeholders) {
        expect(content, `${file} contains ${placeholder}`).not.toContain(
          placeholder,
        );
      }
    }
  });

  it("ep043_unit_dependency_direction_no_provider_keywords", () => {
    const files = collectTsFiles(ROOT);
    const providerKeywords = [
      "SeaweedFS",
      "MinIO",
      "S3Client",
      "createBucket",
      "putObject",
      "OpenAI",
      "Anthropic",
      "Temporal",
      "Keycloak",
    ];
    for (const file of files) {
      const content = readFileSync(file, "utf8");
      for (const keyword of providerKeywords) {
        expect(file, `${file} contains ${keyword}`).not.toContain(keyword);
      }
    }
  });
});
