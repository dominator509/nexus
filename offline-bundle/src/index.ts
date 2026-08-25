/**
 * EP-042 M5 offline bundle barrel (SPEC-016 behavior 5, SPEC-024).
 *
 * Real offline bundle production, digest-bound verification, offline
 * installation composing the M4 transactional installer (no transport),
 * rollback drill, and current-run redacted evidence. Canonical truth
 * remains in crates/nexus-release (M1), apps/setup/src/update/ (M2),
 * infra/release/ (M3), installers/ (M4); this package is the offline
 * distribution + verification + installation boundary.
 */

export * from "./errors";
export * from "./model";
export * from "./produce";
export * from "./verify";
export * from "./install";
export * from "./rollback";
export * from "./evidence";
