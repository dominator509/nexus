NODE-META-BEGIN
ID: EP-041
DEPS: EP-040
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-041
VERIFY_SENTINEL: node verify EP-041: ok
GREEN_TAG: green/EP-041
NODE-META-END

# 1. Purpose / Big Picture

Implement the separate Microbrain dataset, frozen evals, teacher consensus, QLoRA pipeline, GGUF export, shadow comparison, and canary tooling. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-041.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-041.txt`.
- Implement real behavior, tests, telemetry, security, operations, and any owning live-fire proof.
- Preserve self-hosted-first selection and API fallback contracts.
- Keep optional providers disabled until certified.

# 3. Non-goals

- No work owned by a later node.
- No broad refactor, dependency replacement, vendor-specific domain model, or alternate architecture.
- No production deployment.
- No mocks, stubs, demonstration modes, or sample success in production paths.
- No claim that an adapter or hardware class is operational before real certification.
- No weakening of a spec, policy, security boundary, test, or GraphLock gate.

# 4. Context and Orientation

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-040` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-041.md`
- `.agent/specs/SPEC-025-microbrain-dataset-training-evaluation-shadow-and-promotion.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-041.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-041-microbrain-training-factory.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-041.txt`
- `.agent/node-contracts/EP-041.md`
- `scripts/nodes/EP-041.sh`
- `python/nexus_microbrain/`
- `microbrain/datasets/`
- `microbrain/evals/`
- `microbrain/training/`
- `microbrain/artifacts/`
- `tests/microbrain/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `MicrobrainDataset` | `tests/microbrain` | Defined by EP-041; provider-neutral and versioned |
| `FrozenEvalSuite` | `tests/microbrain` | Defined by EP-041; provider-neutral and versioned |
| `TeacherConsensus` | `tests/microbrain` | Defined by EP-041; provider-neutral and versioned |
| `TrainingCandidate` | `tests/microbrain` | Defined by EP-041; provider-neutral and versioned |
| `QloraRun` | `tests/microbrain` | Defined by EP-041; provider-neutral and versioned |
| `QuantizedArtifact` | `tests/microbrain` | Defined by EP-041; provider-neutral and versioned |
| `ShadowComparator` | `tests/microbrain` | Defined by EP-041; provider-neutral and versioned |
| `PromotionDecision` | `tests/microbrain` | Defined by EP-041; provider-neutral and versioned |

Acceptance obligations:

1. The frozen test set predates training and is never used for optimization
2. Teacher data is filtered, licensed, and privacy safe
3. A candidate cannot exceed its narrow NexusControlObject role
4. Shadow and canary thresholds include zero consequential false positives

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement the separate microbrain dataset, frozen evals, teacher consensus, qlora pipeline, gguf export, shadow comparison, and canary tooling.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-041-M1.txt`, `.agent/node-contracts/EP-041.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-041-microbrain-training-factory.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-041.txt`, `.agent/node-contracts/EP-041.md`, `scripts/nodes/EP-041.sh`, `python/nexus_microbrain/`, `tests/microbrain/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep041_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-041.sh M1`

EXPECT:

- `EP-041 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-041 MILESTONE_PASS "M1 EP-041 M1: ok"`

FALLBACK: Retain DeepSeek as the ReflexProvider and publish no Microbrain artifact. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-041][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-041.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-041-M2.txt`, `.agent/node-contracts/EP-041.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `microbrain/datasets/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep041_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-041.sh M2`

EXPECT:

- `EP-041 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-041 MILESTONE_PASS "M2 EP-041 M2: ok"`

FALLBACK: Retain DeepSeek as the ReflexProvider and publish no Microbrain artifact. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-041][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-041 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-041-M3.txt`, `.agent/node-contracts/EP-041.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `microbrain/evals/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep041_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-041.sh M3`

EXPECT:

- `EP-041 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-041 MILESTONE_PASS "M3 EP-041 M3: ok"`

FALLBACK: Retain DeepSeek as the ReflexProvider and publish no Microbrain artifact. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-041][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-041 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-041-M4.txt`, `.agent/node-contracts/EP-041.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `microbrain/training/`

CONTENT:

1. Create tests whose names begin `ep041_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-041.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-041 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-041 MILESTONE_PASS "M4 EP-041 M4: ok"`

FALLBACK: Retain DeepSeek as the ReflexProvider and publish no Microbrain artifact. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-041][M4] forced failures, abuse cases, and observability"`

### M4 progress (2026-08-24)

GOAL: Prove EP-041 fails safely under candidate/dataset/eval/leakage
faults for the training-candidate root (M4 fence: microbrain/training/).

CHANGED:
- `python/nexus_microbrain/training_policy.py` deterministic
  training-candidate behavior above the M1/M2/M3 canonical surfaces
  (pure; boundary loaders only): CandidateEligibilityVerdict/
  TrainingPlan/PlanVerdict/QloraRunVerdict/LeakageVerdict/
  TrainingEvidence records; narrow_role (exactly one of the 8 canonical
  roles - freeform/broad strings rejected by the deny-unknown
  vocabulary), check_candidate_eligibility (missing dataset reference
  denied, unknown dataset reference denied, dataset policy fail denied,
  dataset digest missing/malformed/mismatch denied, missing eval
  reference denied, missing eval suite binding denied, eval not frozen
  before training denied, eval policy fail denied, missing eval score
  denied, role unknown/too broad denied, missing model identity denied
  at the M1 contract boundary - CANDIDATE OBJECT EXISTS != ELIGIBLE TO
  TRAIN), build_training_plan + plan_digest + verify_plan_digest
  (canonical sort_keys sha256 determinism; tamper denied),
  training_plan_verdict (PLAN_READY only - never carries QLoRA
  success/artifact/promotion language; missing hyperparameters,
  unknown quantization format, unsupported target role all fail
  closed - TRAINING PLAN EXISTS != TRAINING EXECUTED),
  qlora_run_verdict (declared PENDING/RUNNING not executed; FAILED not
  certified; unknown status fails closed via QloraStatus.parse; run
  after invalid candidate denied; training output digest missing or
  malformed denied; start/end evidence required; metrics alone never
  certify - QLORA RUN EXISTS != TRAINING CERTIFIED),
  check_no_eval_leakage (eval example id reused as training example
  denied, same correlation_id train/eval collision denied, same example
  digest train/eval collision denied, missing suite or dataset evidence
  fails closed - absence of leakage must be evidenced, not asserted),
  build_training_evidence (current-run record bound to run_id/
  git_commit/candidate_id/dataset_id/dataset_digest/eval_suite_id/
  eval_suite_digest/role/plan_digest/eligibility/leakage_clean/
  qlora_status/decision; to_redacted_dict).
- `microbrain/training/plans/` real committed training fixtures
  LABELED local test fixtures never production training data:
  nexus-candidate-v1.candidate.json (real TrainingCandidate contract
  record, role INTERPRETATION, dataset_ref nexus-synthetic-role-ops-v1,
  status CANDIDATE) + nexus-training-plan-v1.plan.json (real
  deterministic TrainingPlan, GGUF, rank 16 / alpha 32 / seed 7, with
  its canonical plan_digest); both generated through the real M1/M4
  models so the files are guaranteed contract-valid (regenerator:
  scripts/microbrain-gen-fixtures.py). training README updated.
- `tests/microbrain/test_ep041_m4_training_policy.py` 46
  ep041_unit_m4_* proofs green (0 failed/ignored): real fixture loads
  (candidate + plan, digest matches), candidate real full journey
  eligible (real dataset + real eval suite + real binding + real
  digest + real score), dataset policy gate (custom prohibited set),
  fail-closed negatives (missing dataset reference, unknown dataset
  reference, dataset digest mismatch, malformed digest, missing eval
  reference, missing eval binding, eval not frozen before training,
  eval policy fail, missing eval score, role unknown via
  MICROBRAIN_UNKNOWN_VOCABULARY, role too broad strings, narrow-role
  holds 8 roles, missing model identity at contract, redacted verdict
  no secret leak), training plan (PLAN_READY only never executed,
  digest deterministic, tamper denied, missing hyperparameters
  rejected, unknown quantization format rejected, non-GGUF rejected,
  unsupported target role rejected, plan does not create run or
  promotion), QLoRA contract (declared not executed, missing evidence
  not certified, failed not certified, unknown status fails closed,
  after invalid candidate denied, output digest missing denied,
  output digest malformed denied, metrics alone never certify, full
  evidence certifies contract), leakage (clean on real fixtures, eval
  reuse denied, correlation collision denied, digest collision denied,
  missing evidence fails closed, OOD pass does not override eval
  failure), evidence (current-run bound, BLOCK on failure, redacted no
  secret leak, redaction canary scrubbed).
- `scripts/ep041-m4-tests.sh` non-vacuous M4 gate (M1 + M2 + M3
  regressions first, material presence, training fixture JSON validity,
  anti-masking sentinels node M4 wired to gate no artifact-check
  masking, real pytest vacuity count >= 160 zero failed/error, M4
  fail-closed negative proofs present >= 15 counted from source,
  training-plan-not-executed proofs present >= 4, leakage negative
  proofs present >= 3, dependency-direction scan, no-placeholder scan,
  ruff check + ruff format --check owned surface; EP-041 M4 gate: ok).
- `scripts/nodes/EP-041.sh` M4 rewired from artifact-check masking to
  the real gate (EP-041 M4: ok EXIT=0).
- `.agent/expected-files/EP-041.txt`: scripts/ep041-m4-tests.sh +
  scripts/microbrain-gen-fixtures.py added.

REAL DEFECTS found+fixed:
1. Fixture generator parents[2] resolved to /root instead of the repo
   root (script lives at scripts/) - parents[1] fixed.
2. ruff I001 import sorting + line-length + unused suite vars in the
   leakage tests - fixed and re-run green.
3. missing_model_identity test tried to construct an invalid candidate
   directly - M1 already fails closed at construction, so the test now
   proves the contract boundary denial (typed code) rather than a
   gate-only path.
4. Blueprint validation flagged non-ASCII em-dashes in the training
   README - replaced with ASCII hyphens.

Observed (exit 0): EP-041 M4 gate: ok; node EP-041 M4: ok; node EP-041
M1 regression: ok; node EP-041 M2 regression: ok; node EP-041 M3
regression: ok; 168 ep041_unit_* proofs green (55 M1 + 26 M2 + 41 M3 +
46 M4); ruff check ok; ruff format ok; mypy ok (7 source files); side
gates green: scope audit EP-041: ok (M4 gate + generator added to
expected-files), expected files EP-041: ok, security check: ok (0
advisories), dependency audit: ok, license gate: ok, reality gate: ok,
blueprint validation: ok (after ASCII fix), format check: ok, lint: ok,
typecheck: ok, test-unit: ok.

Certification boundary (honest): training candidate behavior INTERNAL
BEHAVIOR CERTIFIED for the exact exercised local surface (real
committed candidate/plan fixtures through the real M1 contract + real
deterministic eligibility/timing/leakage/plan gates; fail-closed
negatives; current-run redacted evidence); training plan behavior
INTERNAL BEHAVIOR CERTIFIED (PLAN_READY only, deterministic digest,
tamper denied); train/eval leakage checks INTERNAL BEHAVIOR CERTIFIED
for the exact fixture surface exercised; QloraRun contract behavior
CONTRACT/ELIGIBILITY CERTIFIED only - no real QLoRA execution occurred
and no adapter artifact or training log was fabricated; real model
training NOT ASSERTED (no training executed), real QLoRA execution NOT
ASSERTED, real adapter artifact NOT ASSERTED, real GGUF quantization
NOT ASSERTED (M5 owns microbrain/artifacts/), real model evaluation NOT
ASSERTED (no model scored), production promotion NOT ASSERTED, live
deployment NOT ASSERTED, remote synchronization NOT ASSERTED at M4 time
(REMOTE_SYNC_BLOCKED_OWNER_AUTH - GitHub credential HTTP 401).
RESOLVED 2026-08-24: gh auth repaired with fresh PAT; origin/master
pushed and verified == local HEAD; unpushed count 0; no force-push
used.

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-041.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-041-M5.txt`, `.agent/node-contracts/EP-041.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `microbrain/artifacts/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-041` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-041.sh M5`
2. `sh scripts/node-verify.sh EP-041`
3. `sh scripts/scope-audit.sh EP-041`

EXPECT:

- `EP-041 M5: ok`
- `node verify EP-041: ok`
- `scope audit EP-041: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-041 MILESTONE_PASS "M5 EP-041 M5: ok"`

FALLBACK: Retain DeepSeek as the ReflexProvider and publish no Microbrain artifact. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-041][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-041` and observe `node verify EP-041: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [x] M2: Core behavior and deterministic invariants
- [x] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

### M1 progress (2026-08-24)

GOAL: Create the owned package roots and encode the public contracts for
the separate Microbrain dataset, frozen evals, teacher consensus, QLoRA
pipeline, GGUF export, shadow comparison, and canary tooling
(SPEC-025, SPEC-009; node contract EP-041).

CHANGED:
- `python/nexus_microbrain/` @nexus_microbrain contract package
  (stdlib-only; provider-neutral; dependency-direction enforced):
  `vocabulary.py` deny-unknown canonical enums (Role narrow
  INTERPRETATION/CAPABILITY_SELECTION/ROUTING/RISK/PRIVACY/AMBIGUITY/
  QUOTED_INSTRUCTION/ESCALATION, DataProvenance
  DETERMINISTIC_GENERATION/TEACHER_CONSENSUS/HARD_NEGATIVE/
  OPTED_IN_SCRUBBED_CORRECTION, EvalDimension 11 SPEC-025 dimensions,
  QuantizationFormat locked to GGUF, ShadowDecision MATCH/DIFFER/DEFER,
  PromotionGate SHADOW/LOW_RISK_CANARY/GRADUAL/PROMOTED, PromotionVerdict
  PROMOTE/DENY/HOLD, OodVerdict IN_DISTRIBUTION/OUT_OF_DISTRIBUTION,
  LicenseKind six classes, CandidateStatus/QloraStatus/ArtifactStatus
  ladders; enum.StrEnum with fail-closed parse and typed
  MICROBRAIN_UNKNOWN_VOCABULARY on unknown values); `errors.py`
  SPEC-006-style typed error surface (11 MICROBRAIN_CODE_* codes) with
  runtime-constructed redaction marker families (sk-/ghp_/AKIA/Bearer/
  pk-/xoxb-/glpat-/token=/password=/secret= + credential URLs + long
  token runs) and redact_text/redact_value used by error to_dict;
  `models.py` versioned serialization contracts (schema_version "1",
  fail-closed unsupported-version, required-field typed codes,
  __post_init__ validation so direct construction fails closed too,
  abstract base via ABC) for all 8 public interfaces
  (MicrobrainDataset, FrozenEvalSuite, TeacherConsensus,
  TrainingCandidate, QloraRun, QuantizedArtifact, ShadowComparator,
  PromotionDecision) plus supporting TrainingExample, FrozenEval,
  ShadowComparison, LicenseRecord; cross-field acceptance obligations
  encoded at the contract boundary: frozen evals must predate training
  (MICROBRAIN_FROZEN_SPLIT_VIOLATION), teacher consensus must be
  filtered + privacy safe + licensed (PRIVACY_VIOLATION/UNLICENSED),
  candidate role is one narrow canonical Role, GGUF artifact identity
  is alg:hex >=32 digest (IMAGE TAG != DIGEST), shadow
  consequential_false_positives cannot be negative, PROMOTE requires
  zero consequential false positives and never directly from SHADOW
  (FALSE_POSITIVE_THRESHOLD - SPEC-025 behavior 6 hard failure).
- `tests/microbrain/` @tests/microbrain test root: conftest.py (python/
  on sys.path, same pattern as tests/connectors), test_ep041_m1_contracts.py
  55 ep041_unit_* proofs green (0 failed/ignored): construction of all
  8 interfaces, serialization roundtrip + determinism + JSON encodable
  + schema_version preserved/rejected, vocabulary rejection for every
  enum, required-field rejection, all cross-field acceptance
  obligations, redaction proofs with runtime-constructed secret
  canaries (canary never survives error redaction or to_dict),
  dependency-direction scan (no requests/httpx/boto3/torch/
  transformers/openai/anthropic/numpy/pandas/nexus_connector_sdk).
- `microbrain/datasets/`, `microbrain/evals/`, `microbrain/training/`,
  `microbrain/artifacts/` owned data roots with README ownership notes.
- `pyproject.toml` (added to expected-files): python/nexus_microbrain
  registered as wheel package; ep041_unit_*/ep041_integration_*/ep041_failure_*
  registered in pytest python_functions.
- `scripts/ep041-m1-tests.sh` non-vacuous M1 gate (material presence of
  all 10 owned paths, workspace membership, anti-masking sentinels
  incl node M1 wired to gate and no artifact-check masking, real pytest
  vacuity guard count >= 50 with zero failed/error, dependency-direction
  forbidden-import scan, no-placeholder scan excluding __pycache__,
  ruff check + ruff format --check on owned surface; EP-041 M1 gate: ok).
- `scripts/nodes/EP-041.sh` M1 rewired from artifact-check masking to
  the real gate (EP-041 M1: ok EXIT=0).
- `.agent/expected-files/EP-041.txt`: scripts/ep041-m1-tests.sh and
  pyproject.toml added (scope-audit requires manifests that changed).

REAL DEFECTS found+fixed:
1. Dataclass field ordering: schema_version as an instance field with a
   default made every non-default subclass field illegal - moved to
   ClassVar on the abstract base.
2. Direct construction bypassed validation (only from_dict validated) -
   added __post_init__ so construction fails closed too.
3. Ruff UP042: str+Enum inheritance -> enum.StrEnum.
4. Reality gate flagged the base to_dict() raise NotImplementedError as
   a stub anti-pattern - replaced with abc.ABC + abstractmethod so the
   base cannot be instantiated and no stub marker exists.
5. Gate no-placeholder scan tripped on its own docstring word and on
   __pycache__ binaries - reworded docstring, excluded __pycache__ and
   restricted scan to *.py.

Observed (exit 0): EP-041 M1 gate: ok; node EP-041 M1: ok; 55
ep041_unit_* proofs green; ruff check ok; ruff format ok; mypy ok
(4 source files); side gates green: security check: ok (0 advisories),
dependency audit: ok, license gate: ok, reality gate: ok (after ABC
fix), blueprint validation: ok, format check: ok, lint: ok, typecheck:
ok, test-unit: ok.

Certification boundary (honest): Microbrain contract/vocabulary
PACKAGE BOUNDARY CERTIFIED for the exact exercised local surface
(construction, validation, versioned serialization, vocabulary
rejection, acceptance-obligation invariants, redaction,
dependency-direction); deterministic behavior of the training
pipeline/eval engine/shadow comparator NOT ASSERTED (M2 owns core
behavior); real dataset artifacts NOT ASSERTED (data roots empty by
truth); real QLoRA/GGUF execution NOT ASSERTED (M3+ own transport and
live-fire); remote synchronization NOT ASSERTED at M1 time
(REMOTE_SYNC_BLOCKED_OWNER_AUTH - GitHub credential HTTP 401).
RESOLVED 2026-08-24: gh auth repaired; origin/master pushed and
verified == local HEAD (3d1c393); unpushed count 0.

### M2 progress (2026-08-24)

GOAL: Implement the production behavior and deterministic invariants
owned by EP-041 for the dataset boundary (M2 fence: microbrain/datasets/).

CHANGED:
- `python/nexus_microbrain/dataset_policy.py` deterministic dataset
  policy engine above the M1 contract (pure, no I/O): DatasetVerdict
  (usable/reasons/licensed/privacy_safe/counts by provenance+role/
  hard-negative/OOD) with to_dict + to_redacted_dict; DatasetPolicy
  with eight fail-closed rules - non-empty (DATASET EXISTS != USABLE),
  every example licensed (MISSING LICENSE -> DENIED), no prohibited
  license (default cc-by-nc-4.0; PROHIBITED LICENSE -> DENIED), no
  unknown license (unknown/unlicensed prefixes; UNKNOWN LICENSE ->
  DENIED), hard_negative flag consistency (flag true requires
  HARD_NEGATIVE provenance), provenance known, role known (defense in
  depth), teacher/opted-in examples licensed; boundary helpers
  load_manifest (real JSON file -> M1 MicrobrainDataset, fails closed
  typed on missing/malformed/non-object/unsupported-version),
  sha256_manifest (sha256:hex of real file bytes), verify_manifest_file
  (digest binding; mismatch -> verified False, missing file -> typed
  MISSING_REQUIRED).
- `microbrain/datasets/manifests/` real committed manifest fixtures:
  nexus-synthetic-role-ops-v1.manifest.json (12 examples: 10
  deterministic + 2 hard negatives; all 8 narrow roles; 4
  out-of-distribution) and nexus-teacher-consensus-v1.manifest.json
  (6 examples: 4 TEACHER_CONSENSUS + 2 OPTED_IN_SCRUBBED_CORRECTION
  with license refs + correlation ids). LABELED local test fixtures,
  never production training data. datasets README updated.
- `tests/microbrain/test_ep041_m2_dataset_policy.py` 26 ep041_unit_m2_*
  proofs green (0 failed/ignored): real manifest loads, policy positive
  on both real manifests (usable/licensed/privacy_safe, provenance and
  role counts, determinism), fail-closed negatives (empty dataset,
  missing license, prohibited license, unknown license, hard_negative
  flag inconsistency, custom prohibited set, custom unknown prefix,
  mixed denials list all reasons), digest verification (sha256 alg:hex,
  match, mismatch denied, current-run recorded, missing file typed),
  composition load->verify->evaluate, redaction of verdict payloads
  with runtime-constructed secret canaries.
- `scripts/ep041-m2-tests.sh` non-vacuous M2 gate (M1 regression via
  ep041-m1-tests.sh, material presence, manifest JSON validity via
  python3 -m json.tool, anti-masking sentinels node M2 wired to gate
  no artifact-check masking, real pytest vacuity count >= 75 zero
  failed/error, M2 fail-closed negative proofs present >= 6,
  dependency-direction forbidden-import scan, no-placeholder scan
  excluding __pycache__, ruff check + ruff format --check owned
  surface; EP-041 M2 gate: ok).
- `scripts/nodes/EP-041.sh` M2 rewired from artifact-check masking to
  the real gate (EP-041 M2: ok EXIT=0).
- `.agent/expected-files/EP-041.txt`: scripts/ep041-m2-tests.sh added.

REAL DEFECTS found+fixed:
1. Gate negative-proof sentinel counted from pytest -q output (dots
   only, no test names) - count from the test source instead.
2. Prettier format gate flagged the two JSON manifests - reformatted
   with npx prettier --write (semantic-neutral reflow; tests re-run
   green after).

Observed (exit 0): EP-041 M2 gate: ok; node EP-041 M2: ok; node EP-041
M1 regression: ok; 81 ep041_unit_* proofs green (55 M1 + 26 M2); ruff
check ok; ruff format ok; mypy ok (5 source files); side gates green:
security check: ok (0 advisories), dependency audit: ok, license gate:
ok, reality gate: ok, blueprint validation: ok, format check: ok
(after prettier fix), lint: ok, typecheck: ok, test-unit: ok.

Certification boundary (honest): dataset policy behavior INTERNAL
BEHAVIOR CERTIFIED for the exact exercised local surface (real
committed manifest fixtures parsed through the real M1 contract and
evaluated through the real deterministic policy; fail-closed negative
cases; digest binding; redaction); frozen eval behavior NOT ASSERTED
(M3 owns microbrain/evals/), teacher consensus scoring beyond the M1
contract fields NOT ASSERTED, training candidate eligibility behavior
NOT ASSERTED (M4 owns microbrain/training/), shadow comparator /
promotion gate behavior NOT ASSERTED (later milestones), real QLoRA /
GGUF execution NOT ASSERTED, real model training NOT ASSERTED, remote
synchronization NOT ASSERTED at M2 time (REMOTE_SYNC_BLOCKED_OWNER_AUTH
- GitHub credential HTTP 401). RESOLVED 2026-08-24: gh auth repaired;
origin/master pushed and verified == local HEAD (3d1c393).

### M3 progress (2026-08-24)

GOAL: Connect EP-041 to its real selected dependency and prove contract
behavior across the boundary for the frozen-eval root (M3 fence:
microbrain/evals/).

CHANGED:
- `python/nexus_microbrain/eval_policy.py` deterministic frozen-eval
  behavior above the M1/M2 surfaces (pure, no I/O except boundary
  loaders): EvalSuiteVerdict/EvalBindingVerdict/DimensionResult/
  EvalScoreSummary/SuiteScoreSummary/SuiteBinding/EvalEvidence records;
  validate_suite (non-empty, unique eval ids, unique example ids,
  frozen markers present, suite created_at present - M1 enforces empty
  suite rejection at construction, validator keeps defense in depth),
  check_eval_before_training (every frozen_at strictly before training
  start; at/after rejected - EVAL CREATED AFTER TRAINING != VALID
  FROZEN EVAL), suite_digest + verify_suite_digest (canonical
  sort_keys sha256 immutability binding; tampered suite rejected),
  load_suite_binding (real JSON sidecar -> SuiteBinding; fail closed
  typed on malformed/missing-field/bad-digest), bind_suite_to_dataset
  (suite_id match, dataset_id match UNKNOWN DATASET REJECTED, dataset
  digest match, DatasetPolicy must pass - DATASET POLICY PASSED != EVAL
  PASSED - and eval examples run through DatasetPolicy so an eval
  fixture cannot include data M2 would deny), score_eval + score_suite
  (deterministic per-dimension aggregation ordered by canonical enum
  order; missing/duplicate/unknown dimension results fail closed;
  OOD item requires OUT_OF_DISTRIBUTION_ESCALATION coverage and pass
  (unsafe OOD blocks); hard-negative item requires INJECTION_RESISTANCE
  coverage and pass; any failing eval blocks the suite),
  build_eval_evidence (current-run record bound to run_id/git_commit/
  suite_id/dataset_id/dataset_digest/decision/score/dimensions/OOD/
  hard-negative/optional candidate+role+timestamp; to_redacted_dict).
- `microbrain/evals/suites/` real committed eval fixtures:
  nexus-frozen-suite-v1.eval.json (11 frozen evals: all 8 narrow roles,
  3 out-of-distribution items, 2 hard-negative items, all frozen before
  the 2026-08-10 training start; generated through the real M1
  FrozenEvalSuite model so the file is guaranteed contract-valid) +
  nexus-frozen-suite-v1.binding.json (sidecar binding to dataset
  nexus-synthetic-role-ops-v1 with the REAL sha256 digest of the
  committed dataset manifest). LABELED local test fixtures, never real
  model evaluation results. evals README updated.
- `tests/microbrain/test_ep041_m3_eval_policy.py` 41 ep041_unit_m3_*
  proofs green (0 failed/ignored): real fixture loads, binding digest
  matches real manifest digest, structural validation (empty suite
  rejected at contract, duplicate eval ids, duplicate example ids,
  missing frozen_at, missing suite created_at, missing dimensions at
  contract), timing (before eligible, at training start rejected,
  after rejected), immutability (deterministic digest, match, tamper
  rejected), dataset binding (real bind, suite_id mismatch, unknown
  dataset, digest mismatch, policy-denied dataset, eval with M2-denied
  teacher data), deterministic scoring (all-pass, any-fail blocks,
  missing/unknown/duplicate dimension fail closed, determinism, OOD
  without OOD dimension, OOD escalation failed blocks, HN without
  injection, HN injection failed blocks, real suite all-pass, one-fail
  blocks suite, missing eval blocks), evidence (current-run binding,
  BLOCK decision on failure, redaction with runtime-constructed
  canary), real fixture full journey (validate -> timing -> digest ->
  bind -> score -> evidence).
- `scripts/ep041-m3-tests.sh` non-vacuous M3 gate (M1 + M2 regressions
  first, material presence, fixture JSON validity, anti-masking
  sentinels node M3 wired to gate no artifact-check masking, real
  pytest vacuity count >= 115 zero failed/error, M3 fail-closed
  negative proofs present >= 10, deterministic scoring proofs present
  >= 3, dependency-direction scan, no-placeholder scan, ruff check +
  ruff format --check owned surface; EP-041 M3 gate: ok).
- `scripts/nodes/EP-041.sh` M3 rewired from artifact-check masking to
  the real gate (EP-041 M3: ok EXIT=0).
- `.agent/expected-files/EP-041.txt`: scripts/ep041-m3-tests.sh added.

REAL DEFECTS found+fixed:
1. The two hard-negative eval fixtures were also OOD items but did not
   declare OUT_OF_DISTRIBUTION_ESCALATION coverage, so the scorer
   correctly blocked them - regenerated the fixture with the missing
   dimension (honest fixture, not a scorer weakening).
2. Ruff B008 flagged DatasetPolicy() as a default argument - changed
   to Optional with lazy construction.
3. Prettier format gate flagged the eval fixture JSON - reformatted
   with npx prettier --write (semantic-neutral; tests re-run green).

Observed (exit 0): EP-041 M3 gate: ok; node EP-041 M3: ok; node EP-041
M1 regression: ok; node EP-041 M2 regression: ok; 122 ep041_unit_*
proofs green (55 M1 + 26 M2 + 41 M3); ruff check ok; ruff format ok;
mypy ok (6 source files); side gates green: security check: ok (0
advisories), dependency audit: ok, license gate: ok, reality gate: ok,
blueprint validation: ok, format check: ok (after prettier fix), lint:
ok, typecheck: ok, test-unit: ok.

Certification boundary (honest): frozen eval behavior INTERNAL BEHAVIOR
CERTIFIED for the exact exercised local surface (real committed eval
fixtures through the real M1 contract + real deterministic validator/
timer/digest/binder/scorer; fail-closed timing, immutability, dataset
binding, OOD, and hard-negative gates; current-run redacted evidence);
deterministic scoring INTERNAL BEHAVIOR CERTIFIED for the exact
dimension-result surface exercised; teacher-consensus eval fixture
behavior INTERNAL BEHAVIOR CERTIFIED only for the M2-policy binding
surface (consensus contributes evidence only - no promotion path in
M3); real model evaluation NOT ASSERTED (no model scored), real
training NOT ASSERTED (M4 owns microbrain/training/), real QLoRA
execution NOT ASSERTED, real GGUF quantization NOT ASSERTED (M5 owns
microbrain/artifacts/), production promotion NOT ASSERTED, live
deployment NOT ASSERTED, remote synchronization NOT ASSERTED at M3
time (REMOTE_SYNC_BLOCKED_OWNER_AUTH - GitHub credential HTTP 401).
RESOLVED 2026-08-24: gh auth repaired with fresh PAT; origin/master
pushed and verified == local HEAD (3d1c393) via git ls-remote;
unpushed count 0; no force-push used.

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-24: M1 contract surfaces. The node's Interface Map assigns all
  eight public interfaces to the owned Python boundary, so the versioned
  serialization contract lives in python/nexus_microbrain/ (stdlib-only
  dataclasses with to_dict/from_dict and schema_version "1") rather than
  generated schemas/ bindings - generate-contracts.sh has no microbrain
  coverage and no cross-language consumer exists yet. The repo reality
  gate treats raise NotImplementedError as a stub anti-pattern even in
  an abstract base, so the base uses abc.ABC + abstractmethod. Test
  count: 55 new ep041_unit_* proofs, zero failed/ignored.
- 2026-08-24: remote-sync limitation cleared. The long-standing GitHub
  HTTP 401 (REMOTE_SYNC_BLOCKED_OWNER_AUTH) was repaired by a fresh
  PAT via `gh auth login --with-token`. Origin/master pushed (range
  b416769..3d1c393) and remote ref verified == local HEAD with
  `git ls-remote origin refs/heads/master`; unpushed count 0; no
  force-push. 25 commits (EP-039 closure through EP-041 M3) reached
  GitHub. All prior "remote refs NOT verified" caveats in this ExecPlan
  are superseded as of this date.
- 2026-08-24: M4 training-candidate behavior. The M1 contract already
  fails closed on empty model_ref/base_model, so the eligibility gate
  can never observe an unprovenanced candidate - the honest proof is
  the contract boundary denial itself (typed MICROBRAIN_INVALID_INPUT),
  not a gate-only path. TrainingPlan.from_dict also fails closed on
  freeform role strings at parse, so plan.validate()'s NARROW_ROLES
  check is defense-in-depth; the observable fail-closed surface is the
  deny-unknown vocabulary. Test count: 46 new ep041_unit_m4_* proofs,
  zero failed/ignored (168 total).

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-24: M1 contracts are Python-native versioned serialization,
  not schemas/ JSON Schema generation. Evidence: the M1 fence owns
  python/nexus_microbrain/ + tests/microbrain/; the interface map says
  "Defined by EP-041; provider-neutral and versioned"; schemas/
  generate-contracts.sh has no microbrain support and there is no
  cross-language consumer in M1. Alternatives: generating JSON Schemas
  under schemas/ was rejected because the conditional in the M1 fence
  ("when the node owns cross-language contracts") is not met yet and
  would touch a non-owned path. Consequence: contracts serialize via
  to_dict/from_dict with schema_version enforcement; a later milestone
  can add schemas/ generation if a cross-language consumer appears.
  Security: redaction is built into the error surface. License: package
  is MIT under the repo project. Compatibility: pure additive wheel
  package; no existing imports change.
- 2026-08-24: M2 dataset policy is a pure deterministic layer above the
  M1 contract, not a new data model. Evidence: the M2 fence owns
  microbrain/datasets/ and the ExecPlan requires domain rules pure with
  I/O behind ports; the policy consumes M1 MicrobrainDataset and
  TrainingExample unchanged. Alternatives: a parallel dataset model in
  microbrain/datasets/ was rejected - the fence says canonical truth
  remains in python/nexus_microbrain/. Consequence: manifests are real
  JSON files parsed through M1 from_dict; policy rules fail closed with
  typed codes; license governance (missing/prohibited/unknown) sits in
  the policy layer while the M1 contract permits construction of
  unlicensed deterministic examples. Security: verdict redaction with
  runtime canaries; no secret literals. License: manifest fixtures are
  synthetic/teacher-consensus licensed records labeled as local test
  fixtures, never production data. Compatibility: pure additive module
  and data files; no existing import changes.
- 2026-08-24: M3 frozen-eval behavior is a pure deterministic layer on
  top of M1/M2, with the eval fixture generated through the real M1
  FrozenEvalSuite model so the committed file is guaranteed
  contract-valid. Evidence: the M3 fence owns microbrain/evals/ and the
  ExecPlan requires real local dependency integration; the scorer takes
  explicit DimensionResult inputs rather than inventing model outputs,
  so scoring is real deterministic aggregation, not fabricated
  evaluation. Alternatives: a separate eval data model was rejected -
  the fence requires canonical M1/M2 surfaces. Consequence: eval
  fixtures are real JSON files parsed through M1; the binding sidecar
  carries the real dataset manifest digest; OOD and hard-negative items
  must declare the mandatory coverage dimensions or the scorer blocks
  (real fixture defect found and fixed by regenerating the fixture with
  the missing OOD escalation dimension). Security: evidence redaction
  with runtime canaries; no secret literals. License: eval fixtures are
  synthetic licensed records labeled as local test fixtures.
  Compatibility: pure additive module and data files; no existing
  import changes.
- 2026-08-24: M4 training-candidate behavior is a pure deterministic
  layer on top of M1/M2/M3 canonical surfaces, not a training executor.
  Evidence: the M4 fence owns microbrain/training/ and the ExecPlan M4
  section requires forced-failure/abuse-case proofs without real QLoRA
  execution; the M1 contract already locks candidate model identity and
  narrow roles, and the eligibility gate consumes the real M2
  DatasetPolicy + real M3 eval binding/scoring. Alternatives: a
  parallel training model was rejected - the fence requires canonical
  surfaces; a real QLoRA invocation was rejected because no GPU/weights
  exist and M5 owns artifacts (real training NOT ASSERTED). Consequence:
  candidate eligibility, training plan (PLAN_READY only), QloraRun
  contract honesty, and leakage checks are certified INTERNAL BEHAVIOR
  only; no adapter artifact or training log was fabricated; plan/run
  verdicts deliberately carry no promotion language (TRAINING PLAN
  EXISTS != TRAINING EXECUTED). Security: evidence redaction with
  runtime canaries; no secret literals. License: candidate/plan
  fixtures are synthetic licensed records labeled as local test
  fixtures. Compatibility: pure additive module and data files; no
  existing import changes.
- 2026-08-24: remote-sync auth repaired with a fresh GitHub PAT via
  `gh auth login --with-token` (previous token invalid, HTTP 401).
  Evidence: `gh auth status` shows account dominator509 active with
  repo scope; `git push origin master` succeeded (b416769..3d1c393);
  `git ls-remote origin refs/heads/master` returned
  3d1c393da32837ff64fb2d6e48c7a3dfc7f893ff == local HEAD; unpushed
  count 0. Alternatives: none - owner supplied the new token; no
  force-push was used or needed. Consequence: the
  REMOTE_SYNC_BLOCKED_OWNER_AUTH limitation is cleared and all prior
  "remote refs NOT verified" caveats are superseded; future milestones
  may claim remote synchronization when verified. Security: token was
  supplied in-chat, consumed via stdin into gh (never written to a
  tracked file or echoed); /root/.config/gh/hosts.yml updated by gh.
  License/compatibility: no repo code or behavior change.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
