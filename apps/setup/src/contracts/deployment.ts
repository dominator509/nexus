/**
 * EP-035 M1 DeploymentChoice contract (SPEC-016).
 *
 * A selected topology is INTENT ONLY. Selecting MANAGED / BYOC /
 * EXISTING_SSH / HYBRID / FULLY_LOCAL records that the user requested
 * that deployment class; it does NOT prove that a host exists, a
 * container runtime is present, ports are open, DNS is correct, TLS
 * works, Nexus is running, or health checks pass.
 *
 * Verification is a separate, explicitly tracked state that can only be
 * reached through a verification record with evidence. Intent never
 * implies verification.
 */

import {
  assertEnum,
  assertNonNegativeInt,
  assertObject,
  assertStringArray,
  assertNonEmptyString,
  assertString,
  assertUuid,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const DEPLOYMENT_MODES = [
  "MANAGED",
  "BYOC",
  "EXISTING_SSH",
  "HYBRID",
  "FULLY_LOCAL",
] as const;
export type DeploymentMode = (typeof DEPLOYMENT_MODES)[number];
const DEPLOYMENT_MODE_SET: ReadonlySet<DeploymentMode> = new Set(
  DEPLOYMENT_MODES,
);

export const RELEASE_CHANNELS = [
  "STABLE",
  "BETA",
  "DEVELOPER",
  "PINNED",
] as const;
export type ReleaseChannel = (typeof RELEASE_CHANNELS)[number];
const RELEASE_CHANNEL_SET: ReadonlySet<ReleaseChannel> = new Set(
  RELEASE_CHANNELS,
);

export const DEPLOYMENT_VERIFICATION_STATES = [
  "UNVERIFIED",
  "VERIFYING",
  "VERIFIED",
  "FAILED",
] as const;
export type DeploymentVerificationState =
  (typeof DEPLOYMENT_VERIFICATION_STATES)[number];
const DEPLOYMENT_VERIFICATION_STATE_SET: ReadonlySet<DeploymentVerificationState> =
  new Set(DEPLOYMENT_VERIFICATION_STATES);

/**
 * Canonical DeploymentProfile value object. Field names and enums are
 * the canonical snake_case wire names from
 * schemas/deployment-profile.schema.json verbatim; parity is enforced
 * by ep035_unit_schema_parity tests.
 */
export interface DeploymentProfileShape {
  id: string;
  mode: DeploymentMode;
  release_channel: ReleaseChannel;
  components: Array<string>;
  nodes: Array<Record<string, unknown>>;
  backup: Record<string, unknown>;
  remote_access: Record<string, unknown>;
}

const DEPLOYMENT_PROFILE_FIELDS = new Set<string>([
  "id",
  "mode",
  "release_channel",
  "components",
  "nodes",
  "backup",
  "remote_access",
]);

export class DeploymentProfile {
  readonly id: string;
  readonly mode: DeploymentMode;
  readonly release_channel: ReleaseChannel;
  readonly components: ReadonlyArray<string>;
  readonly nodes: ReadonlyArray<Record<string, unknown>>;
  readonly backup: Record<string, unknown>;
  readonly remote_access: Record<string, unknown>;

  private constructor(
    id: string,
    mode: DeploymentMode,
    releaseChannel: ReleaseChannel,
    components: ReadonlyArray<string>,
    nodes: ReadonlyArray<Record<string, unknown>>,
    backup: Record<string, unknown>,
    remoteAccess: Record<string, unknown>,
  ) {
    this.id = id;
    this.mode = mode;
    this.release_channel = releaseChannel;
    this.components = components;
    this.nodes = nodes;
    this.backup = backup;
    this.remote_access = remoteAccess;
  }

  static parse(value: unknown, what = "deployment profile"): DeploymentProfile {
    const obj = assertObject(value, what);
    rejectUnknownFields(obj, DEPLOYMENT_PROFILE_FIELDS, what);
    const nodesRaw = obj["nodes"];
    if (!Array.isArray(nodesRaw)) {
      throw new Spec006Error(
        ErrorCode.Validation,
        `${what}.nodes must be an array`,
      );
    }
    const nodes = nodesRaw.map((entry, index) =>
      assertObject(entry, `${what}.nodes[${index}]`),
    );
    return new DeploymentProfile(
      assertNonEmptyString(obj["id"], `${what}.id`),
      assertEnum(obj["mode"], DEPLOYMENT_MODE_SET, `${what}.mode`),
      assertEnum(
        obj["release_channel"],
        RELEASE_CHANNEL_SET,
        `${what}.release_channel`,
      ),
      assertStringArray(obj["components"], `${what}.components`, 256),
      nodes,
      assertObject(obj["backup"], `${what}.backup`),
      assertObject(obj["remote_access"], `${what}.remote_access`),
    );
  }

  toJSON(): DeploymentProfileShape {
    return {
      id: this.id,
      mode: this.mode,
      release_channel: this.release_channel,
      components: [...this.components],
      nodes: this.nodes.map((entry) => ({ ...entry })),
      backup: { ...this.backup },
      remote_access: { ...this.remote_access },
    };
  }
}

export interface DeploymentVerificationEvidenceShape {
  verified_at_unix_s: number;
  evidence_id: string;
  verifier: string;
}

const DEPLOYMENT_VERIFICATION_EVIDENCE_FIELDS = new Set<string>([
  "verified_at_unix_s",
  "evidence_id",
  "verifier",
]);

function parseDeploymentVerificationEvidence(
  value: unknown,
  what: string,
): DeploymentVerificationEvidenceShape {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, DEPLOYMENT_VERIFICATION_EVIDENCE_FIELDS, what);
  return {
    verified_at_unix_s: assertNonNegativeInt(
      obj["verified_at_unix_s"],
      `${what}.verified_at_unix_s`,
    ),
    evidence_id: assertNonEmptyString(
      obj["evidence_id"],
      `${what}.evidence_id`,
    ),
    verifier: assertNonEmptyString(obj["verifier"], `${what}.verifier`),
  };
}

export interface DeploymentVerificationShape {
  state: DeploymentVerificationState;
  evidence?: DeploymentVerificationEvidenceShape | null;
}

export interface DeploymentIntentRecordShape {
  profile: DeploymentProfileShape;
  selected_at_unix_s: number;
  correlation_id: string;
  verification: DeploymentVerificationShape;
}

const DEPLOYMENT_INTENT_FIELDS = new Set<string>([
  "profile",
  "selected_at_unix_s",
  "correlation_id",
  "verification",
]);

const DEPLOYMENT_VERIFICATION_FIELDS = new Set<string>(["state", "evidence"]);

function parseDeploymentVerification(
  value: unknown,
  what: string,
): DeploymentVerificationShape {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, DEPLOYMENT_VERIFICATION_FIELDS, what);
  const state = assertEnum(
    obj["state"],
    DEPLOYMENT_VERIFICATION_STATE_SET,
    `${what}.state`,
  );
  const evidenceRaw = obj["evidence"];
  if (evidenceRaw !== undefined && evidenceRaw !== null) {
    const evidence = parseDeploymentVerificationEvidence(
      evidenceRaw,
      `${what}.evidence`,
    );
    if (state !== "VERIFIED") {
      throw new Spec006Error(
        ErrorCode.Validation,
        `${what} carries verification evidence but state is ${state}; only VERIFIED may carry evidence`,
      );
    }
    return { state, evidence };
  }
  if (state === "VERIFIED") {
    throw new Spec006Error(
      ErrorCode.Verification,
      `${what} is VERIFIED but has no evidence record`,
    );
  }
  return { state };
}

/**
 * DeploymentIntentRecord: a user's deployment selection. The
 * verification state is structurally separate and starts UNVERIFIED.
 * Selection NEVER proves host/runtime/ports/DNS/TLS/health.
 */
export class DeploymentIntentRecord {
  readonly profile: DeploymentProfile;
  readonly selected_at_unix_s: number;
  readonly correlation_id: string;
  readonly verification: DeploymentVerificationShape;

  private constructor(
    profile: DeploymentProfile,
    selectedAtUnixS: number,
    correlationId: string,
    verification: DeploymentVerificationShape,
  ) {
    this.profile = profile;
    this.selected_at_unix_s = selectedAtUnixS;
    this.correlation_id = correlationId;
    this.verification = verification;
  }

  static parse(value: unknown): DeploymentIntentRecord {
    const obj = assertObject(value, "deployment intent record");
    rejectUnknownFields(
      obj,
      DEPLOYMENT_INTENT_FIELDS,
      "deployment intent record",
    );
    return new DeploymentIntentRecord(
      DeploymentProfile.parse(
        obj["profile"],
        "deployment intent record.profile",
      ),
      assertNonNegativeInt(
        obj["selected_at_unix_s"],
        "deployment intent record.selected_at_unix_s",
      ),
      assertUuid(
        obj["correlation_id"],
        "deployment intent record.correlation_id",
      ),
      parseDeploymentVerification(
        obj["verification"],
        "deployment intent record.verification",
      ),
    );
  }

  /** A selection is created with verification UNVERIFIED, always. */
  static select(
    profile: DeploymentProfile,
    correlationId: string,
    atUnixS: number,
  ): DeploymentIntentRecord {
    return new DeploymentIntentRecord(profile, atUnixS, correlationId, {
      state: "UNVERIFIED",
    });
  }

  /** Mark verified only with real evidence; VERIFYING/FAILED are explicit. */
  withVerification(
    state: DeploymentVerificationState,
    atUnixS: number,
    evidence?: DeploymentVerificationEvidenceShape,
  ): DeploymentIntentRecord {
    if (state === "VERIFIED" && evidence === undefined) {
      throw new Spec006Error(
        ErrorCode.Verification,
        "deployment verification requires evidence",
        this.correlation_id,
      );
    }
    if (state !== "VERIFIED" && evidence !== undefined) {
      throw new Spec006Error(
        ErrorCode.Validation,
        "deployment verification evidence only valid for VERIFIED",
        this.correlation_id,
      );
    }
    return new DeploymentIntentRecord(
      this.profile,
      this.selected_at_unix_s,
      this.correlation_id,
      state === "VERIFIED" && evidence !== undefined
        ? { state, evidence }
        : { state },
    );
  }

  toJSON(): DeploymentIntentRecordShape {
    return {
      profile: this.profile.toJSON(),
      selected_at_unix_s: this.selected_at_unix_s,
      correlation_id: this.correlation_id,
      verification: this.verification,
    };
  }
}

export interface DeploymentSelectionRequestShape {
  profile: DeploymentProfileShape;
  correlation_id: string;
}

export interface DeploymentVerificationRequestShape {
  correlation_id: string;
  state: DeploymentVerificationState;
  evidence?: DeploymentVerificationEvidenceShape | null;
}

const DEPLOYMENT_SELECTION_FIELDS = new Set<string>([
  "profile",
  "correlation_id",
]);
const DEPLOYMENT_VERIFICATION_REQUEST_FIELDS = new Set<string>([
  "correlation_id",
  "state",
  "evidence",
]);

export class DeploymentSelectionRequest {
  readonly profile: DeploymentProfile;
  readonly correlation_id: string;

  private constructor(profile: DeploymentProfile, correlationId: string) {
    this.profile = profile;
    this.correlation_id = correlationId;
  }

  static parse(value: unknown): DeploymentSelectionRequest {
    const obj = assertObject(value, "deployment selection request");
    rejectUnknownFields(
      obj,
      DEPLOYMENT_SELECTION_FIELDS,
      "deployment selection request",
    );
    return new DeploymentSelectionRequest(
      DeploymentProfile.parse(
        obj["profile"],
        "deployment selection request.profile",
      ),
      assertUuid(
        obj["correlation_id"],
        "deployment selection request.correlation_id",
      ),
    );
  }
}

export class DeploymentVerificationRequest {
  readonly correlation_id: string;
  readonly state: DeploymentVerificationState;
  readonly evidence: DeploymentVerificationEvidenceShape | undefined;

  private constructor(
    correlationId: string,
    state: DeploymentVerificationState,
    evidence: DeploymentVerificationEvidenceShape | undefined,
  ) {
    this.correlation_id = correlationId;
    this.state = state;
    this.evidence = evidence;
  }

  static parse(value: unknown): DeploymentVerificationRequest {
    const obj = assertObject(value, "deployment verification request");
    rejectUnknownFields(
      obj,
      DEPLOYMENT_VERIFICATION_REQUEST_FIELDS,
      "deployment verification request",
    );
    const state = assertEnum(
      obj["state"],
      DEPLOYMENT_VERIFICATION_STATE_SET,
      "deployment verification request.state",
    );
    const evidenceRaw = obj["evidence"];
    const evidence =
      evidenceRaw === undefined || evidenceRaw === null
        ? undefined
        : parseDeploymentVerificationEvidence(
            evidenceRaw,
            "deployment verification request.evidence",
          );
    if (state === "VERIFIED" && evidence === undefined) {
      throw new Spec006Error(
        ErrorCode.Verification,
        "deployment verification request claims VERIFIED without evidence",
      );
    }
    if (state !== "VERIFIED" && evidence !== undefined) {
      throw new Spec006Error(
        ErrorCode.Validation,
        "deployment verification request carries evidence for a non-VERIFIED state",
      );
    }
    return new DeploymentVerificationRequest(
      assertUuid(
        obj["correlation_id"],
        "deployment verification request.correlation_id",
      ),
      state,
      evidence,
    );
  }
}

/** Provider-neutral DeploymentChoice port. M1 declares the boundary. */
export interface DeploymentChoicePort {
  select(request: DeploymentSelectionRequestShape): DeploymentIntentRecordShape;
  verify(
    request: DeploymentVerificationRequestShape,
  ): DeploymentIntentRecordShape;
}
