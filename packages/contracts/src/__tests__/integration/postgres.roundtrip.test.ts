import { describe, expect, it } from "vitest";
import { spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { Client } from "pg";
import type { ActionRequest, NexusControlObject } from "../../generated.js";

const IMAGE = "postgres:18.4";
const PORT = 55433; // distinct from the Python integration test port
const PASSWORD = "nexus-test";

function run(args: string[]): void {
  const res = spawnSync("docker", args, { encoding: "utf8" });
  if (res.status !== 0) {
    throw new Error(`docker ${args.join(" ")} failed: ${res.stderr}`);
  }
}

async function waitForPostgres(
  container: string,
  timeoutMs = 60000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const res = spawnSync(
      "docker",
      ["exec", container, "pg_isready", "-U", "nexus", "-d", "nexus"],
      {
        encoding: "utf8",
      },
    );
    if (res.status === 0) return;
    await new Promise((r) => setTimeout(r, 1000));
  }
  throw new Error(
    `postgres container ${container} not ready within ${timeoutMs}ms`,
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
      `127.0.0.1:${PORT}:5432`,
      IMAGE,
    ]);
    try {
      await waitForPostgres(name);
      const client = new Client({
        host: "127.0.0.1",
        port: PORT,
        user: "nexus",
        password: PASSWORD,
        database: "nexus",
      });
      await client.connect();
      await client.query(
        "CREATE TABLE contract_roundtrip (id TEXT PRIMARY KEY, payload JSONB NOT NULL)",
      );
      const obj: NexusControlObject = {
        schemaVersion: "1",
        intent: "home.lights.set",
        route: "DETERMINISTIC",
        risk: "R0",
        privacy: "HOUSEHOLD",
        ambiguity: 0,
        approvalRequired: false,
        executableInstruction: true,
        confidence: 0.99,
        requiredCapabilities: ["home.lights.set"],
        entities: {},
      };
      const req: ActionRequest = {
        actionId: "act_1",
        tenantId: "tenant_1",
        principalId: "user_1",
        capabilityId: "cap.lock",
        idempotencyKey: "key_1",
        risk: "R3",
        approvalClass: "HUMAN",
        reversal: "COMPENSATING",
        arguments: { door: "front" },
        expectedState: { locked: true },
        invocation: { channel: "voice" },
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
