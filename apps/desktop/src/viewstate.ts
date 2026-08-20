/**
 * EP-033 M2 desktop view-state composition (deterministic core).
 *
 * Composes the shared ViewState vocabulary with connectivity and
 * freshness so the shell never renders stale cached data as live and
 * never allows a consequential action on stale data (directive J).
 */

import {
  ViewState,
  revalidated,
  ErrorCode,
  Spec006Error,
  type ConnectivityState,
  type UiStateKind,
} from "@nexus/web";

export interface DesktopViewComposition<T> {
  view: ViewState<T>;
  connectivity: ConnectivityState;
  /** True only when the view is FRESH and CONNECTED/DEGRADED. */
  actionable: boolean;
}

export class DesktopViewState {
  /**
   * Compose a fetched payload into an actionable-or-labeled view.
   * When connectivity is OFFLINE/BACKEND_UNAVAILABLE the payload is
   * labeled STALE and is never actionable.
   */
  static compose<T>(
    payload: T,
    correlation: string,
    connectivity: ConnectivityState,
    opts: { observedAt?: number; revision?: number } = {},
  ): DesktopViewComposition<T> {
    const view = ViewState.success(payload, correlation, {
      observedAt: opts.observedAt ?? Date.now(),
      revision: opts.revision ?? 1,
      connectivity,
    });
    const actionable = connectivity === "CONNECTED" || connectivity === "DEGRADED";
    if (actionable) {
      return { view, connectivity, actionable: true };
    }
    const stale = ViewState.fromWire({
      kind: "DEGRADED" as UiStateKind,
      connectivity,
      freshness: "STALE",
      observed_at_unix_ms: opts.observedAt ?? Date.now(),
      revision: opts.revision ?? 0,
      correlation,
    }) as ViewState<T>;
    return { view: stale, connectivity, actionable: false };
  }

  /**
   * Consequential-action gate on a composed view: requires FRESH data
   * and CONNECTED/DEGRADED connectivity.
   */
  static requireActionable<T>(composition: DesktopViewComposition<T>, action: string): T {
    if (!composition.actionable) {
      throw new Spec006Error(
        ErrorCode.Unavailable,
        `${action} refused: backend unavailable`,
        composition.view.correlation,
      );
    }
    return composition.view.requireFresh(action);
  }

  /** Monotonic revalidation: newer revision required. */
  static revalidate<T>(previous: ViewState<T>, next: ViewState<T>): ViewState<T> {
    return revalidated(previous, next);
  }
}
