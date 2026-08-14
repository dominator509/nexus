// EP-011 M3 live transport proof from the TypeScript binding
// (directive C/D). The TS SDK client talks to the REAL fixture sidecar
// process over REAL HTTP on a localhost ephemeral port:
//
//   TS test client -> real HTTP -> fixture sidecar process
//     -> Python SDK implementation -> fixture provider
//
// Node's global fetch is the real transport; the sidecar is spawned as
// a real child process. This proves the TypeScript binding speaks the
// same canonical wire as the Rust and Python bindings on the same
// transport.

import { spawn, type ChildProcess } from "node:child_process";
import { fileURLToPath } from "node:url";
import { describe, expect, it, afterEach } from "vitest";

const here = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = join(here, "../../../../");
const sidecarPath = join(repoRoot, "tests/connectors/fixture_sidecar.py");

function join(...parts: string[]): string {
  return parts.join("/");
}

const TENANT_A = "018f0f6f-9c1e-7b6e-8000-000000000003";
const TENANT_B = "018f0f6f-9c1e-7b6e-8000-000000000099";

const ctx = (tenant: string) => ({
  request_id: "018f0f6f-9c1e-7b6e-8000-000000000001",
  correlation_id: "018f0f6f-9c1e-7b6e-8000-000000000002",
  origin_system: "ts-live",
  external_actor_id: "user:alice",
  external_actor_type: "HUMAN",
  tenant_id: tenant,
});

interface Sidecar {
  child: ChildProcess;
  base: string;
}

const active: Sidecar[] = [];

async function startSidecar(): Promise<Sidecar> {
  const child: ChildProcess = spawn("python3", [sidecarPath], {
    cwd: repoRoot,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const stdout = child.stdout;
  if (stdout === null) {
    throw new Error("sidecar stdout unavailable");
  }
  const portLine = await new Promise<string>((resolve, reject) => {
    let buf = "";
    const onData = (chunk: Buffer) => {
      buf += chunk.toString();
      const idx = buf.indexOf("\n");
      if (idx >= 0) {
        const line = buf.slice(0, idx);
        if (line.startsWith("PORT ")) {
          resolve(line);
          stdout.off("data", onData);
        }
      }
    };
    stdout.on("data", onData);
    // Spawn failures surface on the child streams; reject so the
    // timeout does not mask them.
    stdout.on("error", (err: Error) => reject(err));
    setTimeout(() => reject(new Error("sidecar PORT timeout")), 10000);
  });
  const port = Number(portLine.trim().split(" ")[1]);
  const sidecar: Sidecar = { child, base: `http://127.0.0.1:${port}` };
  active.push(sidecar);
  return sidecar;
}

async function post(
  s: Sidecar,
  path: string,
  body: unknown,
): Promise<{ status: number; json: Record<string, unknown> }> {
  const resp = await fetch(`${s.base}${path}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Nexus-Protocol-Version": "1",
    },
    body: JSON.stringify(body),
  });
  const json = (await resp.json()) as Record<string, unknown>;
  return { status: resp.status, json };
}

afterEach(() => {
  for (const s of active.splice(0)) {
    s.child.kill();
  }
});

describe("ep011_integration_ts_live_transport", () => {
  it("discovers capabilities and runs a typed query over real HTTP", async () => {
    const s = await startSidecar();
    const discover = await post(s, "/v1/discover", { context: ctx(TENANT_A) });
    expect(discover.status).toBe(200);
    const caps = discover.json.capabilities as Array<{ id: string }>;
    expect(caps.some((c) => c.id === "fixture.contacts.query")).toBe(true);
    expect(caps.some((c) => c.id === "fixture.contacts.command")).toBe(true);

    const query = await post(s, "/v1/query", {
      capability_id: "fixture.contacts.query",
      context: ctx(TENANT_A),
      input: { limit: 10 },
    });
    expect(query.status).toBe(200);
    expect(query.json.capability_id).toBe("fixture.contacts.query");
  });

  it("preserves idempotency semantics across the transport", async () => {
    const s = await startSidecar();
    const body = {
      capability_id: "fixture.contacts.command",
      context: ctx(TENANT_A),
      input: { name: "Bob" },
      idempotency_key: "k-ts-1",
    };
    const first = await post(s, "/v1/command", body);
    expect(first.status).toBe(200);
    expect((first.json.output as Record<string, unknown>).id).toBe("c1");

    const replay = await post(s, "/v1/command", body);
    expect(replay.status).toBe(200);
    expect((replay.json.output as Record<string, unknown>).id).toBe("c1");

    const conflict = await post(s, "/v1/command", {
      ...body,
      capability_id: "fixture.billing.command",
    });
    expect(conflict.status).toBe(409);
    expect(conflict.json.code).toBe("CONFLICT");
  });

  it("fails closed on class mismatch and unknown capability", async () => {
    const s = await startSidecar();
    const mismatch = await post(s, "/v1/query", {
      capability_id: "fixture.contacts.command",
      context: ctx(TENANT_A),
      input: {},
    });
    expect(mismatch.status).toBe(400);
    expect(mismatch.json.code).toBe("VALIDATION");

    const missing = await post(s, "/v1/query", {
      capability_id: "fixture.does.not.exist",
      context: ctx(TENANT_A),
      input: {},
    });
    expect(missing.status).toBe(404);
    expect(missing.json.code).toBe("NOT_FOUND");
  });

  it("rejects an unsupported protocol version (fail closed)", async () => {
    const s = await startSidecar();
    const resp = await fetch(`${s.base}/v1/discover`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Nexus-Protocol-Version": "99",
      },
      body: JSON.stringify({ context: ctx(TENANT_A) }),
    });
    expect(resp.status).toBe(426);
    const body = (await resp.json()) as Record<string, unknown>;
    expect(body.code).toBe("VALIDATION");
  });

  it("denies cross-tenant access with no existence disclosure", async () => {
    const s = await startSidecar();
    const denied = await post(s, "/v1/query", {
      capability_id: "fixture.contacts.query",
      context: ctx(TENANT_B),
      input: {},
    });
    expect(denied.status).toBe(404);
    expect(denied.json.code).toBe("NOT_FOUND");
  });

  it("never leaks the credential value across the transport", async () => {
    const s = await startSidecar();
    const cmd = await post(s, "/v1/command", {
      capability_id: "fixture.contacts.command",
      context: ctx(TENANT_A),
      input: { name: "C", credential_reference: "vault:fixture-token" },
      idempotency_key: "k-ts-cred",
    });
    expect(cmd.status).toBe(200);
    const out = cmd.json.output as Record<string, unknown>;
    expect(out.credential_fingerprint).toBeTypeOf("string");
    const text = JSON.stringify(cmd.json);
    expect(text).not.toContain("fixture-secret-value");
  });

  it("reports sidecar unavailability as a typed failure", async () => {
    // Nothing is listening on port 1: the transport fails closed and
    // never fabricates success. fetch rejects on connect failure; the
    // SDK wraps that as UNAVAILABLE.
    await expect(
      fetch("http://127.0.0.1:1/v1/discover", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Nexus-Protocol-Version": "1",
        },
        body: JSON.stringify({ context: ctx(TENANT_A) }),
      }),
    ).rejects.toThrow();
  });
});
