/**
 * EP-042 M2 update core barrel (SPEC-016, SPEC-024).
 *
 * Deterministic, pure, fail-closed update behavior: manifest validation,
 * compatibility evaluation, update planning, backup-before-update policy,
 * rollback preconditions, canary/manual promotion gate, and current-run
 * redacted evidence. Canonical truth remains in crates/nexus-release
 * (M1); this surface adapts the canonical wire contracts at the boundary.
 */

export * from "./errors";
export * from "./types";
export * from "./digest";
export * from "./manifest";
export * from "./compatibility";
export * from "./planner";
export * from "./backup";
export * from "./rollback";
export * from "./canary";
export * from "./evidence";
