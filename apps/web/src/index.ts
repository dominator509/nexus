/**
 * @nexus/web - EP-033 M1 web dashboard contract surface.
 *
 * Provider-neutral, framework-neutral contracts for the dashboard
 * shell and its eight public interfaces. This package is the shared
 * contract root for the React PWA (apps/web) and the Tauri desktop
 * shell (apps/desktop, EP-033 M2): platform UI may differ, authority
 * and state semantics may not (directive H).
 */

// Canonical SPEC-006 error vocabulary.
export { ErrorCode, Spec006Error, classifyError } from "./contracts/errors";
export type { ProblemDetails } from "./contracts/errors";

// Validation primitives (internal, exported for contract authors).
export {
  assertObject,
  rejectUnknownFields,
  assertString,
  assertUuid,
  assertEnum,
  assertInt,
  assertBool,
  assertStringSet,
} from "./contracts/validate";

// Session and business context.
export {
  AuthenticatedSession,
  BusinessContext,
  BoundContext,
  ContextProjection,
  SessionStatus,
  GRANT_FLOWS,
  AUTH_STRENGTHS,
  BUSINESS_SCOPES,
} from "./contracts/session";
export type { AuthSessionShape, BusinessContextShape, GrantFlow, AuthStrength, BusinessScope } from "./contracts/session";

// UI state vocabulary.
export {
  ViewState,
  revalidated,
  UI_STATE_KINDS,
  CONNECTIVITY_STATES,
  DATA_FRESHNESS,
} from "./contracts/state";
export type { UiStateKind, ConnectivityState, DataFreshness } from "./contracts/state";

// Capability presentation.
export {
  PresentedCapability,
  KnownCapabilityVocabulary,
  CapabilityPresentation,
  CAPABILITY_CLASSES,
  CAPABILITY_AVAILABILITY,
} from "./contracts/capability";
export type { PresentedCapabilityShape, CapabilityClass, CapabilityAvailability } from "./contracts/capability";

// Typed command dispatch.
export {
  TypedCommandRequest,
  DispatchGate,
  refuseOrPass,
  lifecycleClaims,
  riskToNumber,
  RISK_CLASSES,
  APPROVAL_CLASSES,
  ACTION_LIFECYCLE,
} from "./contracts/command";
export type { RiskClass, ApprovalClass, ActionLifecycleStage, CommandRequestShape, MutationRefusal } from "./contracts/command";

// Approvals.
export {
  ApprovalCard,
  ApprovalAction,
  FourEyesRecord,
  APPROVAL_STATES,
  APPROVAL_ACTIONS,
} from "./contracts/approval-center";
export type { ApprovalState, ApprovalActionKind, ApprovalCardShape } from "./contracts/approval-center";

// Event subscriptions.
export { EventFilter, EventSubscription } from "./contracts/events";
export type { EventFilterShape } from "./contracts/events";

// Preferences and persistence boundary.
export {
  PreferencePersistence,
  ThemePreference,
  THEME_MODES,
  ALLOWED_PREFERENCE_KEYS,
  FORBIDDEN_PREFERENCE_KEYS,
} from "./contracts/preferences";
export type { ThemeMode, PersistenceDecision } from "./contracts/preferences";

// Accessibility contracts.
export {
  A11ySurface,
  FocusOrder,
  assertReducedMotionSafe,
  assertNonColorStatus,
  A11Y_ROLES,
} from "./contracts/accessibility";
export type { A11yRole, A11ySurfaceShape } from "./contracts/accessibility";

// Redacted diagnostics.
export { RedactedLogEntry, RedactedLogger, redact } from "./contracts/logging";
export type { RedactedLogEntryShape } from "./contracts/logging";

// The eight public dashboard interfaces.
export {
  DashboardShell,
  DASHBOARD_ROUTES,
  DASHBOARD_SURFACES,
} from "./contracts/dashboard-shell";
export type { DashboardRoute, DashboardSurface, DashboardShellShape } from "./contracts/dashboard-shell";

export { ChatMessage, ChatWorkspace, CHAT_ORIGINS, MESSAGE_DIRECTIONS } from "./contracts/chat-workspace";
export type { ChatMessageShape, ChatOrigin, MessageDirection } from "./contracts/chat-workspace";

export { ObjectiveView, TaskNode, OBJECTIVE_STAGES } from "./contracts/objective-view";
export type { ObjectiveStage, ObjectiveShape } from "./contracts/objective-view";

export { FleetDevice, FleetView, DEVICE_STATUSES } from "./contracts/fleet-view";
export type { DeviceStatus, FleetDeviceShape } from "./contracts/fleet-view";

export {
  SecurityIncident,
  SecurityConsole,
  SEVERITY_LEVELS,
  INCIDENT_STATUSES,
} from "./contracts/security-console";
export type { SeverityLevel, IncidentStatus, IncidentShape } from "./contracts/security-console";

export {
  ProviderDisclosure,
  ProviderSettings,
  PROVIDER_ROUTES,
  PROVIDER_CERTIFICATION,
} from "./contracts/provider-settings";
export type { ProviderRoute, ProviderCertification, ProviderDisclosureShape } from "./contracts/provider-settings";

export { AuditRecord, AuditFilter, AuditExplorer } from "./contracts/audit-explorer";
export type { AuditRecordShape } from "./contracts/audit-explorer";
