/**
 * EP-043 M2 production readiness CLI (SPEC-008).
 *
 * Real commands:
 *   readiness  collect real repository state, evaluate acceptance
 *              obligations, write PRODUCTION_READINESS.md
 *   manifest   build dist/release/RELEASE_MANIFEST.json from real
 *              component bytes with real sha256 digests
 *
 * Runs under Node 24 native TS with the resolution-only ESM loader.
 */

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { collectReadinessInputs, defaultRepoPaths } from "./repo-state.ts";
import { evaluateReadiness, validateReadinessInputs } from "./readiness.ts";
import { renderProductionReadinessReport } from "./report.ts";
import {
  buildReleaseManifest,
  type ManifestComponentInput,
} from "./manifest.ts";
import { redactShipMessage } from "./errors.ts";

const [command, ...args] = process.argv.slice(2);

function flag(name: string): string | undefined {
  const index = args.indexOf(`--${name}`);
  if (index === -1 || index + 1 >= args.length) return undefined;
  return args[index + 1];
}

function requireFlag(name: string): string {
  const value = flag(name);
  if (!value) {
    throw new Error(`missing required flag --${name}`);
  }
  return value;
}

function runId(prefix: string): string {
  return `${prefix}-${Date.now()}`;
}

function gitCommit(): string {
  try {
    const head = readFileSync(
      join(process.cwd(), ".git", "HEAD"),
      "utf8",
    ).trim();
    if (head.startsWith("ref:")) {
      const refPath = head.slice(5).trim();
      return readFileSync(join(process.cwd(), ".git", refPath), "utf8")
        .trim()
        .slice(0, 40);
    }
    return head.slice(0, 40);
  } catch {
    return "unknown";
  }
}

async function commandReadiness(): Promise<void> {
  const output = flag("output") ?? "PRODUCTION_READINESS.md";
  const root = process.cwd();
  const paths = defaultRepoPaths(root);
  const inputs = collectReadinessInputs(paths);
  validateReadinessInputs(inputs);
  const evaluation = evaluateReadiness(inputs);
  const report = renderProductionReadinessReport(evaluation, {
    node: "EP-043",
    runId: runId("ep043-readiness"),
    gitCommit: gitCommit(),
    generatedAt: new Date().toISOString(),
  });
  const safe = redactShipMessage(report);
  writeFileSync(join(root, output), safe, "utf8");
  // eslint-disable-next-line no-console
  console.log(
    `readiness: ${evaluation.decision} (${evaluation.blockingReasons.length} blocking reasons)`,
  );
  // eslint-disable-next-line no-console
  console.log(`wrote ${output} (${safe.length} bytes)`);
}

async function commandManifest(): Promise<void> {
  const outputDir = flag("output-dir") ?? "dist/release";
  const releaseId = flag("release-id") ?? "nexus-1.0.0-rc1";
  const root = process.cwd();
  const outPath = join(root, outputDir);
  mkdirSync(outPath, { recursive: true });

  const componentInputs: ManifestComponentInput[] = [
    {
      componentId: "nexus-core",
      name: "nexus-core",
      version: "1.0.0",
      artifactBytes: new Uint8Array(
        readFileSync(
          join(
            root,
            "infra",
            "release",
            "fixtures",
            "components",
            "nexus-core",
          ),
        ),
      ),
      artifactKey: `releases/${releaseId}/components/nexus-core`,
    },
    {
      componentId: "nexus-model",
      name: "nexus-model",
      version: "1.0.0",
      artifactBytes: new Uint8Array(
        readFileSync(
          join(
            root,
            "infra",
            "release",
            "fixtures",
            "components",
            "nexus-model",
          ),
        ),
      ),
      artifactKey: `releases/${releaseId}/components/nexus-model`,
    },
  ];

  const manifest = buildReleaseManifest({
    releaseId,
    version: "1.0.0",
    channel: "STABLE",
    profile: "FULLY_LOCAL",
    createdAt: new Date().toISOString(),
    components: componentInputs,
  });

  writeFileSync(
    join(outPath, "RELEASE_MANIFEST.json"),
    JSON.stringify(manifest, null, 2),
    "utf8",
  );
  // eslint-disable-next-line no-console
  console.log(
    `manifest: wrote ${join(outputDir, "RELEASE_MANIFEST.json")} digest=${manifest.manifest_digest}`,
  );
  // eslint-disable-next-line no-console
  console.log(
    `manifest: ${manifest.components.length} components, signatures PRESENT_NOT_VERIFIED`,
  );
}

async function main(): Promise<void> {
  switch (command) {
    case "readiness":
      await commandReadiness();
      break;
    case "manifest":
      await commandManifest();
      break;
    default:
      // eslint-disable-next-line no-console
      console.error(
        "usage: release-evidence-cli <readiness|manifest> [--output FILE] [--output-dir DIR]",
      );
      process.exitCode = 2;
  }
}

void main();
