/**
 * StatusBadge - EP-033 M3 shared UI component.
 *
 * Renders connectivity/freshness with non-color status signaling
 * (directive I/J/Q): the state is conveyed by text and aria-label,
 * never by color alone. A STALE label is always rendered with the
 * data; stale is never presented as live.
 */

import type { ConnectivityState, DataFreshness } from "@nexus/web";

export interface StatusBadgeProps {
  connectivity: ConnectivityState;
  freshness: DataFreshness;
}

export function StatusBadge(props: StatusBadgeProps): React.ReactElement {
  const { connectivity, freshness } = props;
  return (
    <span
      role="status"
      aria-live="polite"
      data-connectivity={connectivity}
      data-freshness={freshness}
    >
      {connectivity}
      {freshness === "STALE" ? " (stale)" : ""}
    </span>
  );
}
