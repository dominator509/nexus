/**
 * EP-033 M2 desktop preferences integration (deterministic core).
 *
 * Uses the shared PreferencePersistence boundary: only non-sensitive
 * preferences may be stored; tokens/secrets/approval credentials are
 * refused. Theme state is structurally separate from authority state
 * (directive O/R).
 */

import {
  PreferencePersistence,
  ThemePreference,
  ErrorCode,
  Spec006Error,
  type ThemeMode,
} from "@nexus/web";

export class DesktopPreferences {
  readonly #persistence = new PreferencePersistence();

  setTheme(mode: ThemeMode, reducedMotion: boolean, correlation: string): void {
    const theme = new ThemePreference(mode, reducedMotion, correlation);
    this.#persistence.set("theme", theme.mode);
    this.#persistence.set("reduced_motion", String(theme.reduced_motion));
  }

  setLayout(layout: string): void {
    this.#persistence.set("layout", layout);
  }

  setRecentView(view: string): void {
    this.#persistence.set("recent_views", view);
  }

  /** The persistence boundary refuses anything unsafe. */
  setRaw(key: string, value: string): void {
    this.#persistence.set(key, value);
  }

  get(key: string): string | undefined {
    return this.#persistence.get(key);
  }

  snapshot(): ReadonlyMap<string, string> {
    return this.#persistence.snapshot();
  }

  /** Theme application never touches the authority snapshot. */
  static applyTheme<T>(theme: ThemePreference, authorityState: T): T {
    return ThemePreference.apply(theme, authorityState) as T;
  }
}

export { ThemePreference, Spec006Error, ErrorCode };
