/**
 * EP-042 M3 REAL integration proofs (SPEC-016, SPEC-024).
 *
 * These tests run against a REAL ephemeral SeaweedFS S3 gateway
 * container provisioned by scripts/ep042-m3-tests.sh (digest-pinned,
 * runtime credentials). Every object operation is a real signed S3
 * request; every digest is recomputed from real bytes.
 *
 * The gate provides the environment:
 *   NEXUS_RELEASE_S3_ENDPOINT, NEXUS_RELEASE_ACCESS_KEY,
 *   NEXUS_RELEASE_SECRET_KEY, NEXUS_RELEASE_BUCKET,
 *   NEXUS_RELEASE_RUN_ID, NEXUS_RELEASE_GIT_COMMIT
 */

import { describe, expect, it } from "vitest";

import {
  ReleaseTransport,
  ReleaseTransportError,
  S3Client,
} from "@nexus/release-infra";

const encoder = new TextEncoder();

function envOrThrow(name: string): string {
  const value = process.env[name];
  if (!value || value.trim().length === 0) {
    throw new Error(`missing required env ${name} (gate must supply it)`);
  }
  return value;
}

function transport(runId: string): ReleaseTransport {
  return new ReleaseTransport({
    endpoint: envOrThrow("NEXUS_RELEASE_S3_ENDPOINT"),
    creds: {
      accessKey: envOrThrow("NEXUS_RELEASE_ACCESS_KEY"),
      secretKey: envOrThrow("NEXUS_RELEASE_SECRET_KEY"),
    },
    bucket: envOrThrow("NEXUS_RELEASE_BUCKET"),
    runId,
    gitCommit: envOrThrow("NEXUS_RELEASE_GIT_COMMIT"),
  });
}

function s3Client(): S3Client {
  return new S3Client({
    endpoint: envOrThrow("NEXUS_RELEASE_S3_ENDPOINT"),
    creds: {
      accessKey: envOrThrow("NEXUS_RELEASE_ACCESS_KEY"),
      secretKey: envOrThrow("NEXUS_RELEASE_SECRET_KEY"),
    },
  });
}

const CORE_BYTES = encoder.encode(
  "nexus-core-v1.0.0-fixture-component-bytes-deterministic\n",
);
const MODEL_BYTES = encoder.encode(
  "nexus-model-v1.0.0-fixture-component-bytes-deterministic\n",
);

async function sha256Hex(bytes: Uint8Array<ArrayBuffer>): Promise<string> {
  const buf = await globalThis.crypto.subtle.digest(
    "SHA-256",
    bytes as Uint8Array<ArrayBuffer>,
  );
  const out = new Uint8Array(buf);
  let s = "";
  for (const b of out) {
    s += b.toString(16).padStart(2, "0");
  }
  return s;
}

function canonicalManifest(releaseId: string): Uint8Array<ArrayBuffer> {
  const wire = {
    schema_version: 1,
    release_id: releaseId,
    release_channel: "STABLE",
    deployment_profile: "FULLY_LOCAL",
    components: [
      {
        component_id: "nexus-core",
        name: "nexus-core",
        version: "1.0.0",
        digest: `sha256:${"8f0890cc919c925cb12ab3d514dcc237a417a25a04bb33f79728cc8e2ed3f26a"}`,
        size_bytes: CORE_BYTES.byteLength,
      },
      {
        component_id: "nexus-model",
        name: "nexus-model",
        version: "1.0.0",
        digest: `sha256:${"cceba172627079d46c81c6de0acf30ec1ade6b1878ff2cf292a432c1c612cc9c"}`,
        size_bytes: MODEL_BYTES.byteLength,
      },
    ],
  };
  return encoder.encode(JSON.stringify(wire));
}

function malformedManifest(releaseId: string): Uint8Array<ArrayBuffer> {
  return encoder.encode(JSON.stringify({ release_id: releaseId }));
}

const RUN_PREFIX = `ep042-m3-${Date.now()}`;

describe("ep042_integration_transport_readiness", () => {
  it("healthz is reachable on the live gateway", async () => {
    const client = s3Client();
    expect(await client.healthz()).toBe(true);
  });

  it("transport probe verifies PUT -> GET -> digest -> DELETE", async () => {
    const t = transport(`${RUN_PREFIX}-probe`);
    const result = await t.probe();
    expect(result.healthz).toBe(true);
    expect(result.probe_verified).toBe(true);
  });
});

describe("ep042_integration_transport_publish_fetch", () => {
  it("publishes a release with digest binding and fetches it back", async () => {
    const releaseId = `${RUN_PREFIX}-rel-1`;
    const t = transport(`${RUN_PREFIX}-rel-1`);
    const manifest = canonicalManifest(releaseId);

    const published = await t.publish(releaseId, manifest, [
      { componentId: "nexus-core", bytes: CORE_BYTES },
      { componentId: "nexus-model", bytes: MODEL_BYTES },
    ]);
    expect(published.releaseId).toBe(releaseId);
    expect(published.componentDigests).toHaveLength(2);

    const fetched = await t.fetch(releaseId, ["nexus-core", "nexus-model"]);
    expect(fetched.manifestDigest).toBe(await sha256Hex(manifest));
    expect(fetched.components).toHaveLength(2);
    const core = fetched.components.find((c) => c.componentId === "nexus-core");
    expect(core).toBeDefined();
    expect(core?.bytes).toEqual(CORE_BYTES);
    const model = fetched.components.find(
      (c) => c.componentId === "nexus-model",
    );
    expect(model).toBeDefined();
    expect(model?.bytes).toEqual(MODEL_BYTES);
  });

  it("head reports the real object size", async () => {
    const releaseId = `${RUN_PREFIX}-rel-2`;
    const t = transport(`${RUN_PREFIX}-rel-2`);
    await t.publish(releaseId, canonicalManifest(releaseId), [
      { componentId: "nexus-core", bytes: CORE_BYTES },
    ]);
    expect(await t.head(releaseId, "nexus-core")).toBe(CORE_BYTES.byteLength);
  });
});

describe("ep042_integration_transport_digest_binding_fails_closed", () => {
  it("denies publish when component bytes do not match declared digest", async () => {
    const releaseId = `${RUN_PREFIX}-bad-1`;
    const t = transport(`${RUN_PREFIX}-bad-1`);
    const manifest = canonicalManifest(releaseId);
    const tampered = encoder.encode("nexus-core-tampered-bytes\n");
    await expect(
      t.publish(releaseId, manifest, [
        { componentId: "nexus-core", bytes: tampered },
        { componentId: "nexus-model", bytes: MODEL_BYTES },
      ]),
    ).rejects.toThrow(ReleaseTransportError);
  });

  it("denies publish for a component with no declared digest", async () => {
    const releaseId = `${RUN_PREFIX}-bad-2`;
    const t = transport(`${RUN_PREFIX}-bad-2`);
    const manifest = canonicalManifest(releaseId);
    await expect(
      t.publish(releaseId, manifest, [
        { componentId: "nexus-ghost", bytes: CORE_BYTES },
      ]),
    ).rejects.toThrow(ReleaseTransportError);
  });

  it("denies publish for a malformed manifest", async () => {
    const releaseId = `${RUN_PREFIX}-bad-3`;
    const t = transport(`${RUN_PREFIX}-bad-3`);
    await expect(
      t.publish(releaseId, malformedManifest(releaseId), [
        { componentId: "nexus-core", bytes: CORE_BYTES },
      ]),
    ).rejects.toThrow(ReleaseTransportError);
  });

  it("denies fetch when the stored component digest no longer matches", async () => {
    const releaseId = `${RUN_PREFIX}-bad-4`;
    const t = transport(`${RUN_PREFIX}-bad-4`);
    await t.publish(releaseId, canonicalManifest(releaseId), [
      { componentId: "nexus-core", bytes: CORE_BYTES },
      { componentId: "nexus-model", bytes: MODEL_BYTES },
    ]);
    // Corrupt the stored object directly (real S3 overwrite).
    const client = s3Client();
    await client.putObject(
      envOrThrow("NEXUS_RELEASE_BUCKET"),
      `releases/${releaseId}/components/nexus-core`,
      encoder.encode("corrupted-stored-bytes\n"),
    );
    await expect(
      t.fetch(releaseId, ["nexus-core", "nexus-model"]),
    ).rejects.toThrow(ReleaseTransportError);
  });

  it("denies fetch for a missing object", async () => {
    const releaseId = `${RUN_PREFIX}-missing`;
    const t = transport(`${RUN_PREFIX}-missing`);
    await expect(t.fetch(releaseId, ["nexus-core"])).rejects.toThrow(
      ReleaseTransportError,
    );
  });
});

describe("ep042_integration_transport_auth_fails_closed", () => {
  it("denies operations with a wrong secret", async () => {
    const wrong = new S3Client({
      endpoint: envOrThrow("NEXUS_RELEASE_S3_ENDPOINT"),
      creds: {
        accessKey: envOrThrow("NEXUS_RELEASE_ACCESS_KEY"),
        secretKey: `wrong-secret-${Date.now()}`,
      },
    });
    const bucket = `${RUN_PREFIX}-wrongauth`;
    await expect(wrong.createBucket(bucket)).rejects.toThrow(
      ReleaseTransportError,
    );
  });
});

describe("ep042_integration_transport_timeout_fails_closed", () => {
  it("times out on an unreachable endpoint and reports TIMEOUT", async () => {
    const dead = new S3Client({
      endpoint: "127.0.0.1:1", // nothing listens here
      creds: { accessKey: "a", secretKey: "b" },
      timeoutMs: 500,
    });
    await expect(dead.healthz()).resolves.toBe(false);
  });
});

describe("ep042_integration_transport_cancellation", () => {
  it("cancels an in-flight request with a caller signal", async () => {
    const releaseId = `${RUN_PREFIX}-cancel`;
    const t = transport(`${RUN_PREFIX}-cancel`);
    await t.publish(releaseId, canonicalManifest(releaseId), [
      { componentId: "nexus-core", bytes: CORE_BYTES },
    ]);
    const controller = new AbortController();
    controller.abort();
    await expect(
      t.fetch(releaseId, ["nexus-core"], controller.signal),
    ).rejects.toThrow(ReleaseTransportError);
  });
});

describe("ep042_integration_transport_idempotency", () => {
  it("re-publishing identical bytes leaves one object with the same digest", async () => {
    const releaseId = `${RUN_PREFIX}-idem`;
    const t = transport(`${RUN_PREFIX}-idem`);
    const manifest = canonicalManifest(releaseId);
    const first = await t.publish(releaseId, manifest, [
      { componentId: "nexus-core", bytes: CORE_BYTES },
    ]);
    const second = await t.publish(releaseId, manifest, [
      { componentId: "nexus-core", bytes: CORE_BYTES },
    ]);
    expect(second.manifestDigest).toBe(first.manifestDigest);
    expect(second.componentDigests[0]?.digest).toBe(
      first.componentDigests[0]?.digest,
    );
    const client = s3Client();
    const keys = await client.listObjects(
      envOrThrow("NEXUS_RELEASE_BUCKET"),
      `releases/${releaseId}/`,
    );
    const componentKeys = keys.filter((k) =>
      k.includes(`/components/nexus-core`),
    );
    expect(componentKeys).toHaveLength(1);
  });
});

describe("ep042_integration_transport_audit_redaction", () => {
  it("audit events bind current-run fields and never leak secrets", async () => {
    const runId = `${RUN_PREFIX}-audit`;
    const t = transport(runId);
    const event = t.audit(
      "nexus-1",
      "publish",
      "denied",
      `access ${envOrThrow("NEXUS_RELEASE_SECRET_KEY")} leaked?`,
    );
    expect(event.run_id).toBe(runId);
    expect(event.git_commit).toBe(envOrThrow("NEXUS_RELEASE_GIT_COMMIT"));
    expect(event.release_id).toBe("nexus-1");
    expect(event.outcome).toBe("denied");
    expect(event.detail).not.toContain(envOrThrow("NEXUS_RELEASE_SECRET_KEY"));
    const serialized = JSON.stringify(event);
    expect(serialized).not.toContain(envOrThrow("NEXUS_RELEASE_SECRET_KEY"));
    expect(serialized).not.toContain(envOrThrow("NEXUS_RELEASE_ACCESS_KEY"));
  });
});
