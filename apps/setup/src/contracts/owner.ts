/**
 * EP-035 M1 OwnerBootstrap contract (SPEC-004 / SPEC-016).
 *
 * Owner bootstrap is security-critical. The contract models the
 * ladder explicitly:
 *
 *   OWNER_DETAILS_PROVIDED != OWNER_IDENTITY_VERIFIED
 *   OWNER_IDENTITY_VERIFIED != OWNER_PRINCIPAL_CREATED
 *   OWNER_PRINCIPAL_CREATED != OWNER_AUTHORIZED
 *
 * A client-side field such as `isOwner: true` is REJECTED at parse time
 * (deny-unknown): client input can never mint backend authority.
 *
 * First-owner initialization is deterministic and conflict-safe: a
 * replayed bootstrap request returns the same initialized result; a
 * competing request against an initialized owner returns CONFLICT. The
 * durable enforcement is owned by M2+; the contract semantics are
 * encoded here and tested.
 */

import {
  assertEnum,
  assertNonEmptyString,
  assertNonNegativeInt,
  assertObject,
  assertOptionalString,
  assertString,
  assertUuid,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const OWNER_BOOTSTRAP_STATES = [
  "OWNER_DETAILS_PROVIDED",
  "OWNER_IDENTITY_VERIFIED",
  "OWNER_PRINCIPAL_CREATED",
  "OWNER_AUTHORIZED",
] as const;
export type OwnerBootstrapState = (typeof OWNER_BOOTSTRAP_STATES)[number];
const OWNER_BOOTSTRAP_STATE_SET: ReadonlySet<OwnerBootstrapState> = new Set(
  OWNER_BOOTSTRAP_STATES,
);

export interface OwnerBootstrapRequestShape {
  owner_name: string;
  owner_email: string;
  correlation_id: string;
  idempotency_key: string;
  recovery_kit_id?: string;
  verification_method?: string;
}

const OWNER_BOOTSTRAP_REQUEST_FIELDS = new Set<string>([
  "owner_name",
  "owner_email",
  "correlation_id",
  "idempotency_key",
  "recovery_kit_id",
  "verification_method",
]);

export class OwnerBootstrapRequest {
  readonly owner_name: string;
  readonly owner_email: string;
  readonly correlation_id: string;
  readonly idempotency_key: string;
  readonly recovery_kit_id: string | undefined;
  readonly verification_method: string | undefined;

  private constructor(
    ownerName: string,
    ownerEmail: string,
    correlationId: string,
    idempotencyKey: string,
    recoveryKitId: string | undefined,
    verificationMethod: string | undefined,
  ) {
    this.owner_name = ownerName;
    this.owner_email = ownerEmail;
    this.correlation_id = correlationId;
    this.idempotency_key = idempotencyKey;
    this.recovery_kit_id = recoveryKitId;
    this.verification_method = verificationMethod;
  }

  static parse(value: unknown): OwnerBootstrapRequest {
    const obj = assertObject(value, "owner bootstrap request");
    rejectUnknownFields(
      obj,
      OWNER_BOOTSTRAP_REQUEST_FIELDS,
      "owner bootstrap request",
    );
    return new OwnerBootstrapRequest(
      assertNonEmptyString(
        obj["owner_name"],
        "owner bootstrap request.owner_name",
      ),
      assertNonEmptyString(
        obj["owner_email"],
        "owner bootstrap request.owner_email",
      ),
      assertUuid(
        obj["correlation_id"],
        "owner bootstrap request.correlation_id",
      ),
      assertNonEmptyString(
        obj["idempotency_key"],
        "owner bootstrap request.idempotency_key",
      ),
      assertOptionalString(
        obj["recovery_kit_id"],
        "owner bootstrap request.recovery_kit_id",
      ),
      assertOptionalString(
        obj["verification_method"],
        "owner bootstrap request.verification_method",
      ),
    );
  }

  toJSON(): OwnerBootstrapRequestShape {
    return {
      owner_name: this.owner_name,
      owner_email: this.owner_email,
      correlation_id: this.correlation_id,
      idempotency_key: this.idempotency_key,
      ...(this.recovery_kit_id === undefined
        ? {}
        : { recovery_kit_id: this.recovery_kit_id }),
      ...(this.verification_method === undefined
        ? {}
        : { verification_method: this.verification_method }),
    };
  }
}

export interface OwnerBootstrapStateShape {
  state: OwnerBootstrapState;
  owner_email: string;
  principal_id?: string;
  correlation_id: string;
  updated_at_unix_s: number;
}

const OWNER_BOOTSTRAP_STATE_FIELDS = new Set<string>([
  "state",
  "owner_email",
  "principal_id",
  "correlation_id",
  "updated_at_unix_s",
]);

export class OwnerBootstrapStateRecord {
  readonly state: OwnerBootstrapState;
  readonly owner_email: string;
  readonly principal_id: string | undefined;
  readonly correlation_id: string;
  readonly updated_at_unix_s: number;

  private constructor(
    state: OwnerBootstrapState,
    ownerEmail: string,
    principalId: string | undefined,
    correlationId: string,
    updatedAtUnixS: number,
  ) {
    this.state = state;
    this.owner_email = ownerEmail;
    this.principal_id = principalId;
    this.correlation_id = correlationId;
    this.updated_at_unix_s = updatedAtUnixS;
  }

  static parse(value: unknown): OwnerBootstrapStateRecord {
    const obj = assertObject(value, "owner bootstrap state");
    rejectUnknownFields(
      obj,
      OWNER_BOOTSTRAP_STATE_FIELDS,
      "owner bootstrap state",
    );
    const state = assertEnum(
      obj["state"],
      OWNER_BOOTSTRAP_STATE_SET,
      "owner bootstrap state.state",
    );
    const principalRaw = obj["principal_id"];
    const principal =
      principalRaw === undefined
        ? undefined
        : assertUuid(principalRaw, "owner bootstrap state.principal_id");
    if (state === "OWNER_PRINCIPAL_CREATED" || state === "OWNER_AUTHORIZED") {
      if (principal === undefined) {
        throw new Spec006Error(
          ErrorCode.Validation,
          `owner bootstrap state ${state} requires a principal_id`,
        );
      }
    }
    return new OwnerBootstrapStateRecord(
      state,
      assertNonEmptyString(
        obj["owner_email"],
        "owner bootstrap state.owner_email",
      ),
      principal,
      assertUuid(obj["correlation_id"], "owner bootstrap state.correlation_id"),
      assertNonNegativeInt(
        obj["updated_at_unix_s"],
        "owner bootstrap state.updated_at_unix_s",
      ),
    );
  }

  static detailsProvided(
    request: OwnerBootstrapRequest,
    atUnixS: number,
  ): OwnerBootstrapStateRecord {
    return new OwnerBootstrapStateRecord(
      "OWNER_DETAILS_PROVIDED",
      request.owner_email,
      undefined,
      request.correlation_id,
      atUnixS,
    );
  }

  /** Typed transition; identity verification requires a method. */
  advance(
    toState: OwnerBootstrapState,
    atUnixS: number,
    principalId?: string,
    verificationMethod?: string,
  ): OwnerBootstrapStateRecord {
    if (!isValidOwnerBootstrapTransition(this.state, toState)) {
      throw new Spec006Error(
        ErrorCode.Policy,
        `invalid owner bootstrap transition ${this.state} -> ${toState}`,
        this.correlation_id,
      );
    }
    if (toState === "OWNER_IDENTITY_VERIFIED") {
      if (verificationMethod === undefined || verificationMethod.length === 0) {
        throw new Spec006Error(
          ErrorCode.Verification,
          "owner identity verification requires a verification method",
          this.correlation_id,
        );
      }
    }
    if (
      toState === "OWNER_PRINCIPAL_CREATED" ||
      toState === "OWNER_AUTHORIZED"
    ) {
      if (principalId === undefined) {
        throw new Spec006Error(
          ErrorCode.Validation,
          `owner bootstrap ${toState} requires a principal_id`,
          this.correlation_id,
        );
      }
    }
    return new OwnerBootstrapStateRecord(
      toState,
      this.owner_email,
      principalId,
      this.correlation_id,
      atUnixS,
    );
  }

  toJSON(): OwnerBootstrapStateShape {
    return {
      state: this.state,
      owner_email: this.owner_email,
      ...(this.principal_id === undefined
        ? {}
        : { principal_id: this.principal_id }),
      correlation_id: this.correlation_id,
      updated_at_unix_s: this.updated_at_unix_s,
    };
  }
}

const OWNER_BOOTSTRAP_TRANSITIONS: Readonly<
  Record<OwnerBootstrapState, ReadonlySet<OwnerBootstrapState>>
> = {
  OWNER_DETAILS_PROVIDED: new Set<OwnerBootstrapState>([
    "OWNER_IDENTITY_VERIFIED",
  ]),
  OWNER_IDENTITY_VERIFIED: new Set<OwnerBootstrapState>([
    "OWNER_PRINCIPAL_CREATED",
  ]),
  OWNER_PRINCIPAL_CREATED: new Set<OwnerBootstrapState>(["OWNER_AUTHORIZED"]),
  OWNER_AUTHORIZED: new Set<OwnerBootstrapState>([]),
};

export function isValidOwnerBootstrapTransition(
  from: OwnerBootstrapState,
  to: OwnerBootstrapState,
): boolean {
  const allowed = OWNER_BOOTSTRAP_TRANSITIONS[from];
  return allowed !== undefined && allowed.has(to);
}

export type FirstOwnerResult =
  | { kind: "INITIALIZED"; principal_id: string }
  | { kind: "ALREADY_INITIALIZED"; principal_id: string }
  | { kind: "CONFLICT" };

export interface FirstOwnerKnownShape {
  idempotency_key: string;
  principal_id: string;
}

const FIRST_OWNER_KNOWN_FIELDS = new Set<string>([
  "idempotency_key",
  "principal_id",
]);

export class FirstOwnerKnown {
  readonly idempotency_key: string;
  readonly principal_id: string;

  /** Constructible value object; parse also validates wire input. */
  constructor(idempotencyKey: string, principalId: string) {
    this.idempotency_key = idempotencyKey;
    this.principal_id = principalId;
  }

  static parse(value: unknown): FirstOwnerKnown {
    const obj = assertObject(value, "first owner known");
    rejectUnknownFields(obj, FIRST_OWNER_KNOWN_FIELDS, "first owner known");
    return new FirstOwnerKnown(
      assertNonEmptyString(
        obj["idempotency_key"],
        "first owner known.idempotency_key",
      ),
      assertUuid(obj["principal_id"], "first owner known.principal_id"),
    );
  }
}

/**
 * Deterministic first-owner decision contract (SPEC-004 behavior 3).
 *
 * - no known owner + request          -> INITIALIZED (with the request's
 *   principal once created by the caller contract; M1 encodes the
 *   decision shape with the deterministic principal derived from the
 *   request correlation)
 * - same idempotency key replayed     -> ALREADY_INITIALIZED (same principal)
 * - different key, owner exists       -> CONFLICT (never two first owners)
 *
 * Durable enforcement of this contract is owned by M2+; this pure
 * function fixes the semantics and is fully deterministic.
 */
export function resolveFirstOwnerRequest(
  known: FirstOwnerKnown | undefined,
  request: OwnerBootstrapRequest,
  principalId: string,
): FirstOwnerResult {
  if (known === undefined) {
    return { kind: "INITIALIZED", principal_id: principalId };
  }
  if (known.idempotency_key === request.idempotency_key) {
    return { kind: "ALREADY_INITIALIZED", principal_id: known.principal_id };
  }
  return { kind: "CONFLICT" };
}

/** Provider-neutral OwnerBootstrap port. M1 declares the boundary. */
export interface OwnerBootstrapPort {
  initialize(
    request: OwnerBootstrapRequestShape,
    known: FirstOwnerKnownShape | null,
  ): FirstOwnerResult;
}
