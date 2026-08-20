/**
 * EP-033 M2 desktop shell runtime (deterministic core).
 *
 * The desktop shell's state machine: context binding, connectivity
 * transitions, stale-state labeling, and consequential-action gates.
 * All domain rules are pure (no I/O); the Tauri wrapper (thin signed
 * shell, EP-033 fallback) and future real transports inject I/O
 * behind ports - never the reverse.
 *
 * Invariants (directive F/G/I/J):
 * - principal/tenant/business context comes from BoundContext, never
 *   from a selected screen or cached label;
 * - switching business invalidates the previous context's projections;
 * - connectivity is explicit (CONNECTED/DEGRADED/OFFLINE/
 *   AUTH_EXPIRED/BACKEND_UNAVAILABLE);
 * - stale data is labeled stale and never authorizes a consequential
 *   action without revalidation.
 */

import {
  BoundContext,
  BusinessContext,
  ContextProjection,
  SessionStatus,
  ViewState,
  ErrorCode,
  Spec006Error,
  type ConnectivityState,
  type DataFreshness,
} from "@nexus/web";

export interface DesktopRuntimeSnapshot {
  status:
    | "ACTIVE"
    | "DEGRADED"
    | "OFFLINE"
    | "AUTH_EXPIRED"
    | "BACKEND_UNAVAILABLE";
  connectivity: ConnectivityState;
  correlation: string;
}

/**
 * Deterministic desktop runtime. State transitions are pure functions
 * of (context, connectivity, observed data); the runtime never holds
 * bearer tokens, secrets, or privileged payloads.
 */
export class DesktopShellRuntime {
  #context: BoundContext;
  #connectivity: ConnectivityState = "CONNECTED";
  readonly #projections: Array<ContextProjection<unknown>> = [];

  constructor(context: BoundContext) {
    this.#context = context;
  }

  get context(): BoundContext {
    return this.#context;
  }

  get connectivity(): ConnectivityState {
    return this.#connectivity;
  }

  get status(): SessionStatus {
    return this.#context.session.statusAt(Math.floor(Date.now() / 1000));
  }

  snapshot(nowUnixS: number): DesktopRuntimeSnapshot {
    const sessionStatus = this.#context.session.statusAt(nowUnixS);
    if (sessionStatus === SessionStatus.EXPIRED) {
      return {
        status: "AUTH_EXPIRED",
        connectivity: this.#connectivity,
        correlation: this.#context.session.correlation,
      };
    }
    if (sessionStatus === SessionStatus.REVOKED) {
      return {
        status: "AUTH_EXPIRED",
        connectivity: this.#connectivity,
        correlation: this.#context.session.correlation,
      };
    }
    return {
      status:
        this.#connectivity === "CONNECTED"
          ? "ACTIVE"
          : this.#connectivity === "DEGRADED"
            ? "DEGRADED"
            : this.#connectivity,
      connectivity: this.#connectivity,
      correlation: this.#context.session.correlation,
    };
  }

  /** Connectivity transition with fail-closed semantics. */
  setConnectivity(next: ConnectivityState): void {
    this.#connectivity = next;
  }

  /**
   * Switch business workspace: binds the new context and invalidates
   * every projection of the previous context (directive G). A stale
   * projection can never remain actionable after the switch.
   */
  switchBusiness(next: BusinessContext): BoundContext {
    for (const projection of this.#projections) {
      projection.invalidate();
    }
    this.#context = this.#context.switchBusiness(next);
    return this.#context;
  }

  /**
   * Register a context-bound projection. After a context switch the
   * projection is invalidated and requireCurrent fails closed.
   */
  project<T>(data: T): ContextProjection<T> {
    const projection = new ContextProjection<T>(this.#context, data);
    this.#projections.push(projection as ContextProjection<unknown>);
    return projection;
  }

  /**
   * Consequential-action gate (directive N): under AUTH_EXPIRED or a
   * revoked session, mutation is refused and never queued for blind
   * replay. Under BACKEND_UNAVAILABLE/OFFLINE, a mutation requires
   * fresh verified data.
   */
  requireConsequential(nowUnixS: number, action: string): void {
    const sessionStatus = this.#context.session.statusAt(nowUnixS);
    if (sessionStatus !== SessionStatus.ACTIVE) {
      throw new Spec006Error(
        ErrorCode.Authentication,
        `${action} refused: session not active; re-authenticate`,
        this.#context.session.correlation,
      );
    }
    if (
      this.#connectivity === "BACKEND_UNAVAILABLE" ||
      this.#connectivity === "OFFLINE"
    ) {
      throw new Spec006Error(
        ErrorCode.Unavailable,
        `${action} refused: backend unavailable; revalidate before acting`,
        this.#context.session.correlation,
      );
    }
  }

  /**
   * Label a fetched payload with freshness; a payload fetched while
   * offline or degraded is STALE by construction (directive J).
   */
  labelPayload<T>(
    payload: T,
    correlation: string,
    opts: { observedAt?: number; revision?: number } = {},
  ): ViewState<T> {
    const fresh =
      this.#connectivity === "CONNECTED" || this.#connectivity === "DEGRADED";
    const view = ViewState.success(payload, correlation, {
      observedAt: opts.observedAt ?? Date.now(),
      revision: opts.revision ?? 1,
      connectivity: this.#connectivity,
    });
    if (!fresh) {
      // Construct the stale-labeled variant explicitly: never present
      // offline-fetched data as live.
      return ViewState.fromWire({
        kind: "DEGRADED",
        connectivity: this.#connectivity,
        freshness: "STALE",
        observed_at_unix_ms: opts.observedAt ?? Date.now(),
        revision: opts.revision ?? 0,
        correlation,
      }) as ViewState<T>;
    }
    return view;
  }

  /**
   * Revalidation: promote a stale view to fresh with a newer revision.
   * Monotonic; an older revision is refused.
   */
  revalidate<T>(stale: ViewState<T>, fresh: ViewState<T>): ViewState<T> {
    if (fresh.revision <= stale.revision) {
      throw new Spec006Error(
        ErrorCode.Conflict,
        "Revalidation must increase the revision",
        stale.correlation,
      );
    }
    return fresh;
  }

  /** Theme/appearance changes never touch runtime authority state. */
  applyAppearance<T>(authorityState: T): T {
    return authorityState;
  }
}

export type { DataFreshness };
