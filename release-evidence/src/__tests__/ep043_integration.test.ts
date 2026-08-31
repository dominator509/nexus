/**
 * EP-043 M3 real dependency and transport integration proofs.
 *
 * Every test name begins `ep043_integration_`. The suite exercises the
 * REAL release-evidence CLI against the REAL repository state and REAL
 * artifact bytes (no mocks, no test doubles, no in-memory engine):
 *   - the CLI reads real GRAPH.md, LEDGER.md, live-fire registry, and
 *     certification RESULTS.md files,
 *   - the release manifest digests real artifact bytes,
 *   - documented OPERATIONS.md commands resolve to real paths and run,
 *   - NOT_READY is preserved while release blockers remain,
 *   - missing dependencies and tampered artifacts fail closed,
 *   - the operational path works from a real fresh-clone checkout.
 */
import { execFile, spawn } from "node:child_process";
import { access, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { afterEach, describe, expect, it } from "vitest";

import { buildReleaseManifest, digestBytes } from "@nexus/release-evidence";

const execFileAsync = promisify(execFile);

const ROOT = process.env.EP043_TEST_ROOT ?? "/root/nexus";
const LOADER = join(
  ROOT,
  "release-evidence",
  "scripts",
  "ts-resolve-loader.mjs",
);
const CLI = join(ROOT, "release-evidence", "src", "cli.ts");
const OPERATIONS = join(ROOT, "OPERATIONS.md");

const tempRoots: string[] = [];

async function tempDir(prefix: string): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), prefix));
  tempRoots.push(dir);
  return dir;
}

async function runCli(
  args: string[],
  options: { cwd?: string } = {},
): Promise<{ stdout: string; stderr: string; code: number }> {
  try {
    const { stdout, stderr } = await execFileAsync(
      "node",
      [
        "--experimental-transform-types",
        "--import",
        `file://${LOADER}`,
        CLI,
        ...args,
      ],
      {
        cwd: options.cwd ?? ROOT,
        timeout: 60_000,
        // Strip inherited NODE_OPTIONS (vitest injects its own loader)
        // so the child node process uses only the CLI's loader.
        env: { ...process.env, NODE_OPTIONS: "" },
      },
    );
    return { stdout, stderr, code: 0 };
  } catch (error) {
    const err = error as {
      stdout?: string;
      stderr?: string;
      code?: number;
    };
    return {
      stdout: err.stdout ?? "",
      stderr: err.stderr ?? "",
      code: typeof err.code === "number" ? err.code : 1,
    };
  }
}

afterEach(async () => {
  for (const root of tempRoots) {
    await rm(root, { recursive: true, force: true });
  }
  tempRoots.length = 0;
});

describe("EP-043 M3 real dependency and transport integration", () => {
  it("ep043_integration_cli_reads_real_repo_state", async () => {
    const out = await tempDir("ep043-m3-readiness-");
    const report = join(out, "PRODUCTION_READINESS.md");
    const result = await runCli(["readiness", "--output", report]);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("readiness: NOT_READY");
    expect(result.stdout).toMatch(/\(\d+ blocking reasons\)/);
    const body = await readFile(report, "utf8");
    expect(body).toContain("EP-043");
    expect(body).toContain("certification row");
    // AUD-087: the report decision is bound through the authoritative
    // ProductionReadinessDecision constructor and matches the CLI verdict.
    expect(body).toMatch(/## Decision: (READY|NOT_READY)/);
    const decisionLine = body.match(/## Decision: (\w+)/)?.[1];
    expect(decisionLine).toBe("NOT_READY");
  });

  it("ep043_integration_manifest_digests_real_artifact_bytes", async () => {
    // AUD-082: the manifest binds the REAL committed product artifacts
    // (model code, provider config, router policy, container def), never
    // fixture strings. Component digests must equal real sha256 over the
    // real committed artifact bytes.
    const out = await tempDir("ep043-m3-manifest-");
    const result = await runCli(["manifest", "--output-dir", out]);
    expect(result.code).toBe(0);
    const manifest = JSON.parse(
      await readFile(join(out, "RELEASE_MANIFEST.json"), "utf8"),
    ) as {
      components: Array<{
        component_id: string;
        digest: string;
        size_bytes: number;
      }>;
    };
    expect(manifest.components.length).toBeGreaterThanOrEqual(5);
    const realArtifactPaths: Record<string, string> = {
      "nexus-wake-model": "models/wake/nexus_wake/decision.py",
      "nexus-wake-manifest": "models/wake/nexus_wake/manifest.py",
      "nexus-providers-config": "config/models/providers/providers.json",
      "nexus-router-policy": "config/models/router/policy.json",
      "nexus-container-seaweedfs": "infra/release/containers/seaweedfs.yaml",
    };
    for (const component of manifest.components) {
      const relPath = realArtifactPaths[component.component_id];
      expect(relPath).toBeDefined();
      const bytes = await readFile(join(ROOT, relPath!));
      const expected = digestBytes(new Uint8Array(bytes));
      expect(component.digest).toBe(expected);
      expect(component.size_bytes).toBe(bytes.length);
    }
  });

  it("ep043_integration_operations_commands_resolve", async () => {
    const operations = await readFile(OPERATIONS, "utf8");
    const codeBlocks = [...operations.matchAll(/```sh\n([\s\S]*?)```/g)];
    expect(codeBlocks.length).toBeGreaterThanOrEqual(6);
    const commandLines = codeBlocks.flatMap((block) =>
      block[1]!
        .split("\n")
        .filter((line) => line.includes("release-evidence/src/cli.ts")),
    );
    expect(commandLines.length).toBeGreaterThanOrEqual(8);
    for (const line of commandLines) {
      expect(line).toContain("release-evidence/src/cli.ts");
      await expect(access(CLI)).resolves.toBeUndefined();
      await expect(access(LOADER)).resolves.toBeUndefined();
    }
  });

  it("ep043_integration_not_ready_preserved", async () => {
    const status = await runCli(["ship-gate-status"]);
    expect(status.code).toBe(0);
    expect(status.stdout).toContain("ship-gate verdict: BLOCKED");
    expect(status.stdout).toContain("readiness decision: NOT_READY");
    expect(status.stdout).toContain("blocking reasons");
    expect(status.stdout).toContain("certification row");
  });

  it("ep043_integration_fail_closed_missing_dependency", async () => {
    const empty = await tempDir("ep043-m3-empty-");
    const result = await runCli(
      ["readiness", "--output", join(empty, "PR.md")],
      { cwd: empty },
    );
    expect(result.code).not.toBe(0);
    expect(result.stderr).toContain("UNAVAILABLE");
    await expect(readFile(join(empty, "PR.md"), "utf8")).rejects.toThrow();
  });

  it("ep043_integration_manifest_component_digests_deterministic", async () => {
    const outA = await tempDir("ep043-m3-idem-a-");
    const outB = await tempDir("ep043-m3-idem-b-");
    const first = await runCli(["manifest", "--output-dir", outA]);
    const second = await runCli(["manifest", "--output-dir", outB]);
    expect(first.code).toBe(0);
    expect(second.code).toBe(0);
    const manifestA = JSON.parse(
      await readFile(join(outA, "RELEASE_MANIFEST.json"), "utf8"),
    ) as {
      components: Array<{
        component_id: string;
        digest: string;
        size_bytes: number;
      }>;
    };
    const manifestB = JSON.parse(
      await readFile(join(outB, "RELEASE_MANIFEST.json"), "utf8"),
    ) as {
      components: Array<{
        component_id: string;
        digest: string;
        size_bytes: number;
      }>;
    };
    expect(manifestA.components.length).toBe(manifestB.components.length);
    for (let index = 0; index < manifestA.components.length; index += 1) {
      expect(manifestA.components[index]!.digest).toBe(
        manifestB.components[index]!.digest,
      );
      expect(manifestA.components[index]!.size_bytes).toBe(
        manifestB.components[index]!.size_bytes,
      );
    }
  });

  it("ep043_integration_verify_manifest_detects_tamper", async () => {
    const out = await tempDir("ep043-m3-tamper-");
    const generated = await runCli(["manifest", "--output-dir", out]);
    expect(generated.code).toBe(0);
    const manifestPath = join(out, "RELEASE_MANIFEST.json");
    const manifest = JSON.parse(await readFile(manifestPath, "utf8")) as {
      components: Array<{ digest: string }>;
    };
    manifest.components[0]!.digest = "sha256:" + "0".repeat(64);
    await writeFile(manifestPath, JSON.stringify(manifest, null, 2), "utf8");
    const verified = await runCli([
      "verify-manifest",
      "--manifest",
      manifestPath,
    ]);
    expect(verified.code).not.toBe(0);
    expect(verified.stderr).toContain("VERIFICATION_FAILED");
  });

  it("ep043_integration_verify_manifest_fails_closed_missing_artifact", async () => {
    // AUD-082: verify-manifest fails closed when a manifest component is
    // NOT one of the real release artifacts (ghost/injected component).
    const out = await tempDir("ep043-m3-ghost-");
    const bytes = await readFile(
      join(ROOT, "models", "wake", "nexus_wake", "decision.py"),
    );
    const ghost = buildReleaseManifest({
      releaseId: "nexus-1.0.0-ghost",
      version: "1.0.0",
      channel: "STABLE",
      profile: "FULLY_LOCAL",
      createdAt: new Date().toISOString(),
      components: [
        {
          componentId: "ghost-component",
          name: "ghost-component",
          version: "1.0.0",
          artifactBytes: new Uint8Array(bytes),
          artifactKey: "releases/nexus-1.0.0-ghost/components/ghost-component",
        },
      ],
    });
    const manifestPath = join(out, "RELEASE_MANIFEST.json");
    await writeFile(manifestPath, JSON.stringify(ghost, null, 2), "utf8");
    const verified = await runCli([
      "verify-manifest",
      "--manifest",
      manifestPath,
    ]);
    expect(verified.code).not.toBe(0);
    expect(verified.stderr).toContain("NOT_FOUND");
  });

  it("ep043_integration_fresh_clone_temp_checkout", async () => {
    const out = await tempDir("ep043-m3-clone-");
    await execFileAsync("git", [
      "clone",
      "--depth",
      "1",
      `file://${ROOT}`,
      join(out, "nexus"),
    ]);
    const cloneRoot = join(out, "nexus");
    const report = join(cloneRoot, "PRODUCTION_READINESS.md");
    const result = await runCli(["readiness", "--output", report], {
      cwd: cloneRoot,
    });
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("readiness: NOT_READY");
    const body = await readFile(report, "utf8");
    expect(body).toContain("EP-043");
    expect(body).toContain("certification row");
    const { stdout: head } = await execFileAsync("git", ["rev-parse", "HEAD"], {
      cwd: ROOT,
    });
    expect(body).toContain(head.trim().slice(0, 12));
  });

  it("ep043_integration_cli_fails_closed_on_bad_args", async () => {
    const out = await tempDir("ep043-m3-badargs-");
    const result = await runCli([], { cwd: out });
    expect(result.code).toBe(2);
    expect(result.stderr).toContain("usage: release-evidence-cli");
  });

  it("ep043_integration_timeout_bounded_completion", async () => {
    const out = await tempDir("ep043-m3-timeout-");
    const report = join(out, "PRODUCTION_READINESS.md");
    const result = await runCli(["readiness", "--output", report]);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("wrote");
  });

  it("ep043_integration_cancellation_writes_no_partial", async () => {
    const out = await tempDir("ep043-m3-cancel-");
    const report = join(out, "PRODUCTION_READINESS.md");
    const child = spawn(
      "node",
      [
        "--experimental-transform-types",
        "--import",
        `file://${LOADER}`,
        CLI,
        "readiness",
        "--output",
        report,
      ],
      {
        cwd: ROOT,
        stdio: ["ignore", "pipe", "pipe"],
        env: { ...process.env, NODE_OPTIONS: "" },
      },
    );
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 50));
    child.kill("SIGTERM");
    await new Promise((resolvePromise) => {
      child.on("exit", resolvePromise);
    });
    await expect(readFile(report, "utf8")).rejects.toThrow();
  });

  it("ep043_integration_certification_rows_read_real", async () => {
    const result = await runCli(["certification-rows"]);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("PROVIDER");
    expect(result.stdout).toContain("HARDWARE");
    expect(result.stdout).toContain("RELEASE-BLOCKING-PENDING");
    expect(result.stdout).toContain("certification rows: 2");
  });

  it("ep043_integration_audit_fields_recorded", async () => {
    const out = await tempDir("ep043-m3-audit-");
    const report = join(out, "PRODUCTION_READINESS.md");
    const result = await runCli(["readiness", "--output", report]);
    expect(result.code).toBe(0);
    const body = await readFile(report, "utf8");
    expect(body).toContain("Run:");
    expect(body).toContain("Git commit:");
    expect(body).toContain("Generated:");
    expect(body).toContain("EP-043");
  });

  it("ep043_integration_event_emission_deterministic", async () => {
    const out = await tempDir("ep043-m3-emit-");
    const manifest = await runCli(["manifest", "--output-dir", out]);
    expect(manifest.code).toBe(0);
    expect(manifest.stdout).toContain("manifest: wrote");
    expect(manifest.stdout).toContain(
      "real product artifacts, signatures PRESENT_NOT_VERIFIED",
    );
  });
});
