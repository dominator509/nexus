/**
 * EP-042 M2 canonical release/update wire types (SPEC-016, SPEC-024).
 *
 * These types are the TypeScript boundary adaptation of the canonical
 * M1 contract surface in crates/nexus-release (model.rs, vocabulary.rs).
 * Field names and enum values are the canonical snake_case / SCREAMING
 * wire forms from the Rust serde serialization verbatim. Canonical truth
 * remains in crates/nexus-release; this module only adapts the wire
 * format at the boundary and never creates parallel release/update
 * domain models.
 *
 * Every parse path is deny-unknown: unknown fields and unknown vocabulary
 * values are rejected, never silently accepted.
 */

import { ReleaseError, ReleaseErrorCode } from "./errors";
import {
  assertEnum,
  assertIso8601Timestamp,
  assertNonEmptyString,
  assertNonNegativeInt,
  assertObject,
  assertOptionalObject,
  assertOptionalString,
  assertOptionalStringArray,
  assertString,
  assertStringArray,
  assertU32,
  isBase64,
  isHex,
  rejectUnknownFields,
} from "./validate";

// Canonical deployment profile modes and release channels are owned by
// the EP-035 deployment contract (mirrored from
// schemas/deployment-profile.schema.json). The update core reuses those
// canonical bindings - it never duplicates canonical domain names.
import {
  DEPLOYMENT_MODES as DEPLOYMENT_PROFILE_MODES,
  RELEASE_CHANNELS,
  type DeploymentMode as DeploymentProfileMode,
  type ReleaseChannel,
} from "../contracts/deployment";

export { DEPLOYMENT_PROFILE_MODES, RELEASE_CHANNELS };
export type { DeploymentProfileMode, ReleaseChannel };

const DEPLOYMENT_PROFILE_MODE_SET: ReadonlySet<DeploymentProfileMode> = new Set(
  DEPLOYMENT_PROFILE_MODES,
);
const RELEASE_CHANNEL_SET: ReadonlySet<ReleaseChannel> = new Set(
  RELEASE_CHANNELS,
);

export const SIGNATURE_ALGORITHMS = ["ED25519"] as const;
export type SignatureAlgorithm = (typeof SIGNATURE_ALGORITHMS)[number];
const SIGNATURE_ALGORITHM_SET: ReadonlySet<SignatureAlgorithm> = new Set(
  SIGNATURE_ALGORITHMS,
);

export const SIGNATURE_STATES = [
  "UNVERIFIED",
  "PRESENT",
  "VALID",
  "INVALID",
] as const;
export type SignatureState = (typeof SIGNATURE_STATES)[number];
const SIGNATURE_STATE_SET: ReadonlySet<SignatureState> = new Set(
  SIGNATURE_STATES,
);

/**
 * Update step kinds. Deliberately NO PROMOTE: production promotion is an
 * exact manual action outside the transactional update plan.
 */
export const UPDATE_STEP_KINDS = [
  "BACKUP",
  "MIGRATE",
  "CANARY",
  "OBSERVE",
  "ROLLBACK",
] as const;
export type UpdateStepKind = (typeof UPDATE_STEP_KINDS)[number];
const UPDATE_STEP_KIND_SET: ReadonlySet<UpdateStepKind> = new Set(
  UPDATE_STEP_KINDS,
);

export const UPDATE_STATES = [
  "PLANNED",
  "PENDING",
  "IN_PROGRESS",
  "OBSERVING",
  "READY_TO_PROMOTE",
  "ROLLED_BACK",
  "FAILED",
] as const;
export type UpdateState = (typeof UPDATE_STATES)[number];
const UPDATE_STATE_SET: ReadonlySet<UpdateState> = new Set(UPDATE_STATES);

/** Canary verdicts. Deliberately NO PROMOTED: canaries never promote. */
export const CANARY_VERDICTS = [
  "OBSERVING",
  "READY_TO_PROMOTE",
  "ROLLBACK",
] as const;
export type CanaryVerdict = (typeof CANARY_VERDICTS)[number];
const CANARY_VERDICT_SET: ReadonlySet<CanaryVerdict> = new Set(CANARY_VERDICTS);

export const VERIFICATION_STATES = [
  "UNVERIFIED",
  "VERIFIED",
  "MISMATCH",
  "MISSING",
] as const;
export type VerificationState = (typeof VERIFICATION_STATES)[number];
const VERIFICATION_STATE_SET: ReadonlySet<VerificationState> = new Set(
  VERIFICATION_STATES,
);

export const BUNDLE_KINDS = [
  "IMAGE",
  "MODEL",
  "LICENSE",
  "SBOM",
  "MIGRATION",
  "RECOVERY_TOOL",
] as const;
export type BundleKind = (typeof BUNDLE_KINDS)[number];
const BUNDLE_KIND_SET: ReadonlySet<BundleKind> = new Set(BUNDLE_KINDS);

export const ROLLBACK_STATES = [
  "REQUIRES_BACKUP",
  "BACKUP_VERIFIED",
  "ROLLBACK_VERIFIED",
  "FAILED",
] as const;
export type RollbackState = (typeof ROLLBACK_STATES)[number];
const ROLLBACK_STATE_SET: ReadonlySet<RollbackState> = new Set(ROLLBACK_STATES);

export const PROMOTION_STATES = [
  "LOCKED",
  "AWAITING_HUMAN_APPROVAL",
  "APPROVED_MANUAL_ONLY",
] as const;
export type PromotionState = (typeof PROMOTION_STATES)[number];
const PROMOTION_STATE_SET: ReadonlySet<PromotionState> = new Set(
  PROMOTION_STATES,
);

// ---- Primitive value objects --------------------------------------------

/**
 * Canonical content digest in `alg:hex` form (SPEC-024; EP-041 identity
 * precedent). Accepted: `sha256:` followed by >= 32 lowercase hex chars.
 */
export class Digest {
  readonly raw: string;

  private constructor(raw: string) {
    this.raw = raw;
  }

  static parse(value: unknown, what = "digest"): Digest {
    const raw = assertNonEmptyString(value, what);
    const sep = raw.indexOf(":");
    if (sep <= 0) {
      throw new ReleaseError(
        ReleaseErrorCode.DigestMismatch,
        `${what} must be alg:hex`,
        { field: what },
      );
    }
    const alg = raw.slice(0, sep);
    const hex = raw.slice(sep + 1);
    if (alg !== "sha256") {
      throw new ReleaseError(
        ReleaseErrorCode.DigestMismatch,
        `${what} unsupported digest algorithm: ${alg}`,
        { field: what },
      );
    }
    if (hex.length < 32 || !isHex(hex) || /[A-F]/.test(hex)) {
      throw new ReleaseError(
        ReleaseErrorCode.DigestMismatch,
        `${what} must be sha256 lowercase hex with at least 32 characters`,
        { field: what },
      );
    }
    return new Digest(raw);
  }

  alg(): string {
    return this.raw.slice(0, this.raw.indexOf(":"));
  }

  hex(): string {
    return this.raw.slice(this.raw.indexOf(":") + 1);
  }

  asString(): string {
    return this.raw;
  }

  equals(other: Digest): boolean {
    return this.raw === other.raw;
  }
}

/** Reference to an object in the ArtifactStore (SPEC-024). */
export interface ObjectRef {
  backend: string;
  key: string;
}

const OBJECT_REF_FIELDS: ReadonlySet<string> = new Set(["backend", "key"]);

export function parseObjectRef(value: unknown, what: string): ObjectRef {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, OBJECT_REF_FIELDS, what);
  return {
    backend: assertNonEmptyString(obj["backend"], `${what}.backend`),
    key: assertNonEmptyString(obj["key"], `${what}.key`),
  };
}

/**
 * Signature envelope. Presence is not validity: SignatureState is
 * produced by verification, never by construction.
 */
export interface Signature {
  algorithm: SignatureAlgorithm;
  key_id: string;
  value_b64: string;
}

const SIGNATURE_FIELDS: ReadonlySet<string> = new Set([
  "algorithm",
  "key_id",
  "value_b64",
]);

export function parseSignature(value: unknown, what: string): Signature {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, SIGNATURE_FIELDS, what);
  const valueB64 = assertNonEmptyString(obj["value_b64"], `${what}.value_b64`);
  if (!isBase64(valueB64)) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what}.value_b64 must be base64`,
      { field: `${what}.value_b64` },
    );
  }
  return {
    algorithm: assertEnum(
      obj["algorithm"],
      SIGNATURE_ALGORITHM_SET,
      `${what}.algorithm`,
    ),
    key_id: assertNonEmptyString(obj["key_id"], `${what}.key_id`),
    value_b64: valueB64,
  };
}

/** A constructed signature is at most PRESENT: no verifier exists here. */
export function signatureState(_signature: Signature): SignatureState {
  return "PRESENT";
}

// ---- SignedComponent -----------------------------------------------------

export interface SignedComponent {
  component_id: string;
  name: string;
  version: string;
  artifact_ref: ObjectRef;
  digest: string;
  signature: Signature;
  sbom_ref: ObjectRef;
  license_ref: string;
  size_bytes: number;
}

const SIGNED_COMPONENT_FIELDS: ReadonlySet<string> = new Set([
  "component_id",
  "name",
  "version",
  "artifact_ref",
  "digest",
  "signature",
  "sbom_ref",
  "license_ref",
  "size_bytes",
]);

export function parseSignedComponent(
  value: unknown,
  what: string,
): SignedComponent {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, SIGNED_COMPONENT_FIELDS, what);
  const digest = Digest.parse(obj["digest"], `${what}.digest`);
  return {
    component_id: assertNonEmptyString(
      obj["component_id"],
      `${what}.component_id`,
    ),
    name: assertNonEmptyString(obj["name"], `${what}.name`),
    version: assertNonEmptyString(obj["version"], `${what}.version`),
    artifact_ref: parseObjectRef(obj["artifact_ref"], `${what}.artifact_ref`),
    digest: digest.asString(),
    signature: parseSignature(obj["signature"], `${what}.signature`),
    sbom_ref: parseObjectRef(obj["sbom_ref"], `${what}.sbom_ref`),
    license_ref: assertNonEmptyString(
      obj["license_ref"],
      `${what}.license_ref`,
    ),
    size_bytes: assertNonNegativeInt(obj["size_bytes"], `${what}.size_bytes`),
  };
}

// ---- Compatibility -------------------------------------------------------

export interface CompatibilityEntry {
  component_id: string;
  version: string;
  min_version: string;
  max_version: string;
  supported_profiles: ReadonlyArray<DeploymentProfileMode>;
}

const COMPATIBILITY_ENTRY_FIELDS: ReadonlySet<string> = new Set([
  "component_id",
  "version",
  "min_version",
  "max_version",
  "supported_profiles",
]);

export function parseCompatibilityEntry(
  value: unknown,
  what: string,
): CompatibilityEntry {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, COMPATIBILITY_ENTRY_FIELDS, what);
  const profiles = obj["supported_profiles"];
  if (!Array.isArray(profiles) || profiles.length === 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what}.supported_profiles must be a non-empty array`,
      { field: `${what}.supported_profiles` },
    );
  }
  return {
    component_id: assertNonEmptyString(
      obj["component_id"],
      `${what}.component_id`,
    ),
    version: assertNonEmptyString(obj["version"], `${what}.version`),
    min_version: assertNonEmptyString(
      obj["min_version"],
      `${what}.min_version`,
    ),
    max_version: assertNonEmptyString(
      obj["max_version"],
      `${what}.max_version`,
    ),
    supported_profiles: profiles.map((entry, index) =>
      assertEnum(
        entry,
        DEPLOYMENT_PROFILE_MODE_SET,
        `${what}.supported_profiles[${index}]`,
      ),
    ),
  };
}

export interface CompatibilityMatrix {
  matrix_id: string;
  schema_version: number;
  entries: ReadonlyArray<CompatibilityEntry>;
}

const COMPATIBILITY_MATRIX_FIELDS: ReadonlySet<string> = new Set([
  "matrix_id",
  "schema_version",
  "entries",
]);

export function parseCompatibilityMatrix(
  value: unknown,
  what: string,
): CompatibilityMatrix {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, COMPATIBILITY_MATRIX_FIELDS, what);
  const entriesRaw = obj["entries"];
  if (!Array.isArray(entriesRaw) || entriesRaw.length === 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what}.entries must be a non-empty array`,
      { field: `${what}.entries` },
    );
  }
  const entries = entriesRaw.map((entry, index) =>
    parseCompatibilityEntry(entry, `${what}.entries[${index}]`),
  );
  const seen = new Set<string>();
  for (const entry of entries) {
    if (seen.has(entry.component_id)) {
      throw new ReleaseError(
        ReleaseErrorCode.Validation,
        `${what} duplicate component entry: ${entry.component_id}`,
        { field: `${what}.entries` },
      );
    }
    seen.add(entry.component_id);
  }
  const schemaVersion = assertU32(
    obj["schema_version"],
    `${what}.schema_version`,
  );
  if (schemaVersion !== 1) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} unsupported schema_version: ${schemaVersion}`,
      { field: `${what}.schema_version` },
    );
  }
  return {
    matrix_id: assertNonEmptyString(obj["matrix_id"], `${what}.matrix_id`),
    schema_version: schemaVersion,
    entries,
  };
}

// ---- ReleaseManifest -----------------------------------------------------

export interface ReleaseManifest {
  schema_version: number;
  release_id: string;
  version: string;
  channel: ReleaseChannel;
  components: ReadonlyArray<SignedComponent>;
  compatibility: CompatibilityMatrix;
  offline_bundle_ref?: ObjectRef;
  sbom_ref: ObjectRef;
  license_refs: ReadonlyArray<string>;
  created_at: string;
  manifest_digest?: string;
}

const RELEASE_MANIFEST_FIELDS: ReadonlySet<string> = new Set([
  "schema_version",
  "release_id",
  "version",
  "channel",
  "components",
  "compatibility",
  "offline_bundle_ref",
  "sbom_ref",
  "license_refs",
  "created_at",
  "manifest_digest",
]);

export function parseReleaseManifest(
  value: unknown,
  what = "release manifest",
): ReleaseManifest {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, RELEASE_MANIFEST_FIELDS, what);
  const componentsRaw = obj["components"];
  if (!Array.isArray(componentsRaw) || componentsRaw.length === 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must contain at least one signed component`,
      { field: `${what}.components` },
    );
  }
  const components = componentsRaw.map((entry, index) =>
    parseSignedComponent(entry, `${what}.components[${index}]`),
  );
  const licenseRefs = obj["license_refs"];
  if (!Array.isArray(licenseRefs) || licenseRefs.length === 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must reference at least one license`,
      { field: `${what}.license_refs` },
    );
  }
  const schemaVersion = assertU32(
    obj["schema_version"],
    `${what}.schema_version`,
  );
  if (schemaVersion !== 1) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} unsupported schema_version: ${schemaVersion}`,
      { field: `${what}.schema_version` },
    );
  }
  const manifest: ReleaseManifest = {
    schema_version: schemaVersion,
    release_id: assertNonEmptyString(obj["release_id"], `${what}.release_id`),
    version: assertNonEmptyString(obj["version"], `${what}.version`),
    channel: assertEnum(obj["channel"], RELEASE_CHANNEL_SET, `${what}.channel`),
    components,
    compatibility: parseCompatibilityMatrix(
      obj["compatibility"],
      `${what}.compatibility`,
    ),
    sbom_ref: parseObjectRef(obj["sbom_ref"], `${what}.sbom_ref`),
    license_refs: licenseRefs.map((entry, index) =>
      assertNonEmptyString(entry, `${what}.license_refs[${index}]`),
    ),
    created_at: assertIso8601Timestamp(obj["created_at"], `${what}.created_at`),
  };
  const offlineBundleRef = assertOptionalObject(
    obj["offline_bundle_ref"],
    `${what}.offline_bundle_ref`,
  );
  if (offlineBundleRef !== undefined) {
    manifest.offline_bundle_ref = parseObjectRef(
      offlineBundleRef,
      `${what}.offline_bundle_ref`,
    );
  }
  const manifestDigest = assertOptionalString(
    obj["manifest_digest"],
    `${what}.manifest_digest`,
  );
  if (manifestDigest !== undefined) {
    manifest.manifest_digest = Digest.parse(
      manifestDigest,
      `${what}.manifest_digest`,
    ).asString();
  }
  return manifest;
}

// ---- UpdatePlan ----------------------------------------------------------

export interface UpdateStep {
  order: number;
  kind: UpdateStepKind;
  description: string;
}

const UPDATE_STEP_FIELDS: ReadonlySet<string> = new Set([
  "order",
  "kind",
  "description",
]);

export function parseUpdateStep(value: unknown, what: string): UpdateStep {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, UPDATE_STEP_FIELDS, what);
  return {
    order: assertNonNegativeInt(obj["order"], `${what}.order`),
    kind: assertEnum(obj["kind"], UPDATE_STEP_KIND_SET, `${what}.kind`),
    description: assertNonEmptyString(
      obj["description"],
      `${what}.description`,
    ),
  };
}

export interface UpdatePlan {
  schema_version: number;
  plan_id: string;
  release_id: string;
  from_version: string;
  to_version: string;
  channel: ReleaseChannel;
  steps: ReadonlyArray<UpdateStep>;
  idempotency_key: string;
  correlation_id: string;
  created_at: string;
  state: UpdateState;
}

const UPDATE_PLAN_FIELDS: ReadonlySet<string> = new Set([
  "schema_version",
  "plan_id",
  "release_id",
  "from_version",
  "to_version",
  "channel",
  "steps",
  "idempotency_key",
  "correlation_id",
  "created_at",
  "state",
]);

export function parseUpdatePlan(
  value: unknown,
  what = "update plan",
): UpdatePlan {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, UPDATE_PLAN_FIELDS, what);
  const stepsRaw = obj["steps"];
  if (!Array.isArray(stepsRaw) || stepsRaw.length === 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must contain at least one step`,
      { field: `${what}.steps` },
    );
  }
  const steps = stepsRaw.map((entry, index) =>
    parseUpdateStep(entry, `${what}.steps[${index}]`),
  );
  for (let i = 0; i < steps.length; i += 1) {
    const step = steps[i];
    if (step === undefined || step.order !== i + 1) {
      throw new ReleaseError(
        ReleaseErrorCode.Validation,
        `${what} step order must be contiguous starting at 1`,
        { field: `${what}.steps.order` },
      );
    }
  }
  // backup-before-update: the first step is always a backup (M1
  // contract parity; SPEC-016 behavior 6).
  const firstStep = steps[0];
  if (firstStep === undefined || firstStep.kind !== "BACKUP") {
    throw new ReleaseError(
      ReleaseErrorCode.BackupRequired,
      `${what} first step must be a backup`,
      { field: `${what}.steps[0].kind` },
    );
  }
  const schemaVersion = assertU32(
    obj["schema_version"],
    `${what}.schema_version`,
  );
  if (schemaVersion !== 1) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} unsupported schema_version: ${schemaVersion}`,
      { field: `${what}.schema_version` },
    );
  }
  const state = assertEnum(obj["state"], UPDATE_STATE_SET, `${what}.state`);
  if (state !== "PLANNED") {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} initial state must be PLANNED`,
      { field: `${what}.state` },
    );
  }
  return {
    schema_version: schemaVersion,
    plan_id: assertNonEmptyString(obj["plan_id"], `${what}.plan_id`),
    release_id: assertNonEmptyString(obj["release_id"], `${what}.release_id`),
    from_version: assertNonEmptyString(
      obj["from_version"],
      `${what}.from_version`,
    ),
    to_version: assertNonEmptyString(obj["to_version"], `${what}.to_version`),
    channel: assertEnum(obj["channel"], RELEASE_CHANNEL_SET, `${what}.channel`),
    steps,
    idempotency_key: assertNonEmptyString(
      obj["idempotency_key"],
      `${what}.idempotency_key`,
    ),
    correlation_id: assertNonEmptyString(
      obj["correlation_id"],
      `${what}.correlation_id`,
    ),
    created_at: assertIso8601Timestamp(obj["created_at"], `${what}.created_at`),
    state,
  };
}

// ---- CanaryRing ----------------------------------------------------------

export interface CanaryRing {
  schema_version: number;
  ring_id: string;
  release_id: string;
  profile: DeploymentProfileMode;
  cohort_percent: number;
  observation_minutes: number;
  health_criterion: string;
  verdict: CanaryVerdict;
  observed_at?: string;
  evidence_ref?: string;
}

const CANARY_RING_FIELDS: ReadonlySet<string> = new Set([
  "schema_version",
  "ring_id",
  "release_id",
  "profile",
  "cohort_percent",
  "observation_minutes",
  "health_criterion",
  "verdict",
  "observed_at",
  "evidence_ref",
]);

export function parseCanaryRing(
  value: unknown,
  what = "canary ring",
): CanaryRing {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, CANARY_RING_FIELDS, what);
  const cohortPercent = assertNonNegativeInt(
    obj["cohort_percent"],
    `${what}.cohort_percent`,
  );
  if (cohortPercent < 1 || cohortPercent > 100) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what}.cohort_percent must be between 1 and 100`,
      { field: `${what}.cohort_percent` },
    );
  }
  const observationMinutes = assertNonNegativeInt(
    obj["observation_minutes"],
    `${what}.observation_minutes`,
  );
  if (observationMinutes === 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what}.observation_minutes must be positive`,
      { field: `${what}.observation_minutes` },
    );
  }
  const schemaVersion = assertU32(
    obj["schema_version"],
    `${what}.schema_version`,
  );
  if (schemaVersion !== 1) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} unsupported schema_version: ${schemaVersion}`,
      { field: `${what}.schema_version` },
    );
  }
  const ring: CanaryRing = {
    schema_version: schemaVersion,
    ring_id: assertNonEmptyString(obj["ring_id"], `${what}.ring_id`),
    release_id: assertNonEmptyString(obj["release_id"], `${what}.release_id`),
    profile: assertEnum(
      obj["profile"],
      DEPLOYMENT_PROFILE_MODE_SET,
      `${what}.profile`,
    ),
    cohort_percent: cohortPercent,
    observation_minutes: observationMinutes,
    health_criterion: assertNonEmptyString(
      obj["health_criterion"],
      `${what}.health_criterion`,
    ),
    verdict: assertEnum(obj["verdict"], CANARY_VERDICT_SET, `${what}.verdict`),
  };
  const observedAt = assertOptionalString(
    obj["observed_at"],
    `${what}.observed_at`,
  );
  if (observedAt !== undefined) {
    ring.observed_at = assertIso8601Timestamp(
      observedAt,
      `${what}.observed_at`,
    );
  }
  const evidenceRef = assertOptionalString(
    obj["evidence_ref"],
    `${what}.evidence_ref`,
  );
  if (evidenceRef !== undefined) {
    ring.evidence_ref = evidenceRef;
  }
  return ring;
}

// ---- RollbackReceipt -----------------------------------------------------

export interface RollbackReceipt {
  schema_version: number;
  receipt_id: string;
  update_plan_ref: string;
  from_version: string;
  to_version: string;
  backup_ref: ObjectRef;
  backup_verification: VerificationState;
  rollback_verification: VerificationState;
  state: RollbackState;
  actor: string;
  correlation_id: string;
  verified_at?: string;
}

const ROLLBACK_RECEIPT_FIELDS: ReadonlySet<string> = new Set([
  "schema_version",
  "receipt_id",
  "update_plan_ref",
  "from_version",
  "to_version",
  "backup_ref",
  "backup_verification",
  "rollback_verification",
  "state",
  "actor",
  "correlation_id",
  "verified_at",
]);

export function parseRollbackReceipt(
  value: unknown,
  what = "rollback receipt",
): RollbackReceipt {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, ROLLBACK_RECEIPT_FIELDS, what);
  const schemaVersion = assertU32(
    obj["schema_version"],
    `${what}.schema_version`,
  );
  if (schemaVersion !== 1) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} unsupported schema_version: ${schemaVersion}`,
      { field: `${what}.schema_version` },
    );
  }
  const receipt: RollbackReceipt = {
    schema_version: schemaVersion,
    receipt_id: assertNonEmptyString(obj["receipt_id"], `${what}.receipt_id`),
    update_plan_ref: assertNonEmptyString(
      obj["update_plan_ref"],
      `${what}.update_plan_ref`,
    ),
    from_version: assertNonEmptyString(
      obj["from_version"],
      `${what}.from_version`,
    ),
    to_version: assertNonEmptyString(obj["to_version"], `${what}.to_version`),
    backup_ref: parseObjectRef(obj["backup_ref"], `${what}.backup_ref`),
    backup_verification: assertEnum(
      obj["backup_verification"],
      VERIFICATION_STATE_SET,
      `${what}.backup_verification`,
    ),
    rollback_verification: assertEnum(
      obj["rollback_verification"],
      VERIFICATION_STATE_SET,
      `${what}.rollback_verification`,
    ),
    state: assertEnum(obj["state"], ROLLBACK_STATE_SET, `${what}.state`),
    actor: assertNonEmptyString(obj["actor"], `${what}.actor`),
    correlation_id: assertNonEmptyString(
      obj["correlation_id"],
      `${what}.correlation_id`,
    ),
  };
  const verifiedAt = assertOptionalString(
    obj["verified_at"],
    `${what}.verified_at`,
  );
  if (verifiedAt !== undefined) {
    receipt.verified_at = assertIso8601Timestamp(
      verifiedAt,
      `${what}.verified_at`,
    );
  }
  return receipt;
}

// ---- OfflineBundle -------------------------------------------------------

export interface BundleItem {
  kind: BundleKind;
  name: string;
  digest: string;
}

const BUNDLE_ITEM_FIELDS: ReadonlySet<string> = new Set([
  "kind",
  "name",
  "digest",
]);

export function parseBundleItem(value: unknown, what: string): BundleItem {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, BUNDLE_ITEM_FIELDS, what);
  return {
    kind: assertEnum(obj["kind"], BUNDLE_KIND_SET, `${what}.kind`),
    name: assertNonEmptyString(obj["name"], `${what}.name`),
    digest: Digest.parse(obj["digest"], `${what}.digest`).asString(),
  };
}

export interface OfflineBundle {
  schema_version: number;
  bundle_id: string;
  release_id: string;
  contents: ReadonlyArray<BundleItem>;
  manifest_ref: ObjectRef;
  sbom_refs: ReadonlyArray<string>;
  license_refs: ReadonlyArray<string>;
  migrations: ReadonlyArray<string>;
  bundle_digest?: string;
}

const OFFLINE_BUNDLE_FIELDS: ReadonlySet<string> = new Set([
  "schema_version",
  "bundle_id",
  "release_id",
  "contents",
  "manifest_ref",
  "sbom_refs",
  "license_refs",
  "migrations",
  "bundle_digest",
]);

export function parseOfflineBundle(
  value: unknown,
  what = "offline bundle",
): OfflineBundle {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, OFFLINE_BUNDLE_FIELDS, what);
  const contentsRaw = obj["contents"];
  if (!Array.isArray(contentsRaw) || contentsRaw.length === 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must contain at least one item`,
      { field: `${what}.contents` },
    );
  }
  const contents = contentsRaw.map((entry, index) =>
    parseBundleItem(entry, `${what}.contents[${index}]`),
  );
  const kinds = new Set(contents.map((item) => item.kind));
  for (const required of ["IMAGE", "MODEL", "LICENSE", "SBOM"] as const) {
    if (!kinds.has(required)) {
      throw new ReleaseError(
        ReleaseErrorCode.Validation,
        `${what} missing required content kind: ${required}`,
        { field: `${what}.contents` },
      );
    }
  }
  const sbomRefs = obj["sbom_refs"];
  if (!Array.isArray(sbomRefs) || sbomRefs.length === 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must reference at least one SBOM`,
      { field: `${what}.sbom_refs` },
    );
  }
  const licenseRefs = obj["license_refs"];
  if (!Array.isArray(licenseRefs) || licenseRefs.length === 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} must reference at least one license`,
      { field: `${what}.license_refs` },
    );
  }
  const schemaVersion = assertU32(
    obj["schema_version"],
    `${what}.schema_version`,
  );
  if (schemaVersion !== 1) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} unsupported schema_version: ${schemaVersion}`,
      { field: `${what}.schema_version` },
    );
  }
  const bundle: OfflineBundle = {
    schema_version: schemaVersion,
    bundle_id: assertNonEmptyString(obj["bundle_id"], `${what}.bundle_id`),
    release_id: assertNonEmptyString(obj["release_id"], `${what}.release_id`),
    contents,
    manifest_ref: parseObjectRef(obj["manifest_ref"], `${what}.manifest_ref`),
    sbom_refs: sbomRefs.map((entry, index) =>
      assertNonEmptyString(entry, `${what}.sbom_refs[${index}]`),
    ),
    license_refs: licenseRefs.map((entry, index) =>
      assertNonEmptyString(entry, `${what}.license_refs[${index}]`),
    ),
    migrations: assertStringArray(obj["migrations"], `${what}.migrations`),
  };
  const bundleDigest = assertOptionalString(
    obj["bundle_digest"],
    `${what}.bundle_digest`,
  );
  if (bundleDigest !== undefined) {
    bundle.bundle_digest = Digest.parse(
      bundleDigest,
      `${what}.bundle_digest`,
    ).asString();
  }
  return bundle;
}

// ---- ManualPromotion -----------------------------------------------------

export interface ManualPromotion {
  schema_version: number;
  promotion_id: string;
  release_id: string;
  update_plan_ref: string;
  canary_ring_ref: string;
  approval_ref: string;
  approver: string;
  approved_at: string;
  state: PromotionState;
  exact_manual_command: string;
}

const MANUAL_PROMOTION_FIELDS: ReadonlySet<string> = new Set([
  "schema_version",
  "promotion_id",
  "release_id",
  "update_plan_ref",
  "canary_ring_ref",
  "approval_ref",
  "approver",
  "approved_at",
  "state",
  "exact_manual_command",
]);

export function parseManualPromotion(
  value: unknown,
  what = "manual promotion",
): ManualPromotion {
  const obj = assertObject(value, what);
  rejectUnknownFields(obj, MANUAL_PROMOTION_FIELDS, what);
  const schemaVersion = assertU32(
    obj["schema_version"],
    `${what}.schema_version`,
  );
  if (schemaVersion !== 1) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `${what} unsupported schema_version: ${schemaVersion}`,
      { field: `${what}.schema_version` },
    );
  }
  return {
    schema_version: schemaVersion,
    promotion_id: assertNonEmptyString(
      obj["promotion_id"],
      `${what}.promotion_id`,
    ),
    release_id: assertNonEmptyString(obj["release_id"], `${what}.release_id`),
    update_plan_ref: assertNonEmptyString(
      obj["update_plan_ref"],
      `${what}.update_plan_ref`,
    ),
    canary_ring_ref: assertNonEmptyString(
      obj["canary_ring_ref"],
      `${what}.canary_ring_ref`,
    ),
    approval_ref: assertNonEmptyString(
      obj["approval_ref"],
      `${what}.approval_ref`,
    ),
    approver: assertNonEmptyString(obj["approver"], `${what}.approver`),
    approved_at: assertIso8601Timestamp(
      obj["approved_at"],
      `${what}.approved_at`,
    ),
    state: assertEnum(obj["state"], PROMOTION_STATE_SET, `${what}.state`),
    exact_manual_command: assertNonEmptyString(
      obj["exact_manual_command"],
      `${what}.exact_manual_command`,
    ),
  };
}

// ---- Boundary helpers ----------------------------------------------------

export function assertStringEnumValue<T extends string>(
  value: unknown,
  allowed: ReadonlySet<T>,
  what: string,
): T {
  return assertEnum(value, allowed, what);
}

export function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim() !== "";
}

export function isU32(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isInteger(value) &&
    value >= 0 &&
    value <= 0xffff_ffff
  );
}

export function isIso8601Timestamp(value: unknown): value is string {
  return typeof value === "string" && /^\d{4}-\d{2}-\d{2}/.test(value);
}

export function isDigestString(value: unknown): value is string {
  if (typeof value !== "string") {
    return false;
  }
  try {
    Digest.parse(value, "digest");
    return true;
  } catch {
    return false;
  }
}

export function isVerificationState(
  value: unknown,
): value is VerificationState {
  return (
    typeof value === "string" &&
    VERIFICATION_STATE_SET.has(value as VerificationState)
  );
}

export function isRollbackState(value: unknown): value is RollbackState {
  return (
    typeof value === "string" && ROLLBACK_STATE_SET.has(value as RollbackState)
  );
}

export function isPromotionState(value: unknown): value is PromotionState {
  return (
    typeof value === "string" &&
    PROMOTION_STATE_SET.has(value as PromotionState)
  );
}

export function isCanaryVerdict(value: unknown): value is CanaryVerdict {
  return (
    typeof value === "string" && CANARY_VERDICT_SET.has(value as CanaryVerdict)
  );
}

export function isUpdateStepKind(value: unknown): value is UpdateStepKind {
  return (
    typeof value === "string" &&
    UPDATE_STEP_KIND_SET.has(value as UpdateStepKind)
  );
}

export function isUpdateState(value: unknown): value is UpdateState {
  return (
    typeof value === "string" && UPDATE_STATE_SET.has(value as UpdateState)
  );
}

export function isSignatureState(value: unknown): value is SignatureState {
  return (
    typeof value === "string" &&
    SIGNATURE_STATE_SET.has(value as SignatureState)
  );
}

export function isDeploymentProfileMode(
  value: unknown,
): value is DeploymentProfileMode {
  return (
    typeof value === "string" &&
    DEPLOYMENT_PROFILE_MODE_SET.has(value as DeploymentProfileMode)
  );
}

export function isReleaseChannel(value: unknown): value is ReleaseChannel {
  return (
    typeof value === "string" &&
    RELEASE_CHANNEL_SET.has(value as ReleaseChannel)
  );
}

export function isBundleKind(value: unknown): value is BundleKind {
  return typeof value === "string" && BUNDLE_KIND_SET.has(value as BundleKind);
}

export function isSignatureAlgorithm(
  value: unknown,
): value is SignatureAlgorithm {
  return (
    typeof value === "string" &&
    SIGNATURE_ALGORITHM_SET.has(value as SignatureAlgorithm)
  );
}

export function isObjectRef(value: unknown): value is ObjectRef {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const obj = value as Record<string, unknown>;
  return isNonEmptyString(obj["backend"]) && isNonEmptyString(obj["key"]);
}

export function isSignedComponent(value: unknown): value is SignedComponent {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const obj = value as Record<string, unknown>;
  return (
    isNonEmptyString(obj["component_id"]) &&
    isNonEmptyString(obj["version"]) &&
    isDigestString(obj["digest"]) &&
    typeof obj["signature"] === "object" &&
    obj["signature"] !== null
  );
}

export function isReleaseManifest(value: unknown): value is ReleaseManifest {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const obj = value as Record<string, unknown>;
  return (
    isU32(obj["schema_version"]) &&
    obj["schema_version"] === 1 &&
    isNonEmptyString(obj["release_id"]) &&
    isReleaseChannel(obj["channel"]) &&
    Array.isArray(obj["components"]) &&
    obj["components"].length > 0 &&
    typeof obj["compatibility"] === "object" &&
    obj["compatibility"] !== null &&
    isObjectRef(obj["sbom_ref"]) &&
    Array.isArray(obj["license_refs"]) &&
    obj["license_refs"].length > 0 &&
    isIso8601Timestamp(obj["created_at"])
  );
}

export function isUpdatePlan(value: unknown): value is UpdatePlan {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const obj = value as Record<string, unknown>;
  return (
    isU32(obj["schema_version"]) &&
    obj["schema_version"] === 1 &&
    isNonEmptyString(obj["plan_id"]) &&
    isReleaseChannel(obj["channel"]) &&
    Array.isArray(obj["steps"]) &&
    obj["steps"].length > 0 &&
    isNonEmptyString(obj["idempotency_key"]) &&
    isUpdateState(obj["state"]) &&
    obj["state"] === "PLANNED"
  );
}

export function isCanaryRing(value: unknown): value is CanaryRing {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const obj = value as Record<string, unknown>;
  return (
    isU32(obj["schema_version"]) &&
    obj["schema_version"] === 1 &&
    isNonEmptyString(obj["ring_id"]) &&
    isDeploymentProfileMode(obj["profile"]) &&
    typeof obj["cohort_percent"] === "number" &&
    (obj["cohort_percent"] as number) >= 1 &&
    (obj["cohort_percent"] as number) <= 100 &&
    isCanaryVerdict(obj["verdict"])
  );
}

export function isRollbackReceipt(value: unknown): value is RollbackReceipt {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const obj = value as Record<string, unknown>;
  return (
    isU32(obj["schema_version"]) &&
    obj["schema_version"] === 1 &&
    isNonEmptyString(obj["receipt_id"]) &&
    isObjectRef(obj["backup_ref"]) &&
    isVerificationState(obj["backup_verification"]) &&
    isRollbackState(obj["state"])
  );
}

export function isOfflineBundle(value: unknown): value is OfflineBundle {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const obj = value as Record<string, unknown>;
  return (
    isU32(obj["schema_version"]) &&
    obj["schema_version"] === 1 &&
    isNonEmptyString(obj["bundle_id"]) &&
    Array.isArray(obj["contents"]) &&
    obj["contents"].length > 0 &&
    isObjectRef(obj["manifest_ref"])
  );
}

export function isManualPromotion(value: unknown): value is ManualPromotion {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const obj = value as Record<string, unknown>;
  return (
    isU32(obj["schema_version"]) &&
    obj["schema_version"] === 1 &&
    isNonEmptyString(obj["promotion_id"]) &&
    isNonEmptyString(obj["approval_ref"]) &&
    isPromotionState(obj["state"]) &&
    isNonEmptyString(obj["exact_manual_command"])
  );
}

export function assertStringOrUndefined(
  value: unknown,
  what: string,
): string | undefined {
  if (value === undefined) {
    return undefined;
  }
  return assertString(value, what);
}
