/**
 * EP-043 M1 dependency-direction proof.
 *
 * release-evidence/ is a pure contract package: production sources must
 * not import node builtins, provider SDKs, or any other workspace
 * package. The ship boundary stays provider-neutral (SPEC-008).
 */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..", "src");
const FORBIDDEN_IMPORTS = [
  "node:",
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

describe("EP-043 M1 dependency direction", () => {
  it("ep043_unit_dependency_direction_no_foreign_imports", () => {
    const files = collectTsFiles(ROOT);
    expect(files.length).toBeGreaterThan(0);
    for (const file of files) {
      const content = readFileSync(file, "utf8");
      const lines = content
        .split("\n")
        .filter((line) => line.includes('from "') || line.includes("from '"));
      for (const line of lines) {
        for (const forbidden of FORBIDDEN_IMPORTS) {
          expect(line, `${file}: ${line}`).not.toContain(forbidden);
        }
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
});
