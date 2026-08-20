/**
 * EP-033 M1 UI state vocabulary (SPEC-004 behavior 6, directive I/J).
 *
 * Every UI state distinguishes loading, empty, error, degraded,
 * permission-denied, and success. Connectivity is explicit:
 * CONNECTED / DEGRADED / OFFLINE / AUTH_EXPIRED / BACKEND_UNAVAILABLE.
 * Cached data is labeled fresh or stale: stale is never rendered as
 * verified current state, and consequential actions never run on stale
 * optimistic state without revalidation.
 */

import { assertEnum, assertInt, assertObject, rejectUnknownFields } from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const UI_STATE_KINDS = [
  "LOADING",
  "EMPTY",
  "ERROR",
  "DEGRADED",
  "PERMISSION_DENIED",
  "SUCCESS",
] as const;
export type UiStateKind = (typeof UI_STATE_KINDS)[number];

export const CONNECTIVITY_STATES = [
  "CONNECTED",
  "DEGRADED",
  "OFFLINE",
  "AUTH_EXPIRED",
  "BACKEND_UNAVAILABLE",
] as const;
export type ConnectivityState = (typeof CONNECTIVITY_STATES)[number];

export const DATA_FRESHNESS = ["FRESH", "STALE"] as const;
export type DataFreshness = (typeof DATA_FRESHNESS)[number];

const VIEW_STATE_FIELDS = new Set<string>([
  "kind",
  "connectivity",
  "freshness",
  "observed_at_unix_ms",
  "revision",
  "correlation",
]);

/**
 * A state-labeled view payload. The payload is data; the labels are
 * authority about that data's trustworthiness. Rendering must not
 * present a STALE payload as live.
 */
export class ViewState<T> {
  readonly kind: UiStateKind;
  readonly connectivity: ConnectivityState;
  readonly freshness: DataFreshness;
  readonly observed_at_unix_ms: number;
  readonly revision: number;
  readonly correlation: string;
  readonly payload: T | undefined;

  private constructor(
    kind: UiStateKind,
    connectivity: ConnectivityState,
    freshness: DataFreshness,
    observedAt: number,
    revision: number,
    correlation: string,
    payload: T | undefined,
  ) {
    this.kind = kind;
    this.connectivity = connectivity;
    this.freshness = freshness;
    this.observed_at_unix_ms = observedAt;
    this.revision = revision;
    this.correlation = correlation;
    this.payload = payload;
  }

  static loading(correlation: string): ViewState<never> {
    return new ViewState<never>("LOADING", "CONNECTED", "STALE", 0, 0, correlation, undefined);
  }

  static success<T>(
    payload: T,
    correlation: string,
    opts: { observedAt?: number; revision?: number; connectivity?: ConnectivityState } = {},
  ): ViewState<T> {
    return new ViewState<T>(
      "SUCCESS",
      opts.connectivity ?? "CONNECTED",
      "FRESH",
      opts.observedAt ?? Date.now(),
      opts.revision ?? 1,
      correlation,
      payload,
    );
  }

  static empty(correlation: string, connectivity: ConnectivityState = "CONNECTED"): ViewState<never> {
    return new ViewState<never>("EMPTY", connectivity, "FRESH", Date.now(), 1, correlation, undefined);
  }

  static error(
    error: Spec006Error,
    correlation: string,
    connectivity: ConnectivityState = "CONNECTED",
  ): ViewState<never> {
    const kind: UiStateKind =
      error.code === ErrorCode.Authorization || error.code === ErrorCode.Policy
        ? "PERMISSION_DENIED"
        : "ERROR";
    return new ViewState<never>(kind, connectivity, "STALE", Date.now(), 0, correlation, undefined);
  }

  static degraded(
    correlation: string,
    connectivity: ConnectivityState,
    observedAt: number,
  ): ViewState<never> {
    return new ViewState<never>("DEGRADED", connectivity, "STALE", observedAt, 0, correlation, undefined);
  }

  static fromWire(value: unknown): ViewState<unknown> {
    const obj = assertObject(value, "ViewState");
    rejectUnknownFields(obj, VIEW_STATE_FIELDS, "ViewState");
    return new ViewState<unknown>(
      assertEnum(obj.kind, new Set<UiStateKind>(UI_STATE_KINDS), "kind"),
      assertEnum(obj.connectivity, new Set<ConnectivityState>(CONNECTIVITY_STATES), "connectivity"),
      assertEnum(obj.freshness, new Set<DataFreshness>(DATA_FRESHNESS), "freshness"),
      assertInt(obj.observed_at_unix_ms, "observed_at_unix_ms"),
      assertInt(obj.revision, "revision"),
      typeof obj.correlation === "string" ? obj.correlation : "",
      obj.payload,
    );
  }

  /** Consequential action gate: stale data must be revalidated first. */
  requireFresh(action: string): T {
    if (this.freshness !== "FRESH") {
      throw new Spec006Error(
        ErrorCode.Conflict,
        `${action} refused: data is stale; revalidate before acting`,
        this.correlation,
      );
    }
    if (this.payload === undefined) {
      throw new Spec006Error(
        ErrorCode.Conflict,
        `${action} refused: no verified payload`,
        this.correlation,
      );
    }
    return this.payload;
  }
}

/**
 * Revalidation record: after re-fetching, a stale ViewState becomes
 * fresh with a new revision. Revisions are monotonic; a lower revision
 * can never overwrite a higher one (directive J).
 */
export function revalidated<T>(previous: ViewState<T>, next: ViewState<T>): ViewState<T> {
  if (next.revision < previous.revision) {
    throw new Spec006Error(
      ErrorCode.Conflict,
      "Revalidation returned an older revision",
      next.correlation,
    );
  }
  return next;
}
