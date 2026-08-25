/**
 * EP-042 M2 content digest helper (SPEC-024 content addressing).
 *
 * Uses the platform Web Crypto API (globalThis.crypto.subtle) - no node
 * builtin imports, so the update core stays framework-neutral and pure.
 * The returned hex string is exactly 64 characters.
 */

import { ReleaseError, ReleaseErrorCode } from "./errors";
import { Digest } from "./types";

const encoder = new TextEncoder();

/** SHA-256 over bytes, returned as lowercase hex (exactly 64 chars). */
export async function sha256Hex(
  bytes: Uint8Array<ArrayBuffer>,
): Promise<string> {
  const digestBuffer = await globalThis.crypto.subtle.digest("SHA-256", bytes);
  const out = new Uint8Array(digestBuffer);
  let s = "";
  for (const b of out) {
    s += b.toString(16).padStart(2, "0");
  }
  return s;
}

/**
 * Deterministic canonical JSON bytes for a wire-shaped object. The
 * caller supplies the object with properties in canonical field order;
 * JSON.stringify preserves insertion order, so the same logical object
 * always serializes to the same bytes.
 */
export function canonicalJsonBytes(
  value: Record<string, unknown>,
): Uint8Array<ArrayBuffer> {
  return encoder.encode(JSON.stringify(value));
}

/**
 * Real sha256:hex digest over canonical JSON bytes of a wire object.
 * Rejects serialization failures (e.g. non-finite numbers) as internal
 * invariant errors.
 */
export async function contentDigest(
  value: Record<string, unknown>,
): Promise<Digest> {
  let bytes: Uint8Array<ArrayBuffer>;
  try {
    bytes = canonicalJsonBytes(value);
  } catch (error) {
    throw new ReleaseError(
      ReleaseErrorCode.InternalInvariant,
      `canonical serialization failed: ${String(error)}`,
    );
  }
  const hex = await sha256Hex(bytes);
  return Digest.parse(`sha256:${hex}`, "content digest");
}
