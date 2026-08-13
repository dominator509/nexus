# Workflow Versioning Strategy

Status: Accepted (ADR-010, EP-006)
Scope: `@nexus/workflows` contracts + `infra/temporal` engine adapter
Binding requirement: SPEC-023 behavior 8 - "Workflow and event schema
upgrades preserve in-flight compatibility."

## 1. Contract surface versioning

The vocabulary, signals, queries, and activities are versioned semver
constants in `src/versioning.ts`:

- `WORKFLOW_CONTRACT_VERSION` - the whole workflow vocabulary surface.
- `SIGNAL_SCHEMA_VERSION` - payload schema for durable signals.
- `QUERY_SCHEMA_VERSION` - payload schema for read-only queries.
- `ACTIVITY_CONTRACT_VERSION` - activity contracts and idempotency keys.

Bump rules (enforced by review, checked by `ep006_unit_versioning_*` tests):

- **Minor / patch (additive):** add a new optional field, a new signal
  type, or a new query type. Existing payloads must still validate.
  Never change the meaning of an existing field.
- **Major (breaking):** remove a field, change its semantics, or change
  the idempotency-key derivation. A major bump requires a new workflow
  name (below) and a new ADR entry; the old contract remains served until
  every in-flight execution of the old major has drained or been migrated.

Compatibility checks: `isCompatibleSchemaVersion` (major-scoped) and
`isCompatibleSignalVersion` (peer major must equal current major and not
precede the workflow's declared minimum). A signal or query whose major
does not match is rejected fail-closed - never silently reinterpreted.

## 2. Runtime versioning (Temporal engine, infra/temporal)

When a workflow executes on Temporal, in-flight compatibility is
preserved with these mechanisms, all required by this strategy:

1. **Deterministic version markers.** Behavior changes inside a workflow
   body use Temporal's `patched()` / `version()` APIs keyed on the
   workflow name + contract version. Old histories replay through the old
   branch; new executions take the new branch. Without a version marker,
   a code change to a running workflow type is a determinism violation
   and will fail replay - that failure is treated as a bug, not a
   workaround.
2. **Workflow name versioning for breaking changes.** A breaking change
   ships as a new workflow name (e.g. `nexus.objective.v2`) registered on
   its own task queue. Old executions keep running on old workers until
   they finish; new starts use the new name. Signals and queries target a
   workflow by its `workflowId`; the engine routes them to the running
   execution regardless of name generation.
3. **Compatible sets.** When a new workflow name must answer queries or
   receive signals from an older generation during a drain window, the
   older generation is registered in the new generation's compatible set.
   Compatible-set membership is temporary and removed after the drain.
4. **Replay guarantee.** A recorded history replays identically against
   the same workflow name + version because workflow code is
   deterministic (see `src/determinism.ts` and the
   `ep006_unit_determinism_guard` test) and all I/O is behind activities
   with stable idempotency keys. Replaying an old history against a new
   code revision is only valid when the revision carries the same version
   markers the history recorded.
5. **Idempotency stability.** `signalId` (UUIDv7), activity idempotency
   keys, and the `actionDigest` binding never change meaning across a
   contract major. A re-delivered signal with the same `signalId` is a
   duplicate by definition; the engine and workflow deduplicate on it.
6. **Task queue isolation.** Each workflow generation registers its own
   task queue. Workers pin the generation they serve, so an old worker
   never pulls new-generation tasks and vice versa.

## 3. Signal and query evolution

- Signals are immutable once emitted (SPEC-023 behavior 7): principal,
  authentication strength, decision, and `decidedAt` are set by the
  signer boundary and never mutated by workflow code.
- Additive signal fields are allowed on a minor bump. Unknown _signal
  types_ are rejected by the vocabulary (`signalType.parse`), never
  ignored.
- A signal whose schema major is newer than the workflow's support is
  rejected with a typed `CONFLICT`/`VALIDATION` error and surfaced in the
  workflow status query, so an out-of-date worker cannot silently drop an
  approval.
- Queries are read-only and deterministic; their answers derive from the
  durable event history, so a replay answers identically.

## 4. What requires an ADR

- A new vocabulary name (workflow kind, signal type, query type, activity
  kind, error code, state, outcome).
- A major contract bump.
- A change to the idempotency-key derivation or the approval binding rule.
- A change to timeout or cancel semantics.

## 5. Test and live-fire coverage

- Unit: `ep006_unit_versioning_*` (constants parse, compatibility matrix
  consistent, fail-closed on major mismatch, doc markers present).
- Unit: `ep006_unit_determinism_guard` (no non-deterministic calls in
  workflow source).
- Integration (M3, real Temporal): replay of a recorded history; signal
  version mismatch rejection.
- Live-fire (M5, LF-017): worker restart + delayed approval resumes
  exactly once against a real Temporal server.
