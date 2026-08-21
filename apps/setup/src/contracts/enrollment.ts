/**
 * EP-035 M1 EdgeEnrollment contract (SPEC-004 / SPEC-016).
 *
 * Home-edge enrollment models trust layers explicitly:
 *
 *   DISCOVERED != ENROLLMENT_REQUESTED != IDENTITY_VERIFIED
 *   IDENTITY_VERIFIED != ENROLLED != TRUSTED != AUTHORIZED
 *
 * A hostname, IP address, QR string, device label, Bluetooth name, or
 * mDNS response is NEVER sufficient by itself to move a record toward
 * TRUSTED or AUTHORIZED. Enrollment credentials (BootstrapToken) are
 * secrets: they are never exposed through toString, JSON serialization,
 * error messages, or summaries. Used or expired credentials are never
 * valid again, even if the UI cached them.
 */

import {
  assertEnum,
  assertNonEmptyString,
  assertNonNegativeInt,
  assertObject,
  assertString,
  assertUuid,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const ENROLLMENT_TRUST_STATES = [
  "DISCOVERED",
  "ENROLLMENT_REQUESTED",
  "IDENTITY_VERIFIED",
  "ENROLLED",
  "TRUSTED",
  "AUTHORIZED",
] as const;
export type EnrollmentTrustState = (typeof ENROLLMENT_TRUST_STATES)[number];
const ENROLLMENT_TRUST_STATE_SET: ReadonlySet<EnrollmentTrustState> = new Set(
  ENROLLMENT_TRUST_STATES,
);

export const ENROLLMENT_CREDENTIAL_STATES = [
  "ISSUED",
  "USED",
  "REVOKED",
  "EXPIRED",
] as const;
export type EnrollmentCredentialState =
  (typeof ENROLLMENT_CREDENTIAL_STATES)[number];
const ENROLLMENT_CREDENTIAL_STATE_SET: ReadonlySet<EnrollmentCredentialState> =
  new Set(ENROLLMENT_CREDENTIAL_STATES);

const ENROLLMENT_TRUST_TRANSITIONS: Readonly<
  Record<EnrollmentTrustState, ReadonlySet<EnrollmentTrustState>>
> = {
  DISCOVERED: new Set<EnrollmentTrustState>(["ENROLLMENT_REQUESTED"]),
  ENROLLMENT_REQUESTED: new Set<EnrollmentTrustState>(["IDENTITY_VERIFIED"]),
  IDENTITY_VERIFIED: new Set<EnrollmentTrustState>(["ENROLLED"]),
  ENROLLED: new Set<EnrollmentTrustState>(["TRUSTED"]),
  TRUSTED: new Set<EnrollmentTrustState>(["AUTHORIZED"]),
  AUTHORIZED: new Set<EnrollmentTrustState>([]),
};

export function isValidEnrollmentTrustTransition(
  from: EnrollmentTrustState,
  to: EnrollmentTrustState,
): boolean {
  const allowed = ENROLLMENT_TRUST_TRANSITIONS[from];
  return allowed !== undefined && allowed.has(to);
}

/**
 * A trust transition requires evidence appropriate to the destination.
 * Discovery metadata (hostname/IP/QR/label/BLE name/mDNS response) is
 * NOT evidence for anything beyond DISCOVERED -> ENROLLMENT_REQUESTED.
 */
export function requiredTrustEvidence(
  from: EnrollmentTrustState,
  to: EnrollmentTrustState,
  evidence: string | undefined,
): void {
  if (!isValidEnrollmentTrustTransition(from, to)) {
    throw new Spec006Error(
      ErrorCode.Policy,
      `invalid enrollment trust transition ${from} -> ${to}`,
    );
  }
  if (to === "ENROLLMENT_REQUESTED") {
    return;
  }
  if (evidence === undefined || evidence.length === 0) {
    throw new Spec006Error(
      ErrorCode.Verification,
      `enrollment transition ${from} -> ${to} requires verification evidence`,
    );
  }
}

/**
 * BootstrapToken (SPEC-016 canonical name) enrollment credential.
 * The `secret` and `nonce` fields are SECRET: every serialization path
 * is redacted.
 */
export interface EnrollmentCredentialShape {
  credential_id: string;
  kind: "BOOTSTRAP_TOKEN";
  issued_at_unix_s: number;
  expires_at_unix_s: number;
  state: EnrollmentCredentialState;
  nonce: string;
  secret: string;
}

const ENROLLMENT_CREDENTIAL_FIELDS = new Set<string>([
  "credential_id",
  "kind",
  "issued_at_unix_s",
  "expires_at_unix_s",
  "state",
  "nonce",
  "secret",
]);

export interface RedactedEnrollmentCredentialShape {
  credential_id: string;
  kind: "BOOTSTRAP_TOKEN";
  issued_at_unix_s: number;
  expires_at_unix_s: number;
  state: EnrollmentCredentialState;
}

export class EnrollmentCredential {
  readonly credential_id: string;
  readonly kind: "BOOTSTRAP_TOKEN";
  readonly issued_at_unix_s: number;
  readonly expires_at_unix_s: number;
  readonly state: EnrollmentCredentialState;
  readonly nonce: string;
  readonly secret: string;

  private constructor(
    credentialId: string,
    issuedAtUnixS: number,
    expiresAtUnixS: number,
    state: EnrollmentCredentialState,
    nonce: string,
    secret: string,
  ) {
    this.credential_id = credentialId;
    this.kind = "BOOTSTRAP_TOKEN";
    this.issued_at_unix_s = issuedAtUnixS;
    this.expires_at_unix_s = expiresAtUnixS;
    this.state = state;
    this.nonce = nonce;
    this.secret = secret;
  }

  static parse(
    value: unknown,
    what = "enrollment credential",
  ): EnrollmentCredential {
    const obj = assertObject(value, what);
    rejectUnknownFields(obj, ENROLLMENT_CREDENTIAL_FIELDS, what);
    const kind = assertEnum(
      obj["kind"],
      new Set<string>(["BOOTSTRAP_TOKEN"]),
      `${what}.kind`,
    );
    if (kind !== "BOOTSTRAP_TOKEN") {
      throw new Spec006Error(
        ErrorCode.Vocabulary,
        `${what}.kind must be BOOTSTRAP_TOKEN`,
      );
    }
    const issued = assertNonNegativeInt(
      obj["issued_at_unix_s"],
      `${what}.issued_at_unix_s`,
    );
    const expires = assertNonNegativeInt(
      obj["expires_at_unix_s"],
      `${what}.expires_at_unix_s`,
    );
    if (expires <= issued) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `${what} expires_at_unix_s must be after issued_at_unix_s`,
      );
    }
    return new EnrollmentCredential(
      assertUuid(obj["credential_id"], `${what}.credential_id`),
      issued,
      expires,
      assertEnum(
        obj["state"],
        ENROLLMENT_CREDENTIAL_STATE_SET,
        `${what}.state`,
      ),
      assertNonEmptyString(obj["nonce"], `${what}.nonce`),
      assertNonEmptyString(obj["secret"], `${what}.secret`),
    );
  }

  /** A credential is usable only while ISSUED and within its validity window. */
  isUsable(nowUnixS: number): boolean {
    return (
      this.state === "ISSUED" &&
      nowUnixS >= this.issued_at_unix_s &&
      nowUnixS <= this.expires_at_unix_s
    );
  }

  /**
   * Secret-safe serialization: never emits secret or nonce. Every
   * summary, log line, and JSON surface must use this shape.
   */
  redacted(): RedactedEnrollmentCredentialShape {
    return {
      credential_id: this.credential_id,
      kind: this.kind,
      issued_at_unix_s: this.issued_at_unix_s,
      expires_at_unix_s: this.expires_at_unix_s,
      state: this.state,
    };
  }

  toJSON(): RedactedEnrollmentCredentialShape {
    return this.redacted();
  }

  toString(): string {
    return `EnrollmentCredential(${this.credential_id}, ${this.state})`;
  }
}

export interface EdgeEnrollmentRequestShape {
  device_label: string;
  endpoint: string;
  credential_id: string;
  correlation_id: string;
}

const EDGE_ENROLLMENT_REQUEST_FIELDS = new Set<string>([
  "device_label",
  "endpoint",
  "credential_id",
  "correlation_id",
]);

export class EdgeEnrollmentRequest {
  readonly device_label: string;
  readonly endpoint: string;
  readonly credential_id: string;
  readonly correlation_id: string;

  private constructor(
    deviceLabel: string,
    endpoint: string,
    credentialId: string,
    correlationId: string,
  ) {
    this.device_label = deviceLabel;
    this.endpoint = endpoint;
    this.credential_id = credentialId;
    this.correlation_id = correlationId;
  }

  static parse(value: unknown): EdgeEnrollmentRequest {
    const obj = assertObject(value, "edge enrollment request");
    rejectUnknownFields(
      obj,
      EDGE_ENROLLMENT_REQUEST_FIELDS,
      "edge enrollment request",
    );
    return new EdgeEnrollmentRequest(
      assertNonEmptyString(
        obj["device_label"],
        "edge enrollment request.device_label",
      ),
      assertNonEmptyString(obj["endpoint"], "edge enrollment request.endpoint"),
      assertUuid(obj["credential_id"], "edge enrollment request.credential_id"),
      assertUuid(
        obj["correlation_id"],
        "edge enrollment request.correlation_id",
      ),
    );
  }
}

/** Provider-neutral EdgeEnrollment port. M1 declares the boundary. */
export interface EdgeEnrollmentPort {
  requestEnrollment(request: EdgeEnrollmentRequestShape): EnrollmentTrustState;
  verifyIdentity(
    credential: EnrollmentCredentialShape,
    evidence: string,
  ): EnrollmentTrustState;
  enroll(evidence: string): EnrollmentTrustState;
  trust(evidence: string): EnrollmentTrustState;
  authorize(evidence: string): EnrollmentTrustState;
}
