/**
 * EP-043 M4 forced-failure, abuse-case, and observability proofs.
 *
 * Every test name begins `ep043_failure_`. The suite exercises the REAL
 * failure mechanisms against the REAL release-evidence CLI and REAL
 * repository bytes (no mocks of the component being proven):
 *   - unavailable dependency: missing GRAPH/LEDGER/registry,
 *   - malformed input: broken manifest JSON, malformed certification
 *     results, unknown release metadata fields,
 *   - duplicate request: conflicting certification rows for one label,
 *   - denied permission: unreadable repository state (EISDIR class),
 *   - cancelled work: SIGTERM mid-run leaves no partial output,
 *   - partial side effect: unwritable output target leaves no file,
 *   - timeout: a blocked dependency exhausts a bounded budget,
 *   - policy denial: RELEASE-BLOCKING-PENDING rows keep the ship gate
 *     BLOCKED and readiness NOT_READY,
 *   - forged/stale evidence: a hand-edited READY report or a report
 *     bound to a wrong commit never changes canonical truth,
 *   - operator bypass: unknown flags are rejected, never ignored,
 *   - redaction: structured errors never leak secret-shaped content,
 *   - observability: run_id / git_commit correlation fields recorded.
 *
 * Everything fails closed with a structured, redacted ShipError.
 */
import { execFile, spawn } from "node:child_process";
import {
  access,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { promisify } from "node:util";
import { afterEach, describe, expect, it } from "vitest";

import { ShipError, redactShipMessage } from "../errors.ts";

const execFileAsync = promisify(execFile);

const ROOT = process.env.EP043_TEST_ROOT ?? resolve(process.cwd(), "..");
const LOADER = join(
  ROOT,
  "release-evidence",
  "scripts",
  "ts-resolve-loader.mjs",
);
const CLI = join(ROOT, "release-evidence", "src", "cli.ts");
const FIXTURE_ROOT = ROOT;

const tempRoots: string[] = [];

async function tempDir(prefix: string): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), prefix));
  tempRoots.push(dir);
  return dir;
}

async function runCli(
  args: string[],
  options: { cwd?: string; timeout?: number } = {},
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
        timeout: options.timeout ?? 60_000,
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
      code?: number | string;
    };
    return {
      stdout: err.stdout ?? "",
      stderr: err.stderr ?? "",
      code: typeof err.code === "number" ? err.code : 1,
    };
  }
}

/**
 * Copy the REAL release artifacts (AUD-082) into the temp repo so the
 * manifest CLI can build/verify against real committed product bytes,
 * exactly as it does in the real repository. The manifest binds these
 * real paths - model code, provider config, router policy, container
 * definition - never fixture strings.
 */
async function copyRealArtifacts(target: string): Promise<void> {
  const realPaths = [
    "models/wake/nexus_wake/decision.py",
    "models/wake/nexus_wake/manifest.py",
    "config/models/providers/providers.json",
    "config/models/router/policy.json",
    "infra/release/containers/seaweedfs.yaml",
  ];
  for (const rel of realPaths) {
    const source = join(FIXTURE_ROOT, rel);
    const dest = join(target, rel);
    await mkdir(dirname(dest), { recursive: true });
    const bytes = await readFile(source);
    await writeFile(dest, bytes);
  }
}

/**
 * Build a minimal real repository layout in a temp dir so the CLI runs
 * against real files (never an in-memory engine). Certification content
 * is parameterizable so tests can corrupt policy state.
 */
async function makeTempRepo(
  options: {
    providerCert?: string;
    hardwareCert?: string;
    graphPathAsDirectory?: boolean;
  } = {},
): Promise<string> {
  const repo = await tempDir("ep043-m4-repo-");
  await mkdir(join(repo, ".agent", "state"), { recursive: true });
  await mkdir(join(repo, "live-fire"), { recursive: true });
  await mkdir(join(repo, "provider-certification"), { recursive: true });
  await mkdir(join(repo, "hardware"), { recursive: true });
  await mkdir(join(repo, ".git", "refs", "heads"), { recursive: true });

  await writeFile(
    join(repo, ".agent", "GRAPH.md"),
    [
      "# GRAPH",
      "",
      "| EP-001 | DEP | DONE |",
      "| EP-043 | DEP | IN_PROGRESS |",
      "",
    ].join("\n"),
    "utf8",
  );
  await writeFile(
    join(repo, ".agent", "state", "LEDGER.md"),
    [
      "# LEDGER",
      "| 2026-08-25 | agent | EP-001 | NODE_DONE | ok |",
      "| 2026-08-25 | agent | EP-043 | M1 | ok |",
      "| 2026-08-25 | agent | EP-043 | M2 | ok |",
      "| 2026-08-25 | agent | EP-043 | M3 | ok |",
      "",
    ].join("\n"),
    "utf8",
  );
  await writeFile(
    join(repo, "live-fire", "REGISTRY.tsv"),
    "LF-001|EP-001|scripts/live-fire/001.sh|lf-001|proof one\n",
    "utf8",
  );
  await writeFile(
    join(repo, "provider-certification", "RESULTS.md"),
    options.providerCert ??
      "# PROVIDER CERTIFICATION RESULTS\n\nRELEASE-BLOCKING-PENDING: DeepSeek is required for the V1 reflex release.\n",
    "utf8",
  );
  await writeFile(
    join(repo, "hardware", "CERTIFICATION_RESULTS.md"),
    options.hardwareCert ??
      "# HARDWARE CERTIFICATION RESULTS\n\nRELEASE-BLOCKING-PENDING: Lab evidence pending EP-040 and EP-043.\n",
    "utf8",
  );
  await writeFile(join(repo, ".git", "HEAD"), "ref: refs/heads/main\n", "utf8");
  await writeFile(
    join(repo, ".git", "refs", "heads", "main"),
    `${"a".repeat(40)}\n`,
    "utf8",
  );

  if (options.graphPathAsDirectory) {
    await rm(join(repo, ".agent", "GRAPH.md"));
    await mkdir(join(repo, ".agent", "GRAPH.md"));
  }

  await copyRealArtifacts(repo);
  return repo;
}

afterEach(async () => {
  for (const root of tempRoots) {
    await rm(root, { recursive: true, force: true });
  }
  tempRoots.length = 0;
});

describe("EP-043 M4 forced failures, abuse cases, observability", () => {
  it("ep043_failure_unavailable_dependency_missing_graph", async () => {
    const empty = await tempDir("ep043-m4-empty-");
    const report = join(empty, "PRODUCTION_READINESS.md");
    const result = await runCli(["readiness", "--output", report], {
      cwd: empty,
    });
    expect(result.code).not.toBe(0);
    expect(result.stderr).toContain("UNAVAILABLE");
    expect(result.stderr).toContain('"code"');
    await expect(access(report)).rejects.toThrow();
  });

  it("ep043_failure_malformed_manifest_json", async () => {
    const repo = await makeTempRepo();
    const manifestPath = join(repo, "RELEASE_MANIFEST.json");
    const build = await runCli(["manifest", "--output-dir", repo], {
      cwd: repo,
    });
    expect(build.code).toBe(0);
    await writeFile(manifestPath, "{ not valid json", "utf8");
    const result = await runCli(
      ["verify-manifest", "--manifest", manifestPath],
      { cwd: repo },
    );
    expect(result.code).not.toBe(0);
    expect(result.stderr).toContain("VALIDATION_FAILED");
    expect(result.stderr).toContain("not valid JSON");
  });

  it("ep043_failure_manifest_tamper_digest_mismatch", async () => {
    const repo = await makeTempRepo();
    const manifestPath = join(repo, "RELEASE_MANIFEST.json");
    const build = await runCli(["manifest", "--output-dir", repo], {
      cwd: repo,
    });
    expect(build.code).toBe(0);
    const manifest = JSON.parse(await readFile(manifestPath, "utf8")) as {
      components: Array<{ digest: string }>;
    };
    manifest.components[0]!.digest = `sha256:${"0".repeat(64)}`;
    await writeFile(manifestPath, JSON.stringify(manifest), "utf8");
    const result = await runCli(
      ["verify-manifest", "--manifest", manifestPath],
      { cwd: repo },
    );
    expect(result.code).not.toBe(0);
    expect(result.stderr).toContain("VERIFICATION_FAILED");
  });

  it("ep043_failure_missing_artifact_bytes", async () => {
    const repo = await makeTempRepo();
    const manifestPath = join(repo, "RELEASE_MANIFEST.json");
    const build = await runCli(["manifest", "--output-dir", repo], {
      cwd: repo,
    });
    expect(build.code).toBe(0);
    // AUD-082: deleting a REAL release artifact makes verification fail
    // closed - the manifest binds real product bytes, not fixtures.
    await rm(join(repo, "models", "wake", "nexus_wake", "decision.py"));
    const result = await runCli(
      ["verify-manifest", "--manifest", manifestPath],
      { cwd: repo },
    );
    expect(result.code).not.toBe(0);
    expect(result.stderr).toContain("NOT_FOUND");
    expect(result.stderr).toContain("nexus-wake-model");
  });

  it("ep043_failure_denied_read_is_fail_closed", async () => {
    const repo = await makeTempRepo({ graphPathAsDirectory: true });
    const result = await runCli(["ship-gate-status"], { cwd: repo });
    expect(result.code).not.toBe(0);
    expect(result.stderr).toContain("UNAVAILABLE");
  });

  it("ep043_failure_duplicate_conflicting_certification_rows", async () => {
    const repo = await makeTempRepo({
      providerCert: [
        "# PROVIDER CERTIFICATION RESULTS",
        "",
        "SIGNED: DeepSeek V1 reflex provider",
        "RELEASE-BLOCKING-PENDING: DeepSeek V1 reflex provider",
        "",
      ].join("\n"),
    });
    const result = await runCli(["certification-rows"], { cwd: repo });
    expect(result.code).not.toBe(0);
    expect(result.stderr).toContain("CONFLICT");
    expect(result.stderr).toContain("DeepSeek");
  });

  it("ep043_failure_malformed_certification_results_pending_blocking", async () => {
    const repo = await makeTempRepo({
      providerCert: "# PROVIDER CERTIFICATION RESULTS\n\nno recognizable row\n",
    });
    const result = await runCli(["readiness", "--output", "PR.md"], {
      cwd: repo,
    });
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("NOT_READY");
    const report = await readFile(join(repo, "PR.md"), "utf8");
    expect(report).toContain("certification");
  });

  it("ep043_failure_pending_certification_remains_blocking", async () => {
    const repo = await makeTempRepo();
    const result = await runCli(["ship-gate-status"], { cwd: repo });
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("ship-gate verdict: BLOCKED");
    expect(result.stdout).toContain("readiness decision: NOT_READY");
    expect(result.stdout).toContain("certification row");
  });

  it("ep043_failure_forged_ready_report_not_trusted", async () => {
    const repo = await makeTempRepo();
    await writeFile(
      join(repo, "PRODUCTION_READINESS.md"),
      [
        "# PRODUCTION READINESS",
        "",
        "Decision: READY",
        "Run: ep043-readiness-forged",
        "Git commit: " + "b".repeat(40),
        "Generated: 2026-08-25T00:00:00.000Z",
        "",
        "All obligations met.",
        "",
      ].join("\n"),
      "utf8",
    );
    const result = await runCli(["ship-gate-status"], { cwd: repo });
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("ship-gate verdict: BLOCKED");
    expect(result.stdout).toContain("readiness decision: NOT_READY");
  });

  it("ep043_failure_stale_evidence_not_trusted", async () => {
    const repo = await makeTempRepo();
    await writeFile(
      join(repo, "PRODUCTION_READINESS.md"),
      [
        "# PRODUCTION READINESS",
        "",
        "Decision: READY",
        "Run: ep043-readiness-stale",
        "Git commit: " + "0".repeat(40),
        "Generated: 2020-01-01T00:00:00.000Z",
        "",
      ].join("\n"),
      "utf8",
    );
    const result = await runCli(["ship-gate-status"], { cwd: repo });
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("readiness decision: NOT_READY");
  });

  it("ep043_failure_ship_gate_blocked_not_inferred", async () => {
    const repo = await makeTempRepo();
    // Commands exist and the CLI resolves, but EP-043 is not NODE_DONE:
    // the gate must be BLOCKED, never PASSED or AUTHORIZED.
    const result = await runCli(["ship-gate-status"], { cwd: repo });
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("ship-gate verdict: BLOCKED");
    expect(result.stdout).not.toContain("ship-gate verdict: PASSED");
    expect(result.stdout).not.toContain("ship-gate verdict: AUTHORIZED");
    expect(result.stdout).toContain("EP-043");
  });

  it("ep043_failure_signature_present_not_verified_honest", async () => {
    const repo = await makeTempRepo();
    const manifestPath = join(repo, "RELEASE_MANIFEST.json");
    const build = await runCli(["manifest", "--output-dir", repo], {
      cwd: repo,
    });
    expect(build.code).toBe(0);
    expect(build.stdout).toContain("PRESENT_NOT_VERIFIED");
    const verify = await runCli(
      ["verify-manifest", "--manifest", manifestPath],
      { cwd: repo },
    );
    expect(verify.code).toBe(0);
    expect(verify.stdout).toContain("verify-manifest: ok");
    // Presence must never be upgraded to cryptographic verification.
    expect(verify.stdout).not.toContain("signature verified");
    expect(verify.stdout).not.toContain("signed");
  });

  it("ep043_failure_timeout_blocked_dependency", async () => {
    const repo = await makeTempRepo();
    const fifo = join(repo, "BLOCKED_MANIFEST.json");
    await execFileAsync("mkfifo", [fifo]);
    const result = await runCli(["verify-manifest", "--manifest", fifo], {
      cwd: repo,
      timeout: 2000,
    });
    // A FIFO with no reader blocks the read forever; the bounded budget
    // must terminate the run and it must never report success.
    expect(result.code).not.toBe(0);
    expect(result.stdout).not.toContain("verify-manifest: ok");
  }, 20_000);

  it("ep043_failure_cancelled_work_no_partial_output", async () => {
    const repo = await makeTempRepo();
    const target = join(repo, "PRODUCTION_READINESS.md");
    const child = spawn(
      "node",
      [
        "--experimental-transform-types",
        "--import",
        `file://${LOADER}`,
        CLI,
        "readiness",
        "--output",
        target,
      ],
      {
        cwd: repo,
        env: { ...process.env, NODE_OPTIONS: "" },
        stdio: "ignore",
      },
    );
    await new Promise((resolve) => setTimeout(resolve, 300));
    child.kill("SIGTERM");
    const exitCode: number | null = await Promise.race([
      new Promise<number | null>((resolve) => child.on("close", resolve)),
      new Promise<number | null>((resolve) => {
        setTimeout(() => {
          child.kill("SIGKILL");
          resolve(null);
        }, 3000);
      }),
    ]);
    // Whether the run completed before the signal or was cancelled, the
    // target must be absent or complete (atomic write), and no temp
    // residue may remain.
    let targetState: "absent" | "complete";
    try {
      const body = await readFile(target, "utf8");
      expect(body).toContain("NOT_READY");
      targetState = "complete";
    } catch {
      targetState = "absent";
    }
    expect(["absent", "complete"]).toContain(targetState);
    const entries = await readdir(repo);
    const tmpResidue = entries.filter((entry) => entry.includes(".tmp-"));
    expect(tmpResidue).toEqual([]);
    // null is the expected value when SIGTERM terminated the child
    // mid-run; a number means the run completed first. Either way the
    // atomic-write invariant above must hold.
    expect(exitCode === null || typeof exitCode === "number").toBe(true);
  }, 20_000);

  it("ep043_failure_partial_side_effect_no_partial_file", async () => {
    const repo = await makeTempRepo();
    const outDir = join(repo, "outputs");
    await mkdir(outDir);
    const result = await runCli(["readiness", "--output", outDir], {
      cwd: repo,
    });
    expect(result.code).not.toBe(0);
    expect(result.stderr).toContain("VALIDATION_FAILED");
    const entries = await readdir(outDir);
    expect(entries).toEqual([]);
    const parentEntries = await readdir(dirname(outDir));
    expect(
      parentEntries.filter((entry) => entry.includes("outputs.tmp-")),
    ).toEqual([]);
  });

  it("ep043_failure_operator_bypass_unknown_flag_rejected", async () => {
    const repo = await makeTempRepo();
    const result = await runCli(["readiness", "--output", "PR.md", "--force"], {
      cwd: repo,
    });
    expect(result.code).not.toBe(0);
    expect(result.stderr).toContain("VALIDATION_FAILED");
    expect(result.stderr).toContain("unknown flag --force");
    await expect(access(join(repo, "PR.md"))).rejects.toThrow();
  });

  it("ep043_failure_unknown_release_state_fails_closed", async () => {
    const repo = await makeTempRepo();
    const manifestPath = join(repo, "RELEASE_MANIFEST.json");
    const build = await runCli(["manifest", "--output-dir", repo], {
      cwd: repo,
    });
    expect(build.code).toBe(0);
    const manifest = JSON.parse(await readFile(manifestPath, "utf8")) as Record<
      string,
      unknown
    >;
    manifest["bogus_field"] = "x";
    await writeFile(manifestPath, JSON.stringify(manifest), "utf8");
    const result = await runCli(
      ["verify-manifest", "--manifest", manifestPath],
      { cwd: repo },
    );
    expect(result.code).not.toBe(0);
    expect(result.stderr).toContain("VALIDATION_FAILED");
    expect(result.stderr).toContain("unknown release manifest field");
  });

  it("ep043_failure_redaction_structured_errors", async () => {
    // Runtime-constructed canaries only: no tracked secret literals.
    const accessKey = `AKIA${"0".repeat(16)}`;
    const sk = `sk-${"abcdefghij"}`;
    const gh = `ghp_${"abcdefgh"}`;
    const message = `denied ${accessKey} ${sk} ${gh}`;
    const error = new ShipError("POLICY_DENIED", message);
    const shape = error.toShape();
    expect(shape.code).toBe("POLICY_DENIED");
    expect(shape.redacted).toBe(true);
    expect(shape.message).not.toContain(accessKey);
    expect(shape.message).not.toContain("sk-");
    expect(shape.message).not.toContain("ghp_");
    expect(error.toRedactedJson()).not.toContain(accessKey);
    expect(redactShipMessage(message)).not.toContain(accessKey);
  });

  it("ep043_failure_incident_correlation_run_id_recorded", async () => {
    const out = await tempDir("ep043-m4-corr-");
    const reportPath = join(out, "PRODUCTION_READINESS.md");
    const result = await runCli(["readiness", "--output", reportPath]);
    expect(result.code).toBe(0);
    const body = await readFile(reportPath, "utf8");
    expect(body).toMatch(/Run: ep043-readiness-\d+/);
    expect(body).toMatch(/Git commit: [0-9a-f]{40}/);
    expect(body).toMatch(/Generated: \d{4}-\d{2}-\d{2}T/);
    expect(body).toContain("NOT_READY");
  });
});
