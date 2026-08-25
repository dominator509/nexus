/**
 * EP-042 M3 real AWS SigV4 request signing (SPEC-024 S3 transport).
 *
 * Implements the AWS Signature Version 4 algorithm over Web Crypto
 * (globalThis.crypto.subtle HMAC-SHA256). This is REAL cryptographic
 * request signing: canonical request, string-to-sign, signing key
 * derivation, and Authorization header assembly per the AWS SigV4
 * spec. No SDK is involved; the exact exercised local surface is
 * certified by the EP-042 M3 integration suite against a real
 * SeaweedFS S3 gateway.
 *
 * No node builtin imports: runs in Node (type-stripped CLI) and in
 * vitest via global Web Crypto + fetch.
 */

import { ReleaseTransportError } from "./errors.ts";

const encoder = new TextEncoder();

async function hmacSha256(
  key: Uint8Array<ArrayBuffer>,
  data: Uint8Array<ArrayBuffer>,
): Promise<Uint8Array<ArrayBuffer>> {
  const keyBuf = await globalThis.crypto.subtle.importKey(
    "raw",
    key as Uint8Array<ArrayBuffer>,
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const sig = await globalThis.crypto.subtle.sign(
    "HMAC",
    keyBuf,
    data as Uint8Array<ArrayBuffer>,
  );
  return new Uint8Array(sig);
}

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

function hex(bytes: Uint8Array<ArrayBuffer>): string {
  let s = "";
  for (const b of bytes) {
    s += b.toString(16).padStart(2, "0");
  }
  return s;
}

/** Percent-encode per RFC 3986 (SigV4 canonical URI/query rules). */
export function uriEncode(value: string, encodeSlash = false): string {
  let out = "";
  for (const ch of value) {
    const code = ch.charCodeAt(0);
    if (
      (code >= 0x41 && code <= 0x5a) || // A-Z
      (code >= 0x61 && code <= 0x7a) || // a-z
      (code >= 0x30 && code <= 0x39) || // 0-9
      ch === "-" ||
      ch === "_" ||
      ch === "." ||
      ch === "~"
    ) {
      out += ch;
    } else if (ch === "/" && !encodeSlash) {
      out += ch;
    } else {
      const bytes = encoder.encode(ch);
      for (const b of bytes) {
        out += "%" + b.toString(16).toUpperCase().padStart(2, "0");
      }
    }
  }
  return out;
}

export interface SigV4Credentials {
  accessKey: string;
  secretKey: string;
}

export interface SigV4SigningContext {
  region: string;
  service: string;
}

export interface SigV4SignedRequest {
  headers: Record<string, string>;
  /** Canonical request hex digest (x-amz-content-sha256 for bodyless). */
  payloadHash: string;
  /** Canonical (encoded + sorted) query string used in the signature. */
  canonicalQuery: string;
}

/**
 * Sign an HTTP request per AWS SigV4.
 *
 * @param method  HTTP method (GET/PUT/HEAD/DELETE)
 * @param host    host[:port] from the request URL
 * @param path    URL path (path-style, e.g. /bucket/key)
 * @param query   URL query string without leading '?'
 * @param headers headers to sign (host, x-amz-date, x-amz-content-sha256)
 * @param body    request body bytes (empty for GET/HEAD/DELETE)
 * @param creds   runtime credentials
 * @param ctx     region + service
 * @param now     optional Date (injected for deterministic tests)
 */
export async function signRequest(
  method: string,
  host: string,
  path: string,
  query: string,
  headers: Record<string, string>,
  body: Uint8Array<ArrayBuffer>,
  creds: SigV4Credentials,
  ctx: SigV4SigningContext,
  now?: Date,
): Promise<SigV4SignedRequest> {
  const date = now ?? new Date();
  const amzDate = date.toISOString().replace(/[:-]|\.\d{3}/g, "");
  const dateStamp = amzDate.slice(0, 8);

  const payloadHash = await sha256Hex(body);

  const signedHeaders = ["host", "x-amz-content-sha256", "x-amz-date"];
  const canonicalHeaders =
    `host:${host}\n` +
    `x-amz-content-sha256:${payloadHash}\n` +
    `x-amz-date:${amzDate}\n`;

  const canonicalQuery = query
    .split("&")
    .filter((part) => part.length > 0)
    .map((part) => {
      const idx = part.indexOf("=");
      if (idx === -1) return `${uriEncode(part, true)}=`;
      return `${uriEncode(part.slice(0, idx), true)}=${uriEncode(
        part.slice(idx + 1),
        true,
      )}`;
    })
    .sort()
    .join("&");

  const canonicalUri = path
    .split("/")
    .map((seg) => uriEncode(seg))
    .join("/");

  const canonicalRequest =
    `${method}\n` +
    `${canonicalUri}\n` +
    `${canonicalQuery}\n` +
    `${canonicalHeaders}\n` +
    `${signedHeaders.join(";")}\n` +
    `${payloadHash}`;

  const scope = `${dateStamp}/${ctx.region}/${ctx.service}/aws4_request`;
  const stringToSign =
    "AWS4-HMAC-SHA256\n" +
    `${amzDate}\n` +
    `${scope}\n` +
    `${await sha256Hex(encoder.encode(canonicalRequest))}`;

  const kDate = await hmacSha256(
    encoder.encode(`AWS4${creds.secretKey}`),
    encoder.encode(dateStamp),
  );
  const kRegion = await hmacSha256(kDate, encoder.encode(ctx.region));
  const kService = await hmacSha256(kRegion, encoder.encode(ctx.service));
  const kSigning = await hmacSha256(kService, encoder.encode("aws4_request"));
  const signature = hex(
    await hmacSha256(kSigning, encoder.encode(stringToSign)),
  );

  const authorization =
    `AWS4-HMAC-SHA256 Credential=${creds.accessKey}/${scope}, ` +
    `SignedHeaders=${signedHeaders.join(";")}, Signature=${signature}`;

  return {
    headers: {
      "x-amz-date": amzDate,
      "x-amz-content-sha256": payloadHash,
      host,
      authorization,
      ...headers,
    },
    payloadHash,
    canonicalQuery,
  };
}

/** Validate that runtime credentials are non-empty before signing. */
export function assertCredentialsConfigured(creds: SigV4Credentials): void {
  if (
    creds.accessKey.trim().length === 0 ||
    creds.secretKey.trim().length === 0
  ) {
    throw new ReleaseTransportError(
      "CONFIG_MISSING",
      "transport credentials are not configured",
    );
  }
}
