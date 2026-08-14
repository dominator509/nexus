// SDK vocabulary for the TypeScript binding (ADR-016).
//
// Canonical SCREAMING_SNAKE wire values; unknown values are rejected
// (vocabulary lock, SPEC-005).

/** SDK language surface (SPEC-022 behavior 4). */
export type SdkLanguage = "RUST" | "TYPESCRIPT" | "PYTHON";

/** Sandboxed sidecar transport family (SPEC-022 behavior 5). */
export type SidecarTransport =
  | "REST"
  | "SOAP"
  | "GRAPHQL"
  | "SQL"
  | "ODBC"
  | "JDBC"
  | "CLI"
  | "FILES"
  | "EMAIL"
  | "WEBHOOK"
  | "BROWSER"
  | "DESKTOP";

/** Legacy source family wrapped by the LegacyPoller (SPEC-022 behavior 5). */
export type LegacyTransport =
  | "REST"
  | "SOAP"
  | "SQL"
  | "CLI"
  | "FILES"
  | "EMAIL"
  | "BROWSER";

/** Webhook delivery state (SPEC-022 behavior 2). */
export type WebhookDeliveryState = "PENDING" | "DELIVERED" | "FAILED" | "REPLAY";

/** Webhook verification result (SPEC-022 behavior 2). */
export type WebhookVerification = "VALID" | "INVALID" | "REPLAY";

/** Parse a vocabulary value; unknown values are rejected. */
export function parseSdkLanguage(value: string): SdkLanguage {
  if (value === "RUST" || value === "TYPESCRIPT" || value === "PYTHON") {
    return value;
  }
  throw new Error(`unknown SdkLanguage value: ${value}`);
}

/** Parse a sidecar transport; unknown values are rejected. */
export function parseSidecarTransport(value: string): SidecarTransport {
  const allowed = [
    "REST",
    "SOAP",
    "GRAPHQL",
    "SQL",
    "ODBC",
    "JDBC",
    "CLI",
    "FILES",
    "EMAIL",
    "WEBHOOK",
    "BROWSER",
    "DESKTOP",
  ] as const;
  if ((allowed as readonly string[]).includes(value)) {
    return value as SidecarTransport;
  }
  throw new Error(`unknown SidecarTransport value: ${value}`);
}
