/**
 * EP-042 M3 transport unit proofs (SPEC-024).
 *
 * Deterministic SigV4 signing behavior and fail-closed config
 * validation. These run without a container; the REAL gateway behavior
 * is proven by the ep042_integration_* suite against a live SeaweedFS
 * container (tests/release/src/integration/).
 */

import { describe, expect, it } from "vitest";

import { ReleaseTransportError } from "../errors";
import { S3Client, s3Url } from "../s3";
import { signRequest, uriEncode } from "../sigv4";
import {
  ReleaseTransport,
  redact,
  componentKey,
  manifestKey,
} from "../transport";

const encoder = new TextEncoder();

describe("ep042_unit_sigv4_uri_encode", () => {
  it("keeps unreserved characters verbatim", () => {
    expect(uriEncode("abc-_.~XYZ0129")).toBe("abc-_.~XYZ0129");
  });

  it("percent-encodes reserved characters", () => {
    expect(uriEncode("a b")).toBe("a%20b");
    expect(uriEncode("a/b")).toBe("a/b");
    expect(uriEncode("a/b", true)).toBe("a%2Fb");
  });
});

describe("ep042_unit_sigv4_deterministic_signing", () => {
  it("produces a deterministic signature for identical inputs", async () => {
    const body = encoder.encode("deterministic-body");
    const now = new Date("2026-08-25T00:00:00.000Z");
    const a = await signRequest(
      "PUT",
      "127.0.0.1:8333",
      "/bucket/key",
      "",
      {},
      body,
      { accessKey: "nexus-access", secretKey: "nexus-secret" },
      { region: "us-east-1", service: "s3" },
      now,
    );
    const b = await signRequest(
      "PUT",
      "127.0.0.1:8333",
      "/bucket/key",
      "",
      {},
      body,
      { accessKey: "nexus-access", secretKey: "nexus-secret" },
      { region: "us-east-1", service: "s3" },
      now,
    );
    expect(a.headers["authorization"]).toBeDefined();
    expect(a.headers["authorization"]).toBe(b.headers["authorization"]);
  });

  it("changes signature when the payload changes", async () => {
    const now = new Date("2026-08-25T00:00:00.000Z");
    const a = await signRequest(
      "PUT",
      "127.0.0.1:8333",
      "/bucket/key",
      "",
      {},
      encoder.encode("body-a"),
      { accessKey: "nexus-access", secretKey: "nexus-secret" },
      { region: "us-east-1", service: "s3" },
      now,
    );
    const b = await signRequest(
      "PUT",
      "127.0.0.1:8333",
      "/bucket/key",
      "",
      {},
      encoder.encode("body-b"),
      { accessKey: "nexus-access", secretKey: "nexus-secret" },
      { region: "us-east-1", service: "s3" },
      now,
    );
    expect(a.headers["authorization"]).not.toBe(b.headers["authorization"]);
  });

  it("changes signature when the secret changes", async () => {
    const now = new Date("2026-08-25T00:00:00.000Z");
    const a = await signRequest(
      "GET",
      "127.0.0.1:8333",
      "/bucket/key",
      "",
      {},
      new Uint8Array(0),
      { accessKey: "nexus-access", secretKey: "secret-a" },
      { region: "us-east-1", service: "s3" },
      now,
    );
    const b = await signRequest(
      "GET",
      "127.0.0.1:8333",
      "/bucket/key",
      "",
      {},
      new Uint8Array(0),
      { accessKey: "nexus-access", secretKey: "secret-b" },
      { region: "us-east-1", service: "s3" },
      now,
    );
    expect(a.headers["authorization"]).not.toBe(b.headers["authorization"]);
  });

  it("signs a canonical request with correct signed headers", async () => {
    const now = new Date("2026-08-25T00:00:00.000Z");
    const signed = await signRequest(
      "GET",
      "127.0.0.1:8333",
      "/bucket",
      "",
      {},
      new Uint8Array(0),
      { accessKey: "nexus-access", secretKey: "nexus-secret" },
      { region: "us-east-1", service: "s3" },
      now,
    );
    const auth = signed.headers["authorization"] ?? "";
    expect(
      auth.startsWith(
        "AWS4-HMAC-SHA256 Credential=nexus-access/20260825/us-east-1/s3/aws4_request",
      ),
    ).toBe(true);
    expect(auth).toContain(
      "SignedHeaders=host;x-amz-content-sha256;x-amz-date",
    );
    expect(auth).toContain("Signature=");
  });
});

describe("ep042_unit_transport_config_fails_closed", () => {
  it("rejects an empty bucket", () => {
    expect(
      () =>
        new ReleaseTransport({
          endpoint: "127.0.0.1:8333",
          creds: { accessKey: "a", secretKey: "b" },
          bucket: "",
          runId: "run-1",
          gitCommit: "abc",
        }),
    ).toThrow(ReleaseTransportError);
  });

  it("rejects an empty run_id", () => {
    expect(
      () =>
        new ReleaseTransport({
          endpoint: "127.0.0.1:8333",
          creds: { accessKey: "a", secretKey: "b" },
          bucket: "nexus",
          runId: "",
          gitCommit: "abc",
        }),
    ).toThrow(ReleaseTransportError);
  });

  it("rejects missing credentials", () => {
    expect(
      () =>
        new S3Client({
          endpoint: "127.0.0.1:8333",
          creds: { accessKey: "", secretKey: "b" },
        }),
    ).toThrow(ReleaseTransportError);
  });

  it("rejects an empty endpoint", () => {
    expect(
      () =>
        new S3Client({
          endpoint: "",
          creds: { accessKey: "a", secretKey: "b" },
        }),
    ).toThrow(ReleaseTransportError);
  });
});

describe("ep042_unit_transport_key_layout", () => {
  it("builds canonical manifest and component keys", () => {
    expect(manifestKey("nexus-1")).toBe("releases/nexus-1/manifest.json");
    expect(componentKey("nexus-1", "nexus-core")).toBe(
      "releases/nexus-1/components/nexus-core",
    );
  });

  it("builds path-style URLs", () => {
    const url = s3Url(
      { endpoint: "127.0.0.1:8333", creds: { accessKey: "a", secretKey: "b" } },
      "nexus",
      "releases/r1/manifest.json",
    );
    expect(url.href).toBe(
      "http://127.0.0.1:8333/nexus/releases/r1/manifest.json",
    );
  });
});

describe("ep042_unit_transport_redaction", () => {
  it("redacts access-key-shaped and secret-shaped values", () => {
    const canary = "AKIA" + "0".repeat(16);
    expect(redact(`${canary} boom`)).toContain("REDACTED_ACCESS_KEY");
    expect(
      redact(
        "value=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      ),
    ).toContain("REDACTED_SECRET_SHAPE");
  });

  it("leaves plain messages unchanged", () => {
    expect(redact("probe digest mismatch")).toBe("probe digest mismatch");
  });
});

describe("ep042_unit_transport_manifest_digest_reader", () => {
  it("rejects a manifest that is not JSON", async () => {
    const t = new ReleaseTransport({
      endpoint: "127.0.0.1:8333",
      creds: { accessKey: "a", secretKey: "b" },
      bucket: "nexus",
      runId: "run-1",
      gitCommit: "abc",
    });
    await expect(
      t.publish("nexus-1", encoder.encode("not json"), []),
    ).rejects.toThrow(ReleaseTransportError);
  });

  it("rejects a manifest without components", async () => {
    const t = new ReleaseTransport({
      endpoint: "127.0.0.1:8333",
      creds: { accessKey: "a", secretKey: "b" },
      bucket: "nexus",
      runId: "run-1",
      gitCommit: "abc",
    });
    await expect(
      t.publish("nexus-1", encoder.encode('{"release_id":"x"}'), []),
    ).rejects.toThrow(ReleaseTransportError);
  });

  it("rejects a component with bytes but no declared digest", async () => {
    const t = new ReleaseTransport({
      endpoint: "127.0.0.1:8333",
      creds: { accessKey: "a", secretKey: "b" },
      bucket: "nexus",
      runId: "run-1",
      gitCommit: "abc",
    });
    const manifest = JSON.stringify({
      components: [
        {
          component_id: "nexus-core",
          digest:
            "sha256:8f0890cc919c925cb12ab3d514dcc237a417a25a04bb33f79728cc8e2ed3f26a",
        },
      ],
    });
    await expect(
      t.publish("nexus-1", encoder.encode(manifest), [
        { componentId: "other", bytes: encoder.encode("bytes") },
      ]),
    ).rejects.toThrow(ReleaseTransportError);
  });
});
