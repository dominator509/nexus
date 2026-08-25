/**
 * @nexus/setup public barrel (EP-035 M1).
 *
 * Exposes the eight Setup Wizard and Onboarding contract surfaces plus
 * the shared SPEC-006 error and validation vocabulary. Consumers import
 * from this barrel only; src/ paths are package-internal.
 */

export * from "./contracts/errors";
export * from "./contracts/validate";
export * from "./contracts/wizard";
export * from "./contracts/deployment";
export * from "./contracts/hardware";
export * from "./contracts/owner";
export * from "./contracts/enrollment";
export * from "./contracts/discovery";
export * from "./contracts/integration";
export * from "./contracts/recovery";
export * from "./update";
