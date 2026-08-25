/**
 * EP-042 M4 installer barrel (SPEC-016, SPEC-024).
 *
 * Real local installer: transactional install with backup-before-update,
 * staged validation, atomic switch, rollback, quarantine, typed failure
 * classification, abuse-case path guards, append-only journal, redacted
 * observability, and bounded recovery. Canonical truth remains in
 * crates/nexus-release (M1) and apps/setup/src/update/ (M2); this
 * package is the local execution boundary.
 */

export * from "./errors";
export * from "./journal";
export * from "./paths";
export * from "./backup";
export * from "./observability";
export * from "./installer";
