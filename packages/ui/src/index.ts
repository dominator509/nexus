/**
 * @nexus/ui - EP-033 M3 shared React UI components.
 *
 * Real React 19 components over the @nexus/web contract vocabulary,
 * shared by the web PWA and the desktop shell. Components render
 * contract state; they never mint authority.
 */

export { CapabilityButton } from "./components/capability-button";
export type { CapabilityButtonProps } from "./components/capability-button";

export { ApprovalCardView } from "./components/approval-card-view";
export type { ApprovalCardViewProps } from "./components/approval-card-view";

export { StatusBadge } from "./components/status-badge";
export type { StatusBadgeProps } from "./components/status-badge";

export { ChatComposer } from "./components/chat-composer";
export type { ChatComposerProps } from "./components/chat-composer";

export { DashboardShellView } from "./components/dashboard-shell-view";
export type { DashboardShellViewProps } from "./components/dashboard-shell-view";
