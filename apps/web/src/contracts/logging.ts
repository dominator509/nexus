/**
 * EP-033 M1 redacted UI logging (directive P).
 *
 * Frontend diagnostics may carry route, view, correlation id, error
 * class, and backend status. They must NEVER carry the Authorization
 * header, tokens, secrets, private content, passwords, or full
 * sensitive payloads. Redaction happens at the log boundary by
 * construction: entries are built from safe fields only, and the
 * redactor strips secret-shaped substrings from any free text.
 */

import { ErrorCode, Spec006Error } from "./errors";

const SECRET_PATTERNS: ReadonlyArray<RegExp> = [
  /\b(?:bearer|authorization)\s+[A-Za-z0-9._~+/=-]{8,}/i,
  /\b(?:token|access_token|refresh_token|api[_-]?key|secret|password|passphrase)\b[=:]\s*[^\s,;]{4,}/i,
  /\beyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}\b/,
  /\b(?:private[_-]?key|recovery[_-]?kit|approval[_-]?credential)\b[=:]\s*[^\s,;]{4,}/i,
];

export interface RedactedLogEntryShape {
  route: string;
  view: string;
  correlation_id: string;
  error_class: string;
  backend_status: string;
  duration_ms: number;
}

/**
 * A redacted log entry. Fields are safe by construction; any free-text
 * field is passed through the redactor, and the canary test proves
 * secret-shaped content never survives.
 */
export class RedactedLogEntry {
  readonly route: string;
  readonly view: string;
  readonly correlation_id: string;
  readonly error_class: string;
  readonly backend_status: string;
  readonly duration_ms: number;

  private constructor(shape: RedactedLogEntryShape) {
    this.route = redact(shape.route);
    this.view = redact(shape.view);
    this.correlation_id = redact(shape.correlation_id);
    this.error_class = redact(shape.error_class);
    this.backend_status = redact(shape.backend_status);
    this.duration_ms = shape.duration_ms;
  }

  static fromShape(shape: RedactedLogEntryShape): RedactedLogEntry {
    return new RedactedLogEntry(shape);
  }

  /** Serialized form is redacted by construction. */
  serialize(): string {
    return JSON.stringify({
      route: this.route,
      view: this.view,
      correlation_id: this.correlation_id,
      error_class: this.error_class,
      backend_status: this.backend_status,
      duration_ms: this.duration_ms,
    });
  }
}

/** Strip secret-shaped substrings from free text. */
export function redact(text: string): string {
  let out = text;
  for (const pattern of SECRET_PATTERNS) {
    out = out.replace(pattern, "[REDACTED]");
  }
  return out;
}

/**
 * The UI logging boundary. `log` accepts only safe fields; passing a
 * raw secret-bearing field is a compile-time choice of the caller, but
 * the boundary still redacts before anything can be recorded.
 */
export class RedactedLogger {
  readonly #entries: Array<RedactedLogEntry> = [];

  log(entry: RedactedLogEntryShape): RedactedLogEntry {
    const redacted = RedactedLogEntry.fromShape(entry);
    this.#entries.push(redacted);
    return redacted;
  }

  entries(): ReadonlyArray<RedactedLogEntry> {
    return [...this.#entries];
  }

  /** Canary: assert no secret-shaped content survives any entry. */
  assertNoSecrets(): void {
    for (const entry of this.#entries) {
      const serialized = entry.serialize();
      for (const pattern of SECRET_PATTERNS) {
        if (pattern.test(serialized)) {
          throw new Spec006Error(
            ErrorCode.Internal,
            "Log redaction canary failed: secret-shaped content leaked into diagnostics",
          );
        }
      }
    }
  }
}
