/**
 * EP-035 M3 redaction helpers.
 *
 * Onboarding is full of secrets: bootstrap secrets, enrollment tokens,
 * integration credentials, recovery material. Every error, log line,
 * event payload, and evidence record must be redacted before leaving
 * this package. Canary injection tests assert ZERO_LEAKAGE.
 */

/** Secret-shaped substrings are replaced with this marker. */
export const REDACTED_MARKER = "[REDACTED]";

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/**
 * A value is secret-shaped if it looks like a token/key/secret.
 * Correlation IDs (UUIDs) are SAFE fields and never match.
 */
export function isSecretShaped(value: string): boolean {
  const v = value.trim();
  if (v.length < 8) {
    return false;
  }
  if (UUID_RE.test(v)) {
    return false;
  }
  return (
    /^(sk|pk|rk|tok|secret|bootstrap|nexus)_[A-Za-z0-9_-]{8,}$/.test(v) ||
    /^[A-Za-z0-9_-]{24,}$/.test(v) ||
    /^[A-Za-z0-9+/]{32,}={0,2}$/.test(v)
  );
}

/**
 * Replace any secret-shaped token in the input with the marker. Scans
 * every run of token characters (works inside JSON blobs and free text),
 * leaving safe fields such as UUIDs and short identifiers untouched.
 */
export function redactSecrets(input: string): string {
  return input.replace(/[A-Za-z0-9_+/-]{12,}/g, (token) =>
    isSecretShaped(token) ? REDACTED_MARKER : token,
  );
}

/** Redact an arbitrary unknown value into a safe summary string. */
export function safeSummary(value: unknown): string {
  if (value === null || value === undefined) {
    return String(value);
  }
  if (typeof value === "string") {
    return redactSecrets(value);
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  try {
    return redactSecrets(JSON.stringify(value));
  } catch {
    return "[unserializable]";
  }
}

/** Ensure an error message carries no secret-shaped content. */
export function redactErrorDetail(detail: string): string {
  return redactSecrets(detail);
}
