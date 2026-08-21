/**
 * EP-035 M3 unit test: dependency direction.
 *
 * The onboarding integration layer may depend on the contract package
 * (@nexus/setup) and the real transport drivers it owns (pg, nats), but
 * must never reach into concrete provider SDKs or higher-layer packages.
 * Contract semantics stay in @nexus/setup; this package owns transport.
 */

import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const PKG_ROOT = join(import.meta.dirname, "..", "..", "..");
const pkgJson = JSON.parse(
  readFileSync(join(PKG_ROOT, "package.json"), "utf8"),
) as { dependencies?: Record<string, string> };

const ALLOWED_DEPENDENCIES = new Set<string>([
  "@nexus/setup", // canonical contracts
  "nats", // real event bus client (M3-owned transport)
  "pg", // real durable store client (M3-owned transport)
]);

describe("ep035_unit_dependency_direction", () => {
  it("depends only on the contract package and owned transports", () => {
    const deps = Object.keys(pkgJson.dependencies ?? {});
    for (const dep of deps) {
      expect(
        ALLOWED_DEPENDENCIES.has(dep),
        `unexpected dependency: ${dep}`,
      ).toBe(true);
    }
    expect(deps).toContain("@nexus/setup");
    expect(deps).toContain("pg");
    expect(deps).toContain("nats");
  });

  it("never depends on concrete provider SDKs", () => {
    const deps = Object.keys(pkgJson.dependencies ?? {});
    const forbidden = [
      "dockerode",
      "@opentofu",
      "@cloudflare",
      "aws-sdk",
      "@azure",
      "@google-cloud",
      "home-assistant-js-websocket",
      "keycloak-js",
      "bluetooth",
      "@nexus/web",
      "@nexus/contracts",
    ];
    for (const dep of deps) {
      for (const f of forbidden) {
        expect(dep.startsWith(f), `forbidden provider dependency: ${dep}`).toBe(
          false,
        );
      }
    }
  });
});
