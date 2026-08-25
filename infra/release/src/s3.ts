/**
 * EP-042 M3 minimal real S3 client over global fetch (SPEC-024).
 *
 * Path-style requests against an S3-compatible gateway (SeaweedFS S3
 * gateway :8333). Every request is signed with real AWS SigV4 over Web
 * Crypto. Supports the exact object-store surface the release transport
 * needs: createBucket, putObject, getObject, headObject, deleteObject,
 * listObjects, and the provider health endpoint. Timeout and
 * cancellation are real: an AbortController bounds every request, and
 * a caller-supplied signal cancels in-flight work.
 *
 * No node builtin imports: runs in Node (type-stripped CLI) and in
 * vitest via global fetch + Web Crypto.
 */

import { ReleaseTransportError } from "./errors.ts";
import {
  assertCredentialsConfigured,
  signRequest,
  type SigV4Credentials,
} from "./sigv4.ts";

export interface S3ClientConfig {
  /** host:port of the S3 gateway, e.g. "127.0.0.1:8333" */
  endpoint: string;
  creds: SigV4Credentials;
  /** SigV4 region (SeaweedFS accepts any; canonical "us-east-1"). */
  region?: string;
  /** Request timeout in milliseconds. */
  timeoutMs?: number;
  /** Whether the endpoint is https (default false for local fixtures). */
  tls?: boolean;
}

export interface S3ObjectMeta {
  key: string;
  size: number;
}

const DEFAULT_TIMEOUT_MS = 10_000;

/** Build a path-style URL for bucket/key. */
export function s3Url(cfg: S3ClientConfig, bucket: string, key: string): URL {
  const scheme = cfg.tls ? "https" : "http";
  const keyPart = key.startsWith("/") ? key.slice(1) : key;
  const path = keyPart.length > 0 ? `/${bucket}/${keyPart}` : `/${bucket}`;
  return new URL(`${scheme}://${cfg.endpoint}${path}`);
}

export class S3Client {
  readonly config: S3ClientConfig;

  constructor(config: S3ClientConfig) {
    assertCredentialsConfigured(config.creds);
    if (config.endpoint.trim().length === 0) {
      throw new ReleaseTransportError(
        "CONFIG_MISSING",
        "s3 endpoint is not configured",
      );
    }
    this.config = {
      region: "us-east-1",
      timeoutMs: DEFAULT_TIMEOUT_MS,
      tls: false,
      ...config,
    };
  }

  private async request(
    method: string,
    bucket: string,
    key: string,
    body: Uint8Array<ArrayBuffer>,
    query: string,
    signal?: AbortSignal,
  ): Promise<{
    status: number;
    headers: Headers;
    bytes: Uint8Array<ArrayBuffer>;
  }> {
    const url = s3Url(this.config, bucket, key);
    url.search = "";
    const host = url.host;
    const path = url.pathname;

    const preHeaders: Record<string, string> = {};
    const signed = await signRequest(
      method,
      host,
      path,
      query,
      preHeaders,
      body,
      this.config.creds,
      { region: this.config.region ?? "us-east-1", service: "s3" },
    );
    // The request URL must carry exactly the canonical query that was
    // signed. The URL.search setter would re-encode an already-encoded
    // query (double-encoding %2F), so build the href from the canonical
    // query string directly.
    const scheme = this.config.tls ? "https" : "http";
    const href = `${scheme}://${host}${path}${
      signed.canonicalQuery.length > 0 ? `?${signed.canonicalQuery}` : ""
    }`;

    const controller = new AbortController();
    const timeoutMs = this.config.timeoutMs ?? DEFAULT_TIMEOUT_MS;
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    const onOuterAbort = () => controller.abort();
    if (signal) {
      if (signal.aborted) controller.abort();
      else signal.addEventListener("abort", onOuterAbort, { once: true });
    }

    try {
      const hasBody = !(
        method === "GET" ||
        method === "HEAD" ||
        method === "DELETE"
      );
      const init: RequestInit = {
        method,
        headers: { ...signed.headers },
        signal: controller.signal,
      };
      if (hasBody) init.body = body as Uint8Array<ArrayBuffer>;
      const res = await fetch(href, init);
      const bytes =
        method === "HEAD"
          ? new Uint8Array(0)
          : new Uint8Array(await res.arrayBuffer());
      return { status: res.status, headers: res.headers, bytes };
    } catch (err) {
      if (signal?.aborted || controller.signal.aborted) {
        if (signal?.aborted) {
          throw new ReleaseTransportError(
            "CANCELLED",
            "request cancelled by caller",
            { cause: err },
          );
        }
        throw new ReleaseTransportError("TIMEOUT", "request timed out", {
          cause: err,
        });
      }
      throw new ReleaseTransportError("UNREACHABLE", "request failed", {
        cause: err,
      });
    } finally {
      clearTimeout(timer);
      if (signal) signal.removeEventListener("abort", onOuterAbort);
    }
  }

  async healthz(): Promise<boolean> {
    const scheme = this.config.tls ? "https" : "http";
    const controller = new AbortController();
    const timer = setTimeout(
      () => controller.abort(),
      this.config.timeoutMs ?? DEFAULT_TIMEOUT_MS,
    );
    try {
      const res = await fetch(`${scheme}://${this.config.endpoint}/healthz`, {
        signal: controller.signal,
      });
      return res.ok;
    } catch {
      return false;
    } finally {
      clearTimeout(timer);
    }
  }

  async createBucket(bucket: string, signal?: AbortSignal): Promise<void> {
    const res = await this.request(
      "PUT",
      bucket,
      "",
      new Uint8Array(0),
      "",
      signal,
    );
    if (res.status !== 200 && res.status !== 409) {
      throw new ReleaseTransportError("BUCKET_ERROR", "createBucket failed", {
        status: res.status,
      });
    }
  }

  async putObject(
    bucket: string,
    key: string,
    bytes: Uint8Array<ArrayBuffer>,
    signal?: AbortSignal,
  ): Promise<void> {
    const res = await this.request("PUT", bucket, key, bytes, "", signal);
    if (res.status !== 200) {
      throw new ReleaseTransportError("HTTP_ERROR", "putObject failed", {
        status: res.status,
      });
    }
  }

  async getObject(
    bucket: string,
    key: string,
    signal?: AbortSignal,
  ): Promise<{ bytes: Uint8Array<ArrayBuffer>; size: number }> {
    const res = await this.request(
      "GET",
      bucket,
      key,
      new Uint8Array(0),
      "",
      signal,
    );
    if (res.status === 404) {
      throw new ReleaseTransportError("MISSING_OBJECT", "object not found", {
        status: 404,
      });
    }
    if (res.status !== 200) {
      throw new ReleaseTransportError("HTTP_ERROR", "getObject failed", {
        status: res.status,
      });
    }
    return { bytes: res.bytes, size: res.bytes.byteLength };
  }

  async headObject(
    bucket: string,
    key: string,
    signal?: AbortSignal,
  ): Promise<S3ObjectMeta> {
    const res = await this.request(
      "HEAD",
      bucket,
      key,
      new Uint8Array(0),
      "",
      signal,
    );
    if (res.status === 404) {
      throw new ReleaseTransportError("MISSING_OBJECT", "object not found", {
        status: 404,
      });
    }
    if (res.status !== 200) {
      throw new ReleaseTransportError("HTTP_ERROR", "headObject failed", {
        status: res.status,
      });
    }
    const sizeRaw = res.headers.get("content-length");
    return {
      key,
      size: sizeRaw === null ? 0 : Number.parseInt(sizeRaw, 10),
    };
  }

  async deleteObject(
    bucket: string,
    key: string,
    signal?: AbortSignal,
  ): Promise<void> {
    const res = await this.request(
      "DELETE",
      bucket,
      key,
      new Uint8Array(0),
      "",
      signal,
    );
    if (res.status !== 204 && res.status !== 200) {
      throw new ReleaseTransportError("HTTP_ERROR", "deleteObject failed", {
        status: res.status,
      });
    }
  }

  async listObjects(
    bucket: string,
    prefix: string,
    signal?: AbortSignal,
  ): Promise<ReadonlyArray<string>> {
    // Pass the RAW query; signRequest applies canonical SigV4 encoding
    // (uriEncode with encodeSlash) to keys and values exactly once.
    const query = `list-type=2&prefix=${prefix}`;
    const res = await this.request(
      "GET",
      bucket,
      "",
      new Uint8Array(0),
      query,
      signal,
    );
    if (res.status !== 200) {
      throw new ReleaseTransportError("HTTP_ERROR", "listObjects failed", {
        status: res.status,
      });
    }
    const text = new TextDecoder().decode(res.bytes);
    const keys: string[] = [];
    const re = /<Key>([^<]+)<\/Key>/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(text)) !== null) {
      keys.push(m[1] ?? "");
    }
    return keys;
  }
}
