/**
 * EP-035 M4 forced-failure harness: real ephemeral PostgreSQL 18.4 and
 * NATS 2.14.3 containers (digest-pinned per COMPONENT_REGISTRY.yaml),
 * mirroring the M3 integration harness. The failure suite proves
 * fail-closed behavior by exercising the REAL provider boundary:
 * terminating containers, exhausting budgets, corrupting controlled
 * messages, and denying policy decisions. Production code is never
 * mocked; the failure mechanism is the real provider.
 */

import { spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { Client } from "pg";
import { OnboardingDb } from "@nexus/onboarding";

export const POSTGRES_IMAGE = "postgres:18.4";
export const POSTGRES_DIGEST =
  "sha256:a02db8cac496f15b094798a38254f14d6e00741f709360e5e00bb6668ea31636";
export const NATS_IMAGE = "nats:2.14.3";
export const NATS_DIGEST =
  "sha256:67ac7866d010e8d83302dd30332eeae1a2b7a8ee051155e2eb5a5485b720cd4b";
export const PG_USER = "nexus";
export const PG_PASSWORD = "nexus-test";
export const PG_DB = "nexus";

export function run(args: string[]): void {
  const res = spawnSync("docker", args, { encoding: "utf8" });
  if (res.status !== 0) {
    throw new Error(`docker ${args.join(" ")} failed: ${res.stderr}`);
  }
}

export function hostPort(container: string, internalPort: number): number {
  const res = spawnSync("docker", ["port", container, String(internalPort)], {
    encoding: "utf8",
  });
  if (res.status !== 0) {
    throw new Error(
      `docker port ${container} ${internalPort} failed: ${res.stderr}`,
    );
  }
  const line = res.stdout.trim();
  return Number(line.slice(line.lastIndexOf(":") + 1));
}

export async function waitForPostgres(
  port: number,
  timeoutMs = 90000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown = null;
  while (Date.now() < deadline) {
    const probe = new Client({
      host: "127.0.0.1",
      port,
      user: PG_USER,
      password: PG_PASSWORD,
      database: PG_DB,
      connectionTimeoutMillis: 2000,
    });
    try {
      await probe.connect();
      await probe.end();
      return;
    } catch (err) {
      lastError = err;
      await new Promise((r) => setTimeout(r, 500));
    }
  }
  throw new Error(
    `postgres host port ${port} not ready within ${timeoutMs}ms: ${String(lastError)}`,
  );
}

export async function waitForNats(
  port: number,
  timeoutMs = 60000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    // NATS sends its INFO banner immediately on connect; read a larger
    // buffer so the PONG to our PING is actually visible (a 64-byte
    // read only ever captures INFO).
    const ok = spawnSync("bash", [
      "-c",
      `exec 3<>/dev/tcp/127.0.0.1/${port} && printf 'PING\\r\\n' >&3 && timeout 3 dd bs=4096 count=1 <&3 2>/dev/null | grep -q PONG`,
    ]);
    if (ok.status === 0) {
      return;
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`nats host port ${port} not ready within ${timeoutMs}ms`);
}

export interface TestStack {
  pgName: string;
  natsName: string;
  pgPort: number;
  natsPort: number;
  db: OnboardingDb;
}

/** Start both containers, wait for readiness, run migrations. */
export async function startStack(): Promise<TestStack> {
  const pgName = `nexus-ep035-pg-${randomUUID().slice(0, 8)}`;
  const natsName = `nexus-ep035-nats-${randomUUID().slice(0, 8)}`;

  run([
    "run",
    "-d",
    "--name",
    pgName,
    "-e",
    `POSTGRES_USER=${PG_USER}`,
    "-e",
    `POSTGRES_PASSWORD=${PG_PASSWORD}`,
    "-e",
    "POSTGRES_DB=nexus",
    "-p",
    "127.0.0.1::5432",
    `${POSTGRES_IMAGE}@${POSTGRES_DIGEST}`,
  ]);
  run([
    "run",
    "-d",
    "--name",
    natsName,
    "-p",
    "127.0.0.1::4222",
    `${NATS_IMAGE}@${NATS_DIGEST}`,
    "-js", // enable JetStream (off by default in nats-server 2.x)
  ]);

  try {
    const pgPort = hostPort(pgName, 5432);
    const natsPort = hostPort(natsName, 4222);
    await waitForPostgres(pgPort);
    await waitForNats(natsPort);

    const db = new OnboardingDb({
      host: "127.0.0.1",
      port: pgPort,
      user: PG_USER,
      password: PG_PASSWORD,
      database: PG_DB,
    });
    await db.migrate();
    return { pgName, natsName, pgPort, natsPort, db };
  } catch (err) {
    run(["rm", "-f", pgName]);
    run(["rm", "-f", natsName]);
    throw err;
  }
}

/** Stop and remove both containers. */
export async function stopStack(stack: TestStack): Promise<void> {
  try {
    for (const db of OPEN_DBS) {
      try {
        await db.close();
      } catch {
        // already closed
      }
    }
    OPEN_DBS.clear();
  } finally {
    run(["rm", "-f", stack.pgName]);
    run(["rm", "-f", stack.natsName]);
  }
}

const OPEN_DBS = new Set<OnboardingDb>();

/** Fresh DB for each test: run migrations and clear onboarding tables. */
export async function freshDb(stack: TestStack): Promise<OnboardingDb> {
  const db = new OnboardingDb({
    host: "127.0.0.1",
    port: stack.pgPort,
    user: PG_USER,
    password: PG_PASSWORD,
    database: PG_DB,
  });
  OPEN_DBS.add(db);
  await db.migrate();
  // Each test starts from an empty durable state (the singleton first-
  // owner index and one-time token tables must not carry prior rows).
  await db.query(
    `TRUNCATE onboarding_owner, onboarding_deployment_intent,
             onboarding_enrollment_credential, onboarding_integration_state,
             onboarding_recovery_checkpoint, onboarding_event_log`,
  );
  return db;
}

export function pgVersion(stack: TestStack): string {
  const res = spawnSync(
    "bash",
    [
      "-c",
      `PGPASSWORD=${PG_PASSWORD} psql -h 127.0.0.1 -p ${stack.pgPort} -U ${PG_USER} -d ${PG_DB} -Atc 'SHOW server_version'`,
    ],
    { encoding: "utf8" },
  );
  return res.stdout.trim();
}

/** Kill the postgres container immediately (real provider termination). */
export function killPostgres(stack: TestStack): void {
  run(["rm", "-f", stack.pgName]);
}

/** Kill the NATS container immediately (real provider termination). */
export function killNats(stack: TestStack): void {
  run(["rm", "-f", stack.natsName]);
}
