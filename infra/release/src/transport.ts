/**
 * EP-042 M3 release transport orchestration (SPEC-016, SPEC-024).
 *
 * Publish and fetch release manifests + component artifacts over a real
 * S3-compatible object store with digest binding, idempotent publish,
 * readiness probe, current-run redacted audit events, and fail-closed
 * verification. Canonical release truth remains in crates/nexus-release
 * (M1) and apps/setup/src/update/ (M2); this layer transports bytes and
 * verifies digests at the boundary only.
 *
 * Invariants preserved:
 *   - DIGEST PRESENT != ARTIFACT VERIFIED (fetch verifies real bytes)
 *   - UPDATE PLAN EXISTS != UPDATE EXECUTED (transport never executes)
 *   - TRANSPORT CONFIG EXISTS != TRANSPORT EXECUTED
 */

import { ReleaseTransportError } from "./errors.ts";
import { S3Client, type S3ClientConfig } from "./s3.ts";

const encoder = new TextEncoder();

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

/** Canonical object key layout under a release prefix. */
export function manifestKey(releaseId: string): string {
  return `releases/${releaseId}/manifest.json`;
}

export function componentKey(releaseId: string, componentId: string): string {
  return `releases/${releaseId}/components/${componentId}`;
}

export interface ReleaseArtifact {
  /** Canonical component identity (matches SignedComponent.component_id). */
  componentId: string;
  /** Real bytes of the component artifact. */
  bytes: Uint8Array<ArrayBuffer>;
}

export interface PublishedRelease {
  releaseId: string;
  manifestDigest: string;
  componentDigests: ReadonlyArray<{ componentId: string; digest: string }>;
}

export interface FetchedRelease {
  releaseId: string;
  manifestBytes: Uint8Array<ArrayBuffer>;
  manifestDigest: string;
  components: ReadonlyArray<ReleaseArtifact>;
}

export interface AuditEvent {
  run_id: string;
  git_commit: string;
  release_id: string;
  op: string;
  outcome: "ok" | "denied";
  detail: string;
  ts: string;
}

export interface ProbeResult {
  healthz: boolean;
  probe_verified: boolean;
  detail: string;
}

export interface ReleaseTransportConfig extends S3ClientConfig {
  /** Bucket that holds release artifacts. */
  bucket: string;
  /** Current-run identifier bound into audit events. */
  runId: string;
  /** Git commit bound into audit events. */
  gitCommit: string;
}

/** Redact secret-shaped values from any string before it reaches logs. */
export function redact(value: string): string {
  return value
    .replace(/AKIA[0-9A-Z]{16}/g, "REDACTED_ACCESS_KEY")
    .replace(/[A-Za-z0-9+/=]{40,}/g, "REDACTED_SECRET_SHAPE");
}

/** Scrub the exact configured credential values plus generic shapes. */
function redactWithCredentials(
  value: string,
  creds: { accessKey: string; secretKey: string },
): string {
  let out = redact(value);
  if (creds.accessKey.length > 0) {
    out = out.split(creds.accessKey).join("REDACTED_ACCESS_KEY");
  }
  if (creds.secretKey.length > 0) {
    out = out.split(creds.secretKey).join("REDACTED_SECRET_KEY");
  }
  return out;
}

export class ReleaseTransport {
  readonly config: ReleaseTransportConfig;
  private readonly client: S3Client;

  constructor(config: ReleaseTransportConfig) {
    if (config.bucket.trim().length === 0) {
      throw new ReleaseTransportError(
        "CONFIG_MISSING",
        "release bucket is not configured",
      );
    }
    if (config.runId.trim().length === 0) {
      throw new ReleaseTransportError(
        "CONFIG_MISSING",
        "run_id is not configured",
      );
    }
    this.config = config;
    this.client = new S3Client(config);
  }

  /** Real readiness: healthz AND a probe PUT -> GET -> digest -> DELETE. */
  async probe(signal?: AbortSignal): Promise<ProbeResult> {
    const healthz = await this.client.healthz();
    if (!healthz) {
      return {
        healthz: false,
        probe_verified: false,
        detail: "provider health endpoint not ready",
      };
    }
    const probeKey = `probes/${this.config.runId}`;
    const probeBytes = encoder.encode(
      `nexus-release-probe-${this.config.runId}`,
    );
    try {
      await this.client.createBucket(this.config.bucket, signal);
      await this.client.putObject(
        this.config.bucket,
        probeKey,
        probeBytes,
        signal,
      );
      const fetched = await this.client.getObject(
        this.config.bucket,
        probeKey,
        signal,
      );
      const digest = await sha256Hex(fetched.bytes);
      const expected = await sha256Hex(probeBytes);
      if (digest !== expected) {
        return {
          healthz: true,
          probe_verified: false,
          detail: "probe digest mismatch",
        };
      }
      await this.client.deleteObject(this.config.bucket, probeKey, signal);
      return {
        healthz: true,
        probe_verified: true,
        detail: "probe PUT/GET/digest/DELETE verified",
      };
    } catch (err) {
      return {
        healthz: true,
        probe_verified: false,
        detail: redact(err instanceof Error ? err.message : "probe failed"),
      };
    }
  }

  /**
   * Publish a release manifest + component artifacts. Digest binding is
   * verified BEFORE any upload: every component's declared digest must
   * match real bytes, otherwise nothing is published (fail closed).
   * Idempotent: re-publishing identical bytes lands on identical keys.
   */
  async publish(
    releaseId: string,
    manifestBytes: Uint8Array<ArrayBuffer>,
    components: ReadonlyArray<ReleaseArtifact>,
    signal?: AbortSignal,
  ): Promise<PublishedRelease> {
    const declared = this.extractDeclaredDigests(manifestBytes);
    const actual = new Map<string, string>();
    for (const comp of components) {
      actual.set(comp.componentId, await sha256Hex(comp.bytes));
    }
    for (const comp of components) {
      const declaredDigest = declared.get(comp.componentId);
      if (!declaredDigest) {
        throw new ReleaseTransportError(
          "DIGEST_MISMATCH",
          `component ${comp.componentId} has bytes but no declared digest`,
        );
      }
      const actualDigest = actual.get(comp.componentId);
      if (actualDigest !== declaredDigest) {
        throw new ReleaseTransportError(
          "DIGEST_MISMATCH",
          `component ${comp.componentId} digest mismatch (declared != computed)`,
        );
      }
    }
    await this.client.createBucket(this.config.bucket, signal);
    await this.client.putObject(
      this.config.bucket,
      manifestKey(releaseId),
      manifestBytes,
      signal,
    );
    const componentDigests: Array<{ componentId: string; digest: string }> = [];
    for (const comp of components) {
      await this.client.putObject(
        this.config.bucket,
        componentKey(releaseId, comp.componentId),
        comp.bytes,
        signal,
      );
      componentDigests.push({
        componentId: comp.componentId,
        digest: actual.get(comp.componentId) ?? "",
      });
    }
    return {
      releaseId,
      manifestDigest: await sha256Hex(manifestBytes),
      componentDigests,
    };
  }

  /**
   * Fetch a release manifest + all components and verify digests against
   * real fetched bytes. Any mismatch or missing object fails closed.
   */
  async fetch(
    releaseId: string,
    componentIds: ReadonlyArray<string>,
    signal?: AbortSignal,
  ): Promise<FetchedRelease> {
    const manifestRes = await this.client.getObject(
      this.config.bucket,
      manifestKey(releaseId),
      signal,
    );
    const manifestBytes = manifestRes.bytes;
    const declared = this.extractDeclaredDigests(manifestBytes);
    const components: ReleaseArtifact[] = [];
    for (const componentId of componentIds) {
      const declaredDigest = declared.get(componentId);
      if (!declaredDigest) {
        throw new ReleaseTransportError(
          "DIGEST_MISMATCH",
          `component ${componentId} missing from manifest digest map`,
        );
      }
      const res = await this.client.getObject(
        this.config.bucket,
        componentKey(releaseId, componentId),
        signal,
      );
      const computed = await sha256Hex(res.bytes);
      if (computed !== declaredDigest) {
        throw new ReleaseTransportError(
          "DIGEST_MISMATCH",
          `component ${componentId} digest mismatch (computed != declared)`,
        );
      }
      components.push({ componentId, bytes: res.bytes });
    }
    return {
      releaseId,
      manifestBytes,
      manifestDigest: await sha256Hex(manifestBytes),
      components,
    };
  }

  /** Head a single object (used by integration proofs). */
  async head(
    releaseId: string,
    componentId: string,
    signal?: AbortSignal,
  ): Promise<number> {
    const meta = await this.client.headObject(
      this.config.bucket,
      componentKey(releaseId, componentId),
      signal,
    );
    return meta.size;
  }

  /** Current-run audit event, redacted before it can be emitted. */
  audit(
    releaseId: string,
    op: string,
    outcome: "ok" | "denied",
    detail: string,
  ): AuditEvent {
    return {
      run_id: this.config.runId,
      git_commit: this.config.gitCommit,
      release_id: releaseId,
      op,
      outcome,
      detail: redactWithCredentials(detail, this.config.creds),
      ts: new Date().toISOString(),
    };
  }

  /**
   * Minimal canonical manifest digest map reader. The manifest is the
   * M1 ReleaseManifest wire JSON; this reads only the component digest
   * map needed for transport binding. Full semantic validation remains
   * in crates/nexus-release / apps/setup update core (M1/M2).
   */
  private extractDeclaredDigests(
    manifestBytes: Uint8Array<ArrayBuffer>,
  ): Map<string, string> {
    let parsed: unknown;
    try {
      parsed = JSON.parse(new TextDecoder().decode(manifestBytes));
    } catch (err) {
      throw new ReleaseTransportError(
        "MALFORMED_RESPONSE",
        "manifest is not valid JSON",
        { cause: err },
      );
    }
    const obj = parsed as {
      components?: Array<{ component_id?: unknown; digest?: unknown }>;
    };
    if (!Array.isArray(obj?.components)) {
      throw new ReleaseTransportError(
        "MALFORMED_RESPONSE",
        "manifest missing components array",
      );
    }
    const map = new Map<string, string>();
    for (const entry of obj.components) {
      const id = entry?.component_id;
      const digest = entry?.digest;
      if (typeof id !== "string" || id.length === 0) {
        throw new ReleaseTransportError(
          "MALFORMED_RESPONSE",
          "manifest component missing component_id",
        );
      }
      if (typeof digest !== "string") {
        throw new ReleaseTransportError(
          "MALFORMED_RESPONSE",
          `manifest component ${id} missing digest`,
        );
      }
      // Canonical M1 digest form is "sha256:<64 lowercase hex>".
      const hexPart = digest.startsWith("sha256:")
        ? digest.slice("sha256:".length)
        : digest;
      if (!/^[0-9a-f]{64}$/.test(hexPart)) {
        throw new ReleaseTransportError(
          "MALFORMED_RESPONSE",
          `manifest component ${id} missing valid sha256 digest`,
        );
      }
      map.set(id, hexPart);
    }
    return map;
  }
}
