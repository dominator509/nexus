-- EP-035 M3 onboarding durable state (PostgreSQL 18.4).
-- Canonical DDL for the @nexus/onboarding integration layer. The
-- contract semantics (state ladders, deny-unknown, secret redaction)
-- stay in @nexus/setup; these tables persist the real state across
-- process restarts and give the first-owner and one-time-token
-- semantics a real durability boundary.

CREATE TABLE IF NOT EXISTS onboarding_owner (
  owner_id          UUID PRIMARY KEY,
  idempotency_key   TEXT NOT NULL UNIQUE,
  owner_email       TEXT NOT NULL,
  state             TEXT NOT NULL,
  correlation_id    UUID NOT NULL,
  created_at_unix_s BIGINT NOT NULL,
  updated_at_unix_s BIGINT NOT NULL,
  CHECK (state IN (
    'OWNER_DETAILS_PROVIDED',
    'OWNER_IDENTITY_VERIFIED',
    'OWNER_PRINCIPAL_CREATED',
    'OWNER_AUTHORIZED'
  ))
);

-- Durable first-owner singleton: at most one row may exist. A second
-- competing bootstrap hits the unique partial index and maps to
-- CONFLICT; replay with the same idempotency_key maps to
-- ALREADY_INITIALIZED. This is the real persistence boundary (not an
-- in-process mutex).
CREATE UNIQUE INDEX IF NOT EXISTS onboarding_owner_singleton
  ON onboarding_owner ((owner_id IS NOT NULL))
  WHERE owner_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS onboarding_deployment_intent (
  intent_id          UUID PRIMARY KEY,
  mode               TEXT NOT NULL,
  release_channel    TEXT NOT NULL,
  profile_json       JSONB NOT NULL,
  verification_state TEXT NOT NULL,
  selected_at_unix_s BIGINT NOT NULL,
  verified_at_unix_s BIGINT,
  verification_evidence JSONB,
  correlation_id     UUID NOT NULL,
  CHECK (mode IN ('MANAGED','BYOC','EXISTING_SSH','HYBRID','FULLY_LOCAL')),
  CHECK (release_channel IN ('STABLE','BETA','DEVELOPER','PINNED')),
  CHECK (verification_state IN ('SELECTED','VERIFIED')),
  -- SELECTED intent may carry no evidence; VERIFIED requires it.
  CHECK (verification_state <> 'VERIFIED' OR verification_evidence IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS onboarding_enrollment_credential (
  credential_id     UUID PRIMARY KEY,
  kind              TEXT NOT NULL,
  state             TEXT NOT NULL,
  issued_at_unix_s  BIGINT NOT NULL,
  expires_at_unix_s BIGINT NOT NULL,
  used_at_unix_s    BIGINT,
  revoked_at_unix_s BIGINT,
  secret_hash       TEXT NOT NULL,
  nonce_hash        TEXT NOT NULL,
  correlation_id    UUID NOT NULL,
  CHECK (kind = 'BOOTSTRAP_TOKEN'),
  CHECK (state IN ('ISSUED','USED','REVOKED','EXPIRED')),
  CHECK (expires_at_unix_s > issued_at_unix_s),
  -- A consumed credential can never be used again.
  CHECK (state <> 'USED' OR used_at_unix_s IS NOT NULL),
  CHECK (state <> 'REVOKED' OR revoked_at_unix_s IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS onboarding_integration_state (
  integration_id     UUID PRIMARY KEY,
  provider_name      TEXT NOT NULL,
  status             TEXT NOT NULL,
  configured_at_unix_s BIGINT,
  authenticated_at_unix_s BIGINT,
  reachable_at_unix_s BIGINT,
  healthy_at_unix_s  BIGINT,
  capability_json    JSONB NOT NULL DEFAULT '[]'::jsonb,
  correlation_id     UUID NOT NULL,
  updated_at_unix_s  BIGINT NOT NULL,
  CHECK (status IN (
    'UNCONFIGURED','CONFIGURED','AUTHENTICATED',
    'REACHABLE','HEALTHY','DEGRADED','ERROR'
  )),
  -- CONFIGURED requires a configuration timestamp; REACHABLE/HEALTHY
  -- require the corresponding verification timestamp (credential-exists
  -- never implies HEALTHY).
  CHECK (status = 'UNCONFIGURED' OR configured_at_unix_s IS NOT NULL),
  CHECK (status <> 'REACHABLE' OR reachable_at_unix_s IS NOT NULL),
  CHECK (status <> 'HEALTHY' OR healthy_at_unix_s IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS onboarding_recovery_checkpoint (
  checkpoint_id      UUID PRIMARY KEY,
  mutation_id        TEXT NOT NULL UNIQUE,
  mutation_kind      TEXT NOT NULL,
  mutation_state     TEXT NOT NULL,
  failure_class      TEXT NOT NULL,
  outcome            TEXT NOT NULL,
  retry_safe         BOOLEAN NOT NULL,
  created_at_unix_s  BIGINT NOT NULL,
  reconciled_at_unix_s BIGINT,
  detail             TEXT NOT NULL,
  correlation_id     UUID NOT NULL,
  CHECK (mutation_state IN ('UNKNOWN','RECONCILED')),
  CHECK (failure_class IN (
    'AMBIGUOUS','UNAVAILABLE','TIMEOUT','VALIDATION',
    'AUTHORIZATION','CONFLICT','INTERNAL'
  )),
  CHECK (outcome IN (
    'RETRYABLE','NON_RETRYABLE','RESUME_CHECKPOINT','RECONCILE',
    'ROLLBACK','REAUTHENTICATE','RESET','MANUAL_INTERVENTION'
  )),
  -- No blind replay: an UNKNOWN mutation may never be retry-safe.
  CHECK (mutation_state <> 'UNKNOWN' OR retry_safe = FALSE)
);

CREATE TABLE IF NOT EXISTS onboarding_event_log (
  event_id       UUID PRIMARY KEY,
  event_kind     TEXT NOT NULL,
  correlation_id UUID NOT NULL,
  payload_json   JSONB NOT NULL,
  created_at_unix_s BIGINT NOT NULL
);
