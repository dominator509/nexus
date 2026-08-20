/**
 * @nexus/desktop - EP-033 M2 desktop shell core behavior.
 *
 * Deterministic, framework-neutral desktop shell: runtime state
 * machine, typed command dispatcher, approval flow, view-state
 * composition, telemetry, and preferences. All behavior imports the
 * shared @nexus/web contracts; no business logic is duplicated
 * (acceptance obligation 2). The Tauri wrapper remains a thin signed
 * shell (EP-033 fallback).
 */

export { DesktopShellRuntime } from "./runtime";
export type { DesktopRuntimeSnapshot } from "./runtime";

export { DesktopCommandDispatcher, DesktopTelemetry } from "./dispatcher";
export type { DispatchResult, DesktopTelemetryEntry } from "./dispatcher";

export { DesktopApprovalFlow } from "./approvals";
export type { ApprovalProgression } from "./approvals";

export { DesktopViewState } from "./viewstate";
export type { DesktopViewComposition } from "./viewstate";

export { DesktopPreferences } from "./prefs";
