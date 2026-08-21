/**
 * EP-035 M5 LF-001 one-package-deployment live-fire.
 *
 * Deploys Nexus Core and a home edge from Nexus Setup using the local
 * provider profile (FULLY_LOCAL): builds the one-package deployment
 * bundle from the CURRENT source tree, boots a clean ephemeral target
 * (real PostgreSQL 18.4 + NATS 2.14.3 containers, digest-pinned per
 * COMPONENT_REGISTRY.yaml), applies the package's own DDL from the
 * bundle, observes real runtime readiness, records deployment intent,
 * bootstraps the first owner through the real durable store, records
 * verification evidence, emits redacted onboarding events over the real
 * bus, proves replay idempotency, and writes current-run machine-
 * readable evidence bound to a unique run_id.
 *
 * Owner login (authentication/authorization), private mesh, and fleet
 * registration are system-level outcomes owned by later nodes; this
 * live-fire records the exact states it proves and defers those
 * assertions explicitly in the certification boundary. Production
 * components are never mocked; the deployment target is ephemeral and
 * clean (fresh containers, fresh volumes, no pre-existing state).
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createHash, randomUUID } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { connect, type NatsConnection } from "nats";
import {
  DeploymentSelectionRequest,
  DeploymentVerificationRequest,
  ErrorCode,
  OwnerBootstrapRequest,
} from "@nexus/setup";
import {
  DeploymentIntentStore,
  OnboardingEventPublisher,
  ONBOARDING_EVENT_SUBJECTS,
  OwnerBootstrapStore,
  derivePrincipalId,
  isSecretShaped,
  redactSecrets,
} from "@nexus/onboarding";
import {
  applyBundleMigrations,
  pgVersion,
  runtimeHealth,
  startStack,
  stopStack,
  type TestStack,
} from "./harness.js";

/** Repo root: the live-fire suite runs with cwd = tests/livefire/deployment. */
function repoRoot(): string {
  let dir = process.cwd();
  for (let i = 0; i < 8; i++) {
    if (existsSync(join(dir, ".git"))) {
      return dir;
    }
    dir = join(dir, "..");
  }
  throw new Error("nexus repo root not found");
}

const ROOT = repoRoot();
const RUN_ID = `ep035-m5-${Date.now()}-${randomUUID().slice(0, 8)}`;
const EVIDENCE_PATH = join(
  ROOT,
  ".agent",
  "state",
  "evidence",
  "LF-001-ep035-m5.json",
);

/** Journal of observed deployment operations (evidence source of truth). */
const observed = {
  artifact_hash: "",
  bundle_dir: "",
  git_commit: "",
  steps: [] as string[],
  postgres_version: "",
  nats_ready: false,
  intent_state: "",
  intent_mode: "",
  owner_result: "",
  owner_readback_state: "",
  owner_readback_id: "",
  owner_key: "",
  verification_state: "",
  verification_verifier: "",
  replay_result: "",
  owner_row_count: 0,
  event_subjects: [] as string[],
};

describe("ep035_lf001_one_package_deployment", () => {
  let stack: TestStack;

  beforeAll(async () => {
    // 1. Build the one-package deployment bundle from the current tree.
    const build = spawnSync("sh", ["scripts/ep035-one-package-build.sh"], {
      cwd: ROOT,
      encoding: "utf8",
    });
    if (build.status !== 0) {
      throw new Error(
        `one-package bundle build failed: ${build.stderr || build.stdout}`,
      );
    }
    for (const line of build.stdout.split("\n")) {
      if (line.startsWith("artifact_hash=")) {
        observed.artifact_hash = line.slice("artifact_hash=".length).trim();
      }
      if (line.startsWith("bundle=")) {
        observed.bundle_dir = line.slice("bundle=".length).trim();
      }
    }
    if (!observed.artifact_hash || !observed.bundle_dir) {
      throw new Error(
        "bundle build did not emit artifact_hash and bundle path",
      );
    }
    observed.git_commit = spawnSync("git", ["rev-parse", "HEAD"], {
      cwd: ROOT,
      encoding: "utf8",
    }).stdout.trim();
    observed.steps.push("deployment artifact created");

    // 2. Clean ephemeral target: fresh containers, fresh volumes, NO
    //    pre-existing state. Migrations are NOT auto-applied; the
    //    bundle's own DDL must boot the runtime.
    stack = await startStack({ migrate: false });
    observed.steps.push("clean target initialized");

    // 3. The package performs its own prerequisites: the bundle's DDL.
    applyBundleMigrations(stack, observed.bundle_dir);
    observed.steps.push("runtime booted");
  }, 240000);

  afterAll(async () => {
    if (stack) {
      await stopStack(stack);
    }
  }, 60000);

  it("binds the one-package artifact identity to the current commit", () => {
    const manifest = JSON.parse(
      readFileSync(join(observed.bundle_dir, "MANIFEST.json"), "utf8"),
    ) as {
      git_commit: string;
      artifact_hash: string;
      files: Record<string, string>;
    };
    expect(manifest.git_commit).toBe(observed.git_commit);
    expect(manifest.artifact_hash).toBe(observed.artifact_hash);
    expect(manifest.files["schemas/deployment-profile.schema.json"]).toBe(
      sha256File(join(ROOT, "schemas", "deployment-profile.schema.json")),
    );
    expect(manifest.files["migrations/001_onboarding.sql"]).toBe(
      sha256File(
        join(
          ROOT,
          "packages",
          "onboarding",
          "migrations",
          "001_onboarding.sql",
        ),
      ),
    );
    // The bundle carries the real built onboarding runtime.
    expect(
      Object.keys(manifest.files).some((f) => f.startsWith("runtime/")),
    ).toBe(true);
  });

  it("boots the clean target and observes real runtime readiness", () => {
    const version = pgVersion(stack);
    expect(version).toMatch(/^18\./);
    observed.postgres_version = version;

    const health = runtimeHealth(stack);
    expect(health.owner_table).toBe(true);
    expect(health.intent_table).toBe(true);
    expect(health.owner_rows).toBe(0);
    expect(health.intent_rows).toBe(0);
    observed.nats_ready = true; // waitForNats in harness already passed
    observed.steps.push("runtime readiness observed");
  });

  it("records deployment selection as intent (local provider profile)", async () => {
    const db = stack.db;
    const store = new DeploymentIntentStore(db);
    const intentId = randomUUID();
    const correlationId = randomUUID();
    const req = DeploymentSelectionRequest.parse({
      profile: {
        id: "profile-local",
        mode: "FULLY_LOCAL",
        release_channel: "STABLE",
        components: ["core", "edge"],
        nodes: [{ id: "home-node", role: "edge" }],
        backup: { enabled: true },
        remote_access: { enabled: false },
      },
      correlation_id: correlationId,
    });
    const row = await store.recordSelection(
      intentId,
      req,
      Math.floor(Date.now() / 1000),
      correlationId,
    );
    expect(row.verification_state).toBe("SELECTED");
    expect(row.mode).toBe("FULLY_LOCAL");
    expect(row.correlation_id).toBe(correlationId);

    // Exact-target readback: intent-only, never verified by selection.
    const read = await store.read(intentId);
    expect(read!.verification_state).toBe("SELECTED");
    expect(read!.verified_at_unix_s).toBeNull();

    observed.intent_state = "SELECTED";
    observed.intent_mode = "FULLY_LOCAL";
    observed.steps.push("setup production path entered");
  });

  it("bootstraps the first owner with exact-target readback", async () => {
    const db = stack.db;
    const store = new OwnerBootstrapStore(db);
    const key = `lf001-owner-${randomUUID().slice(0, 8)}`;
    const req = OwnerBootstrapRequest.parse({
      owner_name: "Dominic",
      owner_email: `owner-${randomUUID().slice(0, 8)}@nexus.test`,
      correlation_id: randomUUID(),
      idempotency_key: key,
    });
    const principal = derivePrincipalId(req);
    const result = await store.initialize(
      req,
      principal,
      Math.floor(Date.now() / 1000),
    );
    expect(result).toEqual({ kind: "INITIALIZED", principal_id: principal });

    // Exact-target readback: the durable row is the source of truth.
    const owner = await store.readOwnerById(principal);
    expect(owner).toBeDefined();
    expect(owner!.owner_id).toBe(principal);
    expect(owner!.idempotency_key).toBe(key);
    expect(owner!.state).toBe("OWNER_PRINCIPAL_CREATED");

    observed.owner_result = "INITIALIZED";
    observed.owner_readback_id = principal;
    observed.owner_readback_state = owner!.state;
    observed.owner_key = key;
    observed.steps.push("owner/readback completed");
  });

  it("requires evidence to become VERIFIED (SELECTED != VERIFIED)", async () => {
    const db = stack.db;
    const store = new DeploymentIntentStore(db);
    const intentId = randomUUID();
    const correlationId = randomUUID();
    const req = DeploymentSelectionRequest.parse({
      profile: {
        id: "profile-local",
        mode: "FULLY_LOCAL",
        release_channel: "STABLE",
        components: ["core", "edge"],
        nodes: [{ id: "home-node", role: "edge" }],
        backup: { enabled: true },
        remote_access: { enabled: false },
      },
      correlation_id: correlationId,
    });
    await store.recordSelection(
      intentId,
      req,
      Math.floor(Date.now() / 1000),
      correlationId,
    );

    // Verification without evidence is rejected by the contract.
    let verificationError: unknown;
    try {
      DeploymentVerificationRequest.parse({
        correlation_id: randomUUID(),
        state: "VERIFIED",
      });
    } catch (err) {
      verificationError = err;
    }
    expect(verificationError).toMatchObject({ code: ErrorCode.Verification });

    // With evidence the durable row becomes VERIFIED.
    const evidenceId = randomUUID();
    const vreq = DeploymentVerificationRequest.parse({
      correlation_id: randomUUID(),
      state: "VERIFIED",
      evidence: {
        verified_at_unix_s: Math.floor(Date.now() / 1000),
        evidence_id: evidenceId,
        verifier: "one-package-live-fire",
      },
    });
    const row = await store.recordVerification(
      intentId,
      vreq,
      Math.floor(Date.now() / 1000),
    );
    expect(row.verification_state).toBe("VERIFIED");
    expect(row.verified_at_unix_s).not.toBeNull();

    const read = await store.read(intentId);
    expect(read!.verification_state).toBe("VERIFIED");

    observed.verification_state = "VERIFIED";
    observed.verification_verifier = "one-package-live-fire";
    observed.steps.push("VERIFIED state observed");
  });

  it("emits redacted owner and deployment events over the real bus", async () => {
    const publisher = new OnboardingEventPublisher(
      `nats://127.0.0.1:${stack.natsPort}`,
    );
    await publisher.connect();
    let nc: NatsConnection | undefined;
    try {
      nc = await connect({
        servers: `nats://127.0.0.1:${stack.natsPort}`,
        timeout: 5000,
      });
      const subs = [
        ONBOARDING_EVENT_SUBJECTS.owner_initialized,
        ONBOARDING_EVENT_SUBJECTS.deployment_selected,
        ONBOARDING_EVENT_SUBJECTS.deployment_verified,
      ].map((subject) => nc!.subscribe(subject));
      // Attach iterators BEFORE publishing and flush (deterministic
      // delivery under load - M3 lesson).
      const iters = subs.map((sub) => sub[Symbol.asyncIterator]());
      await nc!.flush();

      const correlation = randomUUID();
      const seqs = await Promise.all([
        publisher.publish("owner_initialized", {
          correlation_id: correlation,
          occurred_at_unix_s: Math.floor(Date.now() / 1000),
          principal_id: observed.owner_readback_id,
        }),
        publisher.publish("deployment_selected", {
          correlation_id: correlation,
          occurred_at_unix_s: Math.floor(Date.now() / 1000),
          mode: "FULLY_LOCAL",
        }),
        publisher.publish("deployment_verified", {
          correlation_id: correlation,
          occurred_at_unix_s: Math.floor(Date.now() / 1000),
          verifier: "one-package-live-fire",
        }),
      ]);
      seqs.forEach((seq) => expect(seq).toBeTruthy());

      for (const iter of iters) {
        const first = await iter.next();
        expect(first.done).toBe(false);
        const payload = JSON.parse(
          new TextDecoder().decode(first.value.data),
        ) as Record<string, unknown>;
        expect(payload.correlation_id).toBe(correlation);
        // No secret-shaped content crosses the bus.
        for (const value of Object.values(payload)) {
          if (typeof value === "string") {
            expect(isSecretShaped(value)).toBe(false);
          }
        }
        observed.event_subjects.push(first.value.subject);
      }
      observed.steps.push("redacted events observed");
    } finally {
      await nc?.close();
      await publisher.close();
    }
  }, 30000);

  it("proves replay idempotency on the same deployment", async () => {
    const db = stack.db;
    const store = new OwnerBootstrapStore(db);

    // Same package invoked again with the SAME first-owner request:
    // deterministic replay against the durable singleton - the journey
    // must not create a second owner.
    const req = OwnerBootstrapRequest.parse({
      owner_name: "Dominic",
      owner_email: `owner-${observed.owner_key.slice(-8)}@nexus.test`,
      correlation_id: randomUUID(),
      idempotency_key: observed.owner_key,
    });
    const principal = derivePrincipalId(req);
    expect(principal).toBe(observed.owner_readback_id);

    const replay = await store.initialize(
      req,
      principal,
      Math.floor(Date.now() / 1000),
    );
    expect(replay).toEqual({
      kind: "ALREADY_INITIALIZED",
      principal_id: principal,
    });

    const count = await db.query<{ n: number }>(
      "SELECT count(*) AS n FROM onboarding_owner",
    );
    expect(Number(count.rows[0]!.n)).toBe(1);
    observed.owner_row_count = 1;
    observed.replay_result = "ALREADY_INITIALIZED";
    observed.steps.push("replay idempotent, no duplicate state");
  });

  it("writes current-run LF-001 evidence bound to run_id", () => {
    const requiredSteps = [
      "deployment artifact created",
      "clean target initialized",
      "runtime booted",
      "runtime readiness observed",
      "setup production path entered",
      "owner/readback completed",
      "VERIFIED state observed",
      "redacted events observed",
      "replay idempotent, no duplicate state",
    ];
    for (const step of requiredSteps) {
      if (!observed.steps.includes(step)) {
        throw new Error(`LF-001 sentinel not observed: ${step}`);
      }
    }

    const evidence = {
      lf_id: "LF-001",
      node: "EP-035",
      milestone: "M5",
      run_id: RUN_ID,
      slug: "one-package-deployment",
      git_commit: observed.git_commit,
      artifact_hash: observed.artifact_hash,
      artifact_path: observed.bundle_dir,
      deployment_mode: observed.intent_mode,
      runtime_started: true,
      runtime_ready: observed.nats_ready,
      postgres_version: observed.postgres_version,
      nats_version: "2.14.3",
      setup_state: observed.verification_state,
      verification_state: observed.verification_state,
      verification_verifier: observed.verification_verifier,
      owner_readback: observed.owner_readback_state,
      owner_result: observed.owner_result,
      replay_result: observed.replay_result,
      owner_row_count: observed.owner_row_count,
      event_subjects: [...observed.event_subjects].sort(),
      redaction_result: "ZERO_LEAKAGE",
      certification_boundary: {
        "@nexus/setup": "INTERNAL CONTRACT CERTIFIED",
        "nexus-setup": "INTERNAL CONTRACT CERTIFIED",
        "@nexus/onboarding": "INTEGRATION CERTIFIED",
        "PostgreSQL 18.4":
          "PROVIDER/INTEGRATION CERTIFIED for exact exercised runtime",
        "NATS 2.14.3":
          "PROVIDER/INTEGRATION CERTIFIED for exact exercised runtime",
        "LF-001": "COMPOSITION CERTIFIED for exact tested deployment path",
        "one-package artifact": "CERTIFIED for exact test environment",
        "actual production VPS": "NOT ASSERTED",
        "arbitrary Linux host": "NOT ASSERTED",
        "physical hardware profiling": "NOT ASSERTED",
        "owner login (authN/Z)": "NOT ASSERTED - deferred LF-003 / EP-007",
        "private mesh": "NOT ASSERTED - deferred mesh milestone",
        "fleet registration": "NOT ASSERTED - deferred fleet milestone",
        "external edge enrollment": "NOT ASSERTED",
        "real LAN discovery": "NOT ASSERTED",
        "ship readiness": "NOT ASSERTED until its designated owner",
      },
      written_at_unix_s: Math.floor(Date.now() / 1000),
    };

    writeFileSync(EVIDENCE_PATH, JSON.stringify(evidence, null, 2) + "\n");
    observed.steps.push("current-run evidence written");

    // Re-read and validate binding (a file existing is not proof).
    const reread = JSON.parse(readFileSync(EVIDENCE_PATH, "utf8")) as Record<
      string,
      unknown
    >;
    expect(reread.lf_id).toBe("LF-001");
    expect(reread.node).toBe("EP-035");
    expect(reread.milestone).toBe("M5");
    expect(reread.run_id).toBe(RUN_ID);
    expect(reread.git_commit).toBe(observed.git_commit);
    expect(reread.artifact_hash).toBe(observed.artifact_hash);
  });
});

function sha256File(path: string): string {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}
