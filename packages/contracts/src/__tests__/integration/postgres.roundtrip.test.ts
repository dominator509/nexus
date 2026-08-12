import { describe, expect, it } from "vitest";
import { spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { Client } from "pg";
import type { ActionRequest, NexusControlObject } from "../../generated.js";

const IMAGE = "postgres:18.4";
const PASSWORD = "nexus-test";

function run(args: string[]): void {
  const res = spawnSync("docker", args, { encoding: "utf8" });
  if (res.status !== 0) {
    throw new Error(`docker ${args.join(" ")} failed: ${res.stderr}`);
  }
}

/** Host port docker assigned for the container's 5432. */
function hostPort(container: string): number {
  const res = spawnSync("docker", ["port", container, "5432"], {
    encoding: "utf8",
  });
  if (res.status !== 0) {
    throw new Error(`docker port ${container} 5432 failed: ${res.stderr}`);
  }
  const line = res.stdout.trim();
  return Number(line.slice(line.lastIndexOf(":") + 1));
}

async function waitForPostgres(port: number, timeoutMs = 60000): Promise<void> {
  // Readiness is proven by connecting through the PUBLISHED HOST PORT, not
  // pg_isready inside the container: docker's host-port publish can lag the
  // server, and the test consumes the host port (EP-001 M5 flake fix).
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown = null;
  while (Date.now() < deadline) {
    const probe = new Client({
      host: "127.0.0.1",
      port,
      user: "nexus",
      password: PASSWORD,
      database: "nexus",
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
    `postgres host port ${port} not ready within ${timeoutMs}ms`,
    { cause: lastError },
  );
}

describe("EP-001 generated contracts through real PostgreSQL", () => {
  it("round-trips a NexusControlObject and ActionRequest via SQL", async () => {
    const name = `nexus-ep001-ts-${randomUUID().slice(0, 8)}`;
    run([
      "run",
      "-d",
      "--name",
      name,
      "-e",
      `POSTGRES_USER=nexus`,
      "-e",
      `POSTGRES_PASSWORD=${PASSWORD}`,
      "-e",
      "POSTGRES_DB=nexus",
      "-p",
      `127.0.0.1::5432`,
      IMAGE,
    ]);
    try {
      const port = hostPort(name);
      await waitForPostgres(port);
      const client = new Client({
        host: "127.0.0.1",
        port,
        user: "nexus",
        password: PASSWORD,
        database: "nexus",
      });
      await client.connect();
      await client.query(
        "CREATE TABLE contract_roundtrip (id TEXT PRIMARY KEY, payload JSONB NOT NULL)",
      );
      const obj: NexusControlObject = {
        schema_version: "1.0.0",
        intent: "home.lights.set",
        route: "DETERMINISTIC",
        risk: "R0",
        privacy: "HOUSEHOLD",
        ambiguity: 0,
        approval_required: false,
        executable_instruction: true,
        confidence: 0.99,
        required_capabilities: ["home.lights.set"],
        entities: {},
      };
      const req: ActionRequest = {
        action_id: "act_1",
        tenant_id: "tenant_1",
        principal_id: "user_1",
        capability_id: "cap.lock",
        idempotency_key: "key_1",
        risk: "R3",
        approval_class: "HUMAN",
        reversal: "COMPENSATING",
        arguments: { door: "front" },
        expected_state: { locked: true },
        invocation: {
          request_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073",
          correlation_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6074",
          origin_system: "voice",
          external_actor_id: "user_1",
          external_actor_type: "HUMAN",
          channel: "voice",
        },
      };
      await client.query(
        "INSERT INTO contract_roundtrip (id, payload) VALUES ($1, $2), ($3, $4)",
        ["obj", JSON.stringify(obj), "req", JSON.stringify(req)],
      );
      const res = await client.query<{
        id: string;
        payload: Record<string, unknown>;
      }>("SELECT id, payload FROM contract_roundtrip ORDER BY id");
      const fetched = new Map(res.rows.map((r) => [r.id, r.payload]));
      expect(fetched.get("obj")).toEqual(obj);
      expect(fetched.get("req")).toEqual(req);
      await client.end();
    } finally {
      run(["rm", "-f", name]);
    }
  }, 120000);
});
