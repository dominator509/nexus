/**
 * EP-043 M1 production readiness and ship barrel (SPEC-008).
 *
 * Provider-neutral versioned contracts for the final ship gate, release
 * evidence index, manual deploy handoff, and production readiness
 * decision. Canonical release/update/install truth remains in
 * crates/nexus-release (EP-042 M1), apps/setup/src/update/ (EP-042 M2),
 * infra/release/ (EP-042 M3), installers/ (EP-042 M4), offline-bundle/
 * (EP-042 M5); this package is the ship certification boundary.
 */

export * from "./errors";
export * from "./model";
