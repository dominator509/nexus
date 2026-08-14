// Nexus TypeScript connector SDK (EP-011 M2).
//
// The SPEC-022 shared contract corpus: capability discovery, typed
// query, idempotent command, health observation, and change-feed
// access over the generated canonical bindings. This surface mirrors
// the Rust `ConnectorSdk` trait (crates/nexus-connector-sdk) so the
// same conformance corpus can run against both implementations.
//
// The SDK never grants authority: it resolves through the canonical
// capability contract; authorization remains EP-008's boundary and
// discovery results are metadata only.

export * from "./error.js";
export * from "./sdk.js";
export * from "./vocabulary.js";

/** Shared contract corpus version (SPEC-022 behavior 4). */
export const CONTRACT_VERSION = "1.0.0";
