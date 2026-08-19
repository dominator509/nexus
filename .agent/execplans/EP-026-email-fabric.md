NODE-META-BEGIN
ID: EP-026
DEPS: EP-025
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-026
VERIFY_SENTINEL: node verify EP-026: ok
GREEN_TAG: green/EP-026
NODE-META-END

# 1. Purpose / Big Picture

Implement universal mailboxes, Gmail, Microsoft Graph, IMAP and SMTP, self-hosted mail option, attachments, drafts, sends, and audit. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-026.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-026.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-025` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-026.md`
- `.agent/specs/SPEC-014-email-phone-fax-notifications-and-communications-routing.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-026.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-026-email-fabric.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-026.txt`
- `.agent/node-contracts/EP-026.md`
- `scripts/nodes/EP-026.sh`
- `crates/nexus-email/`
- `connectors/gmail/`
- `connectors/microsoft-mail/`
- `connectors/imap-smtp/`
- `infra/mail/`
- `tests/email/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `EmailProvider` | `nexus-email` | Defined by EP-026; provider-neutral and versioned |
| `Mailbox` | `nexus-email` | Defined by EP-026; provider-neutral and versioned |
| `Thread` | `nexus-email` | Defined by EP-026; provider-neutral and versioned |
| `Message` | `nexus-email` | Defined by EP-026; provider-neutral and versioned |
| `Attachment` | `nexus-email` | Defined by EP-026; provider-neutral and versioned |
| `Draft` | `nexus-email` | Defined by EP-026; provider-neutral and versioned |
| `SendRequest` | `nexus-email` | Defined by EP-026; provider-neutral and versioned |
| `MailChangeFeed` | `nexus-email` | Defined by EP-026; provider-neutral and versioned |
| `MailPolicy` | `nexus-email` | Defined by EP-026; provider-neutral and versioned |

Acceptance obligations:

1. Gmail, Microsoft Graph, and generic IMAP or SMTP map to canonical objects
2. Read and send scopes are separate
3. Attachments use ArtifactStore and malware scanning
4. Draft, approval, send, reply, forward, archive, and label actions audit correctly

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement universal mailboxes, gmail, microsoft graph, imap and smtp, self-hosted mail option, attachments, drafts, sends, and audit.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-026-M1.txt`, `.agent/node-contracts/EP-026.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-026-email-fabric.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-026.txt`, `.agent/node-contracts/EP-026.md`, `scripts/nodes/EP-026.sh`, `crates/nexus-email/`, `tests/email/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep026_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-026.sh M1`

EXPECT:

- `EP-026 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-026 MILESTONE_PASS "M1 EP-026 M1: ok"`

FALLBACK: Use generic IMAP and SMTP plus controlled polling when provider webhooks are unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-026][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-026.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-026-M2.txt`, `.agent/node-contracts/EP-026.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/gmail/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep026_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-026.sh M2`

EXPECT:

- `EP-026 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-026 MILESTONE_PASS "M2 EP-026 M2: ok"`

FALLBACK: Use generic IMAP and SMTP plus controlled polling when provider webhooks are unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-026][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-026 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-026-M3.txt`, `.agent/node-contracts/EP-026.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/microsoft-mail/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep026_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-026.sh M3`

EXPECT:

- `EP-026 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-026 MILESTONE_PASS "M3 EP-026 M3: ok"`

FALLBACK: Use generic IMAP and SMTP plus controlled polling when provider webhooks are unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-026][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-026 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-026-M4.txt`, `.agent/node-contracts/EP-026.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/imap-smtp/`

CONTENT:

1. Create tests whose names begin `ep026_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-026.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-026 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-026 MILESTONE_PASS "M4 EP-026 M4: ok"`

FALLBACK: Use generic IMAP and SMTP plus controlled polling when provider webhooks are unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-026][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-026.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-026-M5.txt`, `.agent/node-contracts/EP-026.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/mail/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-026` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-026.sh M5`
2. `sh scripts/node-verify.sh EP-026`
3. `sh scripts/scope-audit.sh EP-026`

EXPECT:

- `EP-026 M5: ok`
- `node verify EP-026: ok`
- `scope audit EP-026: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-026 MILESTONE_PASS "M5 EP-026 M5: ok"`

FALLBACK: Use generic IMAP and SMTP plus controlled polling when provider webhooks are unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-026][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-026` and observe `node verify EP-026: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-011` `email-lifecycle`: Receive, search, summarize, draft, approve, send, and verify a real message through a certified mail provider.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [ ] M2: Core behavior and deterministic invariants
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

## M1 detail

- Created `crates/nexus-email` (nexus-email) contract crate: SPEC-014 vocabulary
  locked (Mailbox, Thread, Message, Draft, DeliveryReceipt, DisclosurePolicy
  boundary), typed ids (MailboxId/ThreadId/MessageId/AttachmentId/DraftId/
  DeliveryReceiptId), EmailAddress validation, MailScope with SEPARATE
  Read/Send/Draft/Reply/Forward/Archive/Label/Attachments scopes (acceptance
  obligation 2), MailCommand with required_scope mapping, MailState ladder
  DRAFT<QUEUED<SENDING<SENT<DELIVERED + terminal Failed/Archived/Deleted
  (SENT != DELIVERED: DeliveryReceipt is the only delivery authority),
  MailPrivacyClass, Attachment with sha256 digest + ScanStatus gate
  (only CLEAN deliverable - acceptance obligation 3), Message/Draft/SendRequest
  (idempotency key + has_send_scope), DeliveryReceipt, MailPolicy
  (allowed scopes/commands, approval threshold, retention bound, attachment
  bounds), MailChange/MailChangeKind (provider-neutral change feed shape).
- `crates/nexus-email/src/error.rs`: MailError with SPEC-006 codes
  (Validation/Authorization/Policy/NotFound/Conflict/Unavailable/Timeout/
  Verification/Vocabulary/External/RateLimit/Internal), correlation + resource,
  redacted surface.
- `crates/nexus-email/src/provider.rs`: EmailProvider fail-closed port
  (list_mailboxes/list_threads/fetch_message/list_attachments/save_draft/send/
  reply/forward/archive/label/delete/message_state/changes) + enforce_mail_policy
  (command + scope + approval gates BEFORE any provider mutation - SPEC-014
  behavior 8).
- `crates/nexus-email/src/verifier.rs`: MailVerifier exact-target
  (Verified/Mismatch/Unknown/UnrelatedChange; unrelated message never verifies
  the target).
- `crates/nexus-email/tests/dependency_direction.rs`: contract crate depends
  only on nexus-domain + serde + serde_json + sha2.
- `tests/email` (nexus-email-e2e): M1 surface tests compose the public
  contracts (canonical message serde, draft/send separation, unbound provider
  fails closed, attachment digest never raw content).
- `scripts/ep026-m1-tests.sh`: real gate with vacuity guards (EP-001 masking
  class); pre-created node script M1 case ran only the artifact check.
- Workspace: crates/nexus-email + tests/email registered in Cargo.toml;
  Cargo.lock updated; both registered in the expected-files fence.
- 18 nexus-email ep026_unit tests + 4 nexus-email-e2e ep026_unit tests green;
  clippy -D warnings clean; fmt clean; scope audit EP-026: ok; reality gate:
  ok; security check: ok; license gate: ok; dependency audit: ok.

# 12. Surprises & Discoveries

- 2026-08-19: `cargo fmt -p <crate>` is not a valid invocation (cargo fmt has
  no -p); format via `rustfmt --edition 2021 crates/nexus-email/src/*.rs`.
- 2026-08-19: `cargo test <filter>` under the rtk-tee wrapper compresses the
  stream in interactive mode; appending to a log file (the gate pattern)
  preserves the raw `running N tests` / `test result: ok` sentinels the
  vacuity guards require.
- 2026-08-19: clippy `manual-contains` prefers `Vec::contains(&x)` over
  `.iter().any(|v| *v == x)` for Copy elements - applied to MailPolicy and
  SendRequest.

# 13. Decision Log

- 2026-08-19: nexus-email contract crate depends on nexus-domain + serde +
  serde_json + sha2 (sha256 for Attachment digest evidence, SPEC-014
  inputs/outputs; nexus-telephony precedent). Alternatives: raw attachment
  content in domain - rejected (SPEC-014 says artifacts carry a digest, never
  raw content; SECURITY.md data classification). Consequence: attachments are
  referenced by storage_ref + digest until ArtifactStore materialization in
  M2/M3. Reversal: ADR + schema update. Security: digest-only keeps private
  content out of domain/telemetry. License: MIT/Apache-2.0 audit-gated.
- 2026-08-19: MailScope is an explicit enum with one authority per variant
  (Read/Send/Draft/Reply/Forward/Archive/Label/Attachments) instead of a
  boolean read/send pair. Evidence: acceptance obligation 2 (read and send
  scopes separate) + SPEC-014 behavior 2 (draft/reply/forward/archive/label
  have separate scopes). Alternatives: single read_write flag - rejected
  (would widen authority). Reversal: ADR + schema update.
- 2026-08-19: M1 gate script scripts/ep026-m1-tests.sh registered in the
  expected-files fence alongside Cargo.toml/Cargo.lock (EP-024 precedent for
  workspace-root registration). Evidence: scope audit EP-026: ok.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
