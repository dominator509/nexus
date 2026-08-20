/**
 * EP-033 M1 preference and state persistence contract (directive O/R).
 *
 * Local UI persistence is classified: only non-sensitive preferences
 * (theme, layout, recent view state, reduced-motion) may be stored.
 * Bearer tokens, secrets, approval credentials, and privileged backend
 * responses are refused by the persistence boundary itself, so a
 * rendering bug cannot leak them into storage.
 *
 * Theme/appearance preference is isolated from authority state:
 * changing the visual mode never mutates tenant, permissions,
 * capabilities, or approval state (directive R).
 */

import { ErrorCode, Spec006Error } from "./errors";

export const THEME_MODES = ["LIGHT", "DARK", "SYSTEM"] as const;
export type ThemeMode = (typeof THEME_MODES)[number];

/**
 * Keys the UI may persist locally. Everything else is refused.
 * This is the allowlist: storage of a key outside this set is a
 * contract violation.
 */
export const ALLOWED_PREFERENCE_KEYS = new Set<string>([
  "theme",
  "layout",
  "reduced_motion",
  "recent_views",
]);

/**
 * Keys that must NEVER be persisted locally, even accidentally.
 * Used by the persistence boundary and by canary tests.
 */
export const FORBIDDEN_PREFERENCE_KEYS = new Set<string>([
  "access_token",
  "refresh_token",
  "bearer",
  "authorization",
  "secret",
  "password",
  "passkey",
  "approval_credential",
  "private_key",
  "recovery_kit",
]);

const SECRET_MARKERS = [
  "token",
  "secret",
  "password",
  "passphrase",
  "private_key",
  "authorization",
  "bearer",
  "credential",
];

export interface PersistenceDecision {
  allowed: boolean;
  key: string;
  reason: string;
}

/**
 * The local persistence boundary. Values are classified by key AND
 * content: a value containing secret markers is refused even under an
 * innocent-looking key (defense in depth).
 */
export class PreferencePersistence {
  readonly #storage: Map<string, string> = new Map();

  classify(key: string, value: string): PersistenceDecision {
    if (FORBIDDEN_PREFERENCE_KEYS.has(key)) {
      return { allowed: false, key, reason: "forbidden key" };
    }
    if (!ALLOWED_PREFERENCE_KEYS.has(key)) {
      return { allowed: false, key, reason: "key not in allowlist" };
    }
    const lower = value.toLowerCase();
    for (const marker of SECRET_MARKERS) {
      if (lower.includes(marker)) {
        return { allowed: false, key, reason: `value contains secret marker '${marker}'` };
      }
    }
    return { allowed: true, key, reason: "allowed" };
  }

  /**
   * Persist a preference. Refuses (throws) when the classification is
   * not allowed: a renderer bug cannot persist secrets.
   */
  set(key: string, value: string): void {
    const decision = this.classify(key, value);
    if (!decision.allowed) {
      throw new Spec006Error(
        ErrorCode.Policy,
        `Persistence refused: ${decision.reason}`,
      );
    }
    this.#storage.set(key, value);
  }

  get(key: string): string | undefined {
    return this.#storage.get(key);
  }

  snapshot(): ReadonlyMap<string, string> {
    return new Map(this.#storage);
  }
}

/**
 * Theme preference. Carries only appearance state: it is structurally
 * impossible for a ThemePreference to carry tenant, permission,
 * capability, or approval data.
 */
export class ThemePreference {
  readonly mode: ThemeMode;
  readonly reduced_motion: boolean;
  readonly correlation: string;

  constructor(mode: ThemeMode, reducedMotion: boolean, correlation: string) {
    if (!THEME_MODES.includes(mode)) {
      throw new Spec006Error(ErrorCode.Vocabulary, `Unsupported theme mode '${mode}'`);
    }
    this.mode = mode;
    this.reduced_motion = reducedMotion;
    this.correlation = correlation;
  }

  /** Appearance only: applying a theme never touches authority state. */
  static apply(_preference: ThemePreference, authorityState: unknown): unknown {
    // The theme is rendered, not applied to state. Returning the
    // authority state unchanged proves isolation by construction.
    return authorityState;
  }
}
