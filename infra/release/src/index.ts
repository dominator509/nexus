/**
 * EP-042 M3 release transport infrastructure barrel (SPEC-016,
 * SPEC-024).
 *
 * Real SigV4 S3 transport (Web Crypto + global fetch) for release
 * manifests and component artifacts: digest-bound publish/fetch,
 * readiness probe, idempotent publish, timeout/cancellation,
 * current-run redacted audit events, and fail-closed config
 * validation. Canonical release truth remains in crates/nexus-release
 * (M1) and apps/setup/src/update/ (M2); this is the transport boundary.
 */

export * from "./errors.ts";
export * from "./sigv4.ts";
export * from "./s3.ts";
export * from "./transport.ts";
