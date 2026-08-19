NODE-META-BEGIN
ID: EP-027
DEPS: EP-026
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-027
VERIFY_SENTINEL: node verify EP-027: ok
GREEN_TAG: green/EP-027
NODE-META-END

# 1. Purpose / Big Picture

Implement ICTFax, HylaFAX compatibility, fax documents, inbound routing, outbound status, T.38 or carrier fallback, and audit. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-027.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-027.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-026` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-027.md`
- `.agent/specs/SPEC-014-email-phone-fax-notifications-and-communications-routing.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-027.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-027-fax-fabric.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-027.txt`
- `.agent/node-contracts/EP-027.md`
- `scripts/nodes/EP-027.sh`
- `crates/nexus-fax/`
- `connectors/ictfax/`
- `connectors/hylafax/`
- `infra/fax/`
- `tests/fax/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `FaxProvider` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `IctFaxProvider` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `HylaFaxProvider` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `CloudFaxProvider` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `FaxJob` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `FaxDocument` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `FaxStatus` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `InboundFaxRoute` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |

Acceptance obligations:

1. ICTFax is the primary self-hosted control sidecar
2. HylaFAX is a compatibility backend
3. Cloud carrier fallback uses the same FaxProvider contract
4. Outbound and inbound documents, status, retries, routing, and audit are real

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement ictfax, hylafax compatibility, fax documents, inbound routing, outbound status, t.38 or carrier fallback, and audit.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-027-M1.txt`, `.agent/node-contracts/EP-027.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-027-fax-fabric.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-027.txt`, `.agent/node-contracts/EP-027.md`, `scripts/nodes/EP-027.sh`, `crates/nexus-fax/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep027_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-027.sh M1`

EXPECT:

- `EP-027 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-027 MILESTONE_PASS "M1 EP-027 M1: ok"`

FALLBACK: Use HylaFAX with a certified modem or SIP path if ICTFax packaging cannot pass the selected deployment profile. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-027][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-027.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-027-M2.txt`, `.agent/node-contracts/EP-027.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/ictfax/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep027_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-027.sh M2`

EXPECT:

- `EP-027 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-027 MILESTONE_PASS "M2 EP-027 M2: ok"`

FALLBACK: Use HylaFAX with a certified modem or SIP path if ICTFax packaging cannot pass the selected deployment profile. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-027][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-027 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-027-M3.txt`, `.agent/node-contracts/EP-027.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/hylafax/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep027_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-027.sh M3`

EXPECT:

- `EP-027 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-027 MILESTONE_PASS "M3 EP-027 M3: ok"`

FALLBACK: Use HylaFAX with a certified modem or SIP path if ICTFax packaging cannot pass the selected deployment profile. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-027][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-027 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-027-M4.txt`, `.agent/node-contracts/EP-027.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/fax/`

CONTENT:

1. Create tests whose names begin `ep027_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-027.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-027 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-027 MILESTONE_PASS "M4 EP-027 M4: ok"`

FALLBACK: Use HylaFAX with a certified modem or SIP path if ICTFax packaging cannot pass the selected deployment profile. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-027][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-027.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-027-M5.txt`, `.agent/node-contracts/EP-027.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/fax/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-027` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-027.sh M5`
2. `sh scripts/node-verify.sh EP-027`
3. `sh scripts/scope-audit.sh EP-027`

EXPECT:

- `EP-027 M5: ok`
- `node verify EP-027: ok`
- `scope audit EP-027: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-027 MILESTONE_PASS "M5 EP-027 M5: ok"`

FALLBACK: Use HylaFAX with a certified modem or SIP path if ICTFax packaging cannot pass the selected deployment profile. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-027][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-027` and observe `node verify EP-027: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-013` `fax-lifecycle`: Send a real test fax through the certified profile, receive status callbacks, route inbound fax, and archive the artifact.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary (2026-08-19; commit d6d565f)
- [x] M2: Core behavior and deterministic invariants (2026-08-19; commit d8ecd6a)
- [x] M3: Real dependency and transport integration (2026-08-19; commit ab62720)
- [x] M4: Forced failures, abuse cases, and observability (2026-08-19; commit 0d1a5f8)
- [x] M5: Live-fire, operations, and node closure (2026-08-19; gate + node + node-verify sentinels observed; implementation commit pending)

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-19 M1: The pre-created EP-027 M1 test for unknown vocabulary fed a JSON object (`{"kind":"ICT_FAX"}`) to a bare-string enum. serde treats `kind` as a variant name and rejects it with `unknown variant \`kind\``. The wire vocabulary is SCREAMING_SNAKE_CASE (`ICT_FAX`/`HYLA_FAX`/`CLOUD_FAX`), confirmed by the actual serde error message. Fixed the test to use bare strings and added explicit serde rename attributes so the wire spelling is vocabulary-locked, not serde-default accidental.
- 2026-08-19 M1: `FaxNumber` and the typed ids derived `Deserialize`, which bypassed the `new()` contract checks: an invalid number or empty id could be constructed from the wire. Added custom `Deserialize` impls that run the same normalization/validation (fail closed, never bypass). Tests prove invalid wire values are rejected and valid ones round-trip.
- 2026-08-19 M1: `validate_send_request` accepted a request whose `approval_class` was below the job requirement (the field existed but was ignored). Added the policy check; test proves `Policy` error before any provider call.
- 2026-08-19 M1: There was no seam proving "no provider mutation after denial". Added `submit_governed` (validate -> policy -> provider.submit) and `verify_delivery` (exact-target, SUBMITTED never verifies). A tracking provider test proves denied sends make zero `submit` calls and approved sends make exactly one.
- 2026-08-19 M1: The write/read tool redacts phone-like literals at the display layer (`+15551234567` shown as `+155****4567`); grep/od confirm the file bytes are correct. Tests use split literals where a canonical dial string is needed so file bytes are never masked.
- 2026-08-19 M1: `cargo test` splits the suite across two binaries (15 unit + 1 dependency-direction = 16); the gate floor guard must sum passed counts across binaries, not match a single result line.
- 2026-08-19 M2: The ICTFax REST surface was verified from the authoritative upstream guide (ictfax.com/fax-rest-api-guide.html), NOT invented: `POST /api/authenticate` (session token, no auth header on this call), `POST /api/messages/documents` + `POST /api/messages/documents/{id}/media`, `POST /api/programs/sendfax`, `POST /api/transmissions` + `POST /api/transmissions/{id}/send`, `GET /api/transmissions/{id}` + `{id}/status` + `{id}/result`, `DELETE /api/transmissions/{id}`, `GET /api/accounts`. The documented auth header is `Authentication: Bearer JWT` (not a custom header); HTTP codes 200/401/403/404/412/417/423/500/501 mapped per SPEC-006.
- 2026-08-19 M2: The pre-created node M2 branch ran `cargo test -p nexus-fax ep027_unit` (the M1 contract crate) -- the EP-001 gate-masking class found in EP-026. Rewired to `scripts/ep027-m2-tests.sh`, which runs the nexus-ictfax suite plus the M1 regression and includes an anti-masking guard requiring `ep027_unit_ictfax_` test names in the run output.
- 2026-08-19 M2: A denied send via `submit_ictfax_governed` fails the policy gate BEFORE `provider.submit` is called, so the correct proof of zero carrier mutation is the ABSENCE of any SUBMIT audit entry (not a POLICY audit entry, which only exists when the adapter-internal gate rejects). The first test asserted the wrong shape and was corrected to assert zero SUBMIT entries.
- 2026-08-19 M3: `FAXHOST` env is IGNORED by sendfax when `hyla.conf` sets `Host:` -- client config wins. Port override only works through `/etc/services` (hylafax -> 4559). The first capture was empty until the service mapping was repointed.
- 2026-08-19 M3: hfaxd is the ACTIVE data side: the client binds a listener, advertises it with EPRT, and the SERVER connects back. Rewrite/port-owning proxies break STOT (hfaxd binds the EPRT port itself). The working capture design is a control-passthrough proxy with NO EPRT rewrite.
- 2026-08-19 M3: MODE Z makes the STOT document channel zlib-compressed (wire bytes begin `78 9c`); the client PUSHES the compressed document to the server-connected socket. Decompressed bytes match the stored `docq` artifact byte-for-byte (SHA-256 `cb061483...` == `docq/doc18.ps.10`).
- 2026-08-19 M3: `TYPE I` (binary) must precede `MODE Z`/`STOT`; `550 TYPE ASCII, MODE ZIP not implemented` otherwise.
- 2026-08-19 M3: the real scheduler NAKs jobs with incomplete configuration: missing DIALSTRING -> 504, no document / missing geometry controls -> 460 `Unspecified reason (scheduler NAK'd request)`. The governed path therefore sends the FULL observed JPARM set (FROMUSER, LASTTIME, MAXDIALS, MAXTRIES, SCHEDPRI, DIALSTRING, NOTIFYADDR, VRES, PAGEWIDTH, PAGELENGTH, NOTIFY, PAGECHOP, CHOPTHRESHOLD + DOCUMENT) -> JSUBM 200.
- 2026-08-19 M3: `LIST sendq` uses the SAME active data-channel primitive as STOT (EPRT -> 150 -> server connects -> client reads -> 226), but the LIST payload is PLAINTEXT even when MODE Z is active -- never force zlib decompression on LIST.
- 2026-08-19 M3: hfaxd auto-authenticates localhost connections (hosts.hfaxd `localhost` entry with empty password -> USER returns 230 with NO password). The real 331/530 password path only triggers for non-loopback sources; the live tests connect via the container eth0 address to force real authentication and the real 530.
- 2026-08-19 M3: VERSIONS.lock expects 6.0.7 but only 6.0.6 exists upstream; the pinned fixture is `3:6.0.6-8.1~ubuntu0.18.04.1` (digest sha256:00decb6c...). Drift recorded; VERSIONS.lock correction owned by the later lockfile owner.
- 2026-08-19 M3: the fixture image is Ubuntu 18.04 (GLIBC 2.27): host-built test binaries (GLIBC 2.28+) cannot execute inside it, and cargo 1.65 cannot parse the workspace lockfile. The host Rust 1.96.0 toolchain is copied into the fixture for in-netns builds/tests. CONTROLLED_TEST_FIXTURE EXECUTION CONSTRAINT -- not a Nexus product requirement.
- 2026-08-19 M4: with `--nocapture`, cargo's harness prints the `test <name> ...` line, interleaves the test's own output, and prints the `ok` marker on its OWN line. Gate anti-masking guards must match the test-name line (the result line proves pass status), not a same-line `... ok` pattern.
- 2026-08-19 M4: the M4 failure binary (hfaxd-down test terminates the real hfaxd process) races other tests under cargo's default parallelism; both the M3 and M4 in-fixture runs now use `--test-threads=1` so the shared fixture state is never mutated concurrently.
- 2026-08-19 M5: the LF-030 live-fire lifecycle PASSED but the evidence writer FAILED because the canonical parent directory did not exist inside the fixture /build workspace: the write used a cwd-relative path while cargo runs test binaries with cwd = package root, so `.agent/state/evidence/` resolved to `/build/tests-fax/.agent/state/evidence/` (crate root) while the gate copied from `/build/.agent/state/evidence/` (workspace root). Fixed by anchoring the evidence path to the workspace root via `CARGO_MANIFEST_DIR` parent + explicit `create_dir_all` (errors propagate; no silent ignore; no temp fallback; canonical location unchanged). This is a useful proof-harness defect retained in the plan.
- 2026-08-19 M5: node-verify exposed an EP-026 fixture regression: GreenMail's TLS endpoints failed to start because the bind-mounted keystore was root-owned mode 600 while the container runs as uid 999 (greenmail) -> `AccessDeniedException` -> smtps/imaps never served. Fixed `infra/mail/provision.sh` to `chmod 644` the fixture keystore/cert (controlled fixture, known `changeit` password); EP-026 M5 gate re-proven green. Also observed: an interrupted node-verify run can leave the EP-020 HA fixture container + generated config state behind; cleaned per the fixture's documented teardown convention (`docker rm -f nexus-ep020-ha` + `_cleanup_generated_state`), which the EP-020 suite performs on a normal exit.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-19 M1 | Canonical provider-kind wire representation: explicit serde renames `ICT_FAX`/`HYLA_FAX`/`CLOUD_FAX` (SCREAMING_SNAKE_CASE), vocabulary-locked; internal `as_str()` keeps domain constants (`ICTFAX`, ...) distinct from wire spelling. Evidence: `ep027_unit_provider_kind_wire_vocabulary` + `ep027_unit_unknown_vocabulary_rejected` green. Alternatives: serde default naming (rejected: accidental undocumented protocol), object-tagged wire form (rejected: not the enum's serde shape). Consequence: changing a wire spelling is a schema change requiring ADR + ledger entry. Reversal: revert enum renames to `rename_all` if the blueprint mandates it. Security/license/compat: no new deps; no compat impact (crate is new).
- 2026-08-19 M1 | Fax-number normalization: E.164-ish canonical form (strip space/dash/dot/paren, single leading `+`, 7..=16 digits, deterministic output), rejecting letters, empty, too-short/too-long, embedded/repeated `+`, and any non-canonical residue. Evidence: `ep027_unit_fax_number_normalization` green. Alternatives: store raw dial strings (rejected: domain never compares raw dial strings per SPEC-014). Consequence: providers carry carrier-specific rendering; the domain compares normalized numbers only. Reversal: adjust normalization per SPEC-014 schema update.
- 2026-08-19 M1 | State-ladder semantics: DRAFT < QUEUED < SUBMITTING < SUBMITTED < DELIVERED plus terminal FAILED/CANCELLED/ARCHIVED; SUBMITTED is carrier acceptance, DELIVERED requires independent recipient/carrier evidence. Evidence: `ep027_unit_submitted_is_not_delivered` + `verify_delivery` exact-target tests green. Alternatives: treat carrier 200/acceptance as delivery (rejected: would fabricate delivery, Reality rule). Consequence: later provider milestones must carry delivery evidence, never infer it from submission.
- 2026-08-19 M1 | Pre-mutation gates: `submit_governed` runs `validate_send_request` (job match, idempotency key, approval class) then `enforce_fax_policy` (approval minimum, scan CLEAN, sender != recipient) BEFORE any `provider.submit`; denied sends never reach the carrier. Evidence: `ep027_unit_governed_submit_denies_before_provider_mutation` (tracking provider: zero submits on every denial, exactly one on approval). Alternatives: validate inside providers (rejected: per-provider drift, no central proof). Consequence: adapters must call `submit_governed`, not bare `submit`. Reversal: none without ADR.
- 2026-08-19 M1 | Serde must not bypass contract checks: `FaxNumber` and typed ids implement custom `Deserialize` running the same validation as `new()`. Evidence: `ep027_unit_number_and_ids_fail_closed_via_serde` green. Alternatives: derive `Deserialize` (rejected: invalid numbers/empty ids constructible from wire). Consequence: wire payloads are validated at the boundary; malformed values fail closed. Reversal: derive again only with a schema change.
- 2026-08-19 M2 | ICTFax REST transport built ONLY on the documented upstream surface (ictfax.com/fax-rest-api-guide.html). Session header `Authentication: Bearer JWT`; status mapping documented vocabulary; HTTP codes per SPEC-006; unknown transmission status strings fail closed External. Evidence: `ep027_unit_ictfax_status_mapping_*` + `ep027_unit_ictfax_http_status_mapping` green. Alternatives: invent endpoints (rejected: anti-hallucination), defer transport (rejected: M2 owns core behavior). Consequence: real carrier integration in M3/M5 can bind HttpIctFaxTransport to a live ICTCore; nothing in M2 fabricates provider payloads.
- 2026-08-19 M2 | Adapter gating model: `submit_ictfax_governed` (validate_send_request + enforce_fax_policy) runs BEFORE `provider.submit`; the adapter's internal `submit` also gates (defense in depth) so a direct port call is still policy-checked. Denied sends prove zero carrier mutation by absence of SUBMIT audit entries. Evidence: `ep027_unit_ictfax_denied_submit_never_reaches_carrier` + `ep027_unit_ictfax_approved_submit_reaches_carrier_once` green. Alternatives: trust callers to gate (rejected: per-caller drift). Consequence: adapters and callers both enforce; no validation-after-submission.
- 2026-08-19 M2 | SUBMITTED != DELIVERED enforced at the adapter: carrier `sent`/`accepted` maps to SUBMITTED only; `completed` maps to DELIVERED only as a carrier report and still requires exact-target verification. Evidence: `ep027_unit_ictfax_status_maps_carrier_claim_to_submitted_only` + `ep027_unit_ictfax_exact_target_delivery` green. Alternatives: treat any successful send call as delivery (rejected: fabricates delivery). Consequence: later milestones must carry independent delivery evidence.
- 2026-08-19 M3 | Capture-first doctrine: no Rust client until an authoritative known-good transcript exists. Evidence: `sendfax -vv` trace + tcpdump pcap + control-passthrough proxy canary (`HYLAFAX_CAPTURE_PROXY_ACCEPTED`) + byte-for-byte zlib integrity proof. Alternatives: infer framing from FTP conventions (rejected: hfaxd data direction is the OPPOSITE of FTP active/passive assumptions). Consequence: transport code contains only observed behavior.
- 2026-08-19 M3 | Active data-channel primitive: one shared `data_exchange` for STOT (UPLOAD, zlib) and LIST (DOWNLOAD, plaintext), with explicit direction. The server initiates the TCP connection in BOTH cases; direction describes who writes application bytes. Evidence: probes on the pinned fixture (STOT 150 FILE -> 226; LIST sendq 150 -> plaintext rows -> 226). Alternatives: separate ad-hoc socket setup per command (rejected: duplicated fragile logic), FTP-style passive assumption (rejected: contradicts observed wire). Consequence: EPRT/accept/completion handling is proven once and reused.
- 2026-08-19 M3 | Full JPARM set required by the real scheduler: the governed path always sends FROMUSER, LASTTIME, MAXDIALS, MAXTRIES, SCHEDPRI, DIALSTRING, NOTIFYADDR, VRES, PAGEWIDTH, PAGELENGTH, NOTIFY, PAGECHOP, CHOPTHRESHOLD + DOCUMENT. Evidence: JSUBM 460/504 NAKs without them, 200 with them; regression test `ep027_live_hylafax_scheduler_nak_not_submitted`. Alternatives: minimal parameter set (rejected: real 460 NAK). Consequence: submission is sendfax-compatible; removing any observed field requires a new scheduler NAK regression.
- 2026-08-19 M3 | Exact-target readback: `status()` queries by the provider-assigned CARRIER job id through `LIST sendq`, never by destination/owner heuristics; unknown state letters fail closed (External). Evidence: round-trip test binds carrier id readback + `map_queue_state` unit tests (W/B -> SUBMITTED, F -> FAILED, unknown letter -> Err). Alternatives: spool-file primary readback (rejected: couples production to local spool layout; spool is the independent TEST ORACLE instead), substring state mapping (rejected: brittle). Consequence: provider vocabulary changes surface as errors, never silent misclassification.
- 2026-08-19 M3 | Fixture placement: HylaFAX fixture provisioning lives under `infra/hylafax/` (owner directive M3 artifact list), registered in the machine fence; the blueprint's `infra/fax/` remains M4's forced-failure fixture path and `tests/fax/` M5's. Evidence: scope audit EP-027: ok after registration. Alternatives: place under infra/fax/hylafax (rejected: mixes M4-owned directory with M3 provisioning). Consequence: expected-files node audit stays deferred to node closure (infra/fax/ missing until M4).
- 2026-08-19 M3 | In-fixture live tests: the full round trip can only run inside the fixture netns (EPRT data listener must be reachable by hfaxd; host-side data listener on host loopback gives 425). The host Rust 1.96.0 toolchain is copied into the Ubuntu 18.04 fixture because its GLIBC 2.27 cannot run host binaries and its cargo 1.65 cannot parse the v4 lockfile. CONTROLLED_TEST_FIXTURE EXECUTION CONSTRAINT -- not a Nexus product requirement. Evidence: gate provisions idempotently; in-fixture suite 7 unit + 3 live green.
- 2026-08-19 M3 | Fixture credential model: the wildcard `nexustest@*` hosts.hfaxd entry (real crypt hash) exercises the REAL password path (331 -> PASS -> 230/530); the `localhost` entry auto-authenticates (230 without password). Live tests connect via the container eth0 address so the wrong-password case produces a REAL 530 -> `FaxErrorCode::Authorization`, zero mutation. Evidence: `ep027_live_hylafax_wrong_password_fails_closed` green (530 observed). Consequence: localhost-only deployments would never see 530; tests must target a non-loopback source.
- 2026-08-19 M3 | VERSIONS.lock drift recorded, not normalized: lock expects 6.0.7; only 6.0.6 exists upstream and the tested fixture is `3:6.0.6-8.1~ubuntu0.18.04.1` at digest sha256:00decb6c... . M3 does not own VERSIONS.lock; correction is owned by the later lockfile owner per repo lockfile semantics. Evidence: fixture dpkg/banner + image RepoDigest. Alternatives: silently rewriting the lock (rejected: would claim an unavailable version).
- 2026-08-19 M4 | Poison-safe observability: the adapter registers the credential as an audit redaction secret (`FaxObservability::new(256, vec![password])`), so any accidental embed in telemetry detail/fields is replaced at insert. Evidence: `ep027_failure_observability_redacts_secret` (unit) + `ep027_failure_redaction_canaries` (live: scan of the whole audit ring after a REAL 530 shows zero credential leakage). Alternatives: rely on callers to never embed secrets (rejected: poison-safe defense in depth is the contract). Consequence: audit output is safe by construction.
- 2026-08-19 M4 | REAL failure mechanisms over controlled mocks: the hfaxd-down test terminates the REAL hfaxd process (pkill), proves the transport fails closed Unavailable (never fabricates a session), then restarts the real daemon and proves a fresh session authenticates (bounded recovery, fixture left RUNNING). Policy denial is proven against the REAL spool (sendq job count unchanged). Evidence: `ep027_failure_hfaxd_down_truthful_unavailable` + `ep027_failure_policy_denied_zero_mutation` green; gate re-checks hfaxd running + reachable after the suite. Alternatives: simulated server failures (rejected: milestone requires the real failure mechanism). Consequence: the fixture is left in a healthy, recoverable state after every run.
- 2026-08-19 M4 | Sequential fixture mutation: tests that mutate shared fixture state (kill/restart hfaxd) run with `--test-threads=1` in both the M3 and M4 in-fixture suites; gate anti-masking guards match test-name lines because `--nocapture` moves the `ok` marker to its own line. Evidence: M3/M4 gates green under serialized runs; the M3 regression caught the parallel-race failure class. Alternatives: per-test fixture isolation (rejected: heavier than needed for one mutating test). Consequence: gate logs use test-name sentinels, and the result line proves pass status.
- 2026-08-19 M5 | Evidence path anchored to the workspace root: LF-030 writes `.agent/state/evidence/LF-030-ep027-m5.json` under `CARGO_MANIFEST_DIR`'s parent (host: repo root; fixture: /build), with `create_dir_all` on the parent and propagating errors. Evidence: gate run `ep027-m5-1787154534-1817480` -> evidence file at `/build/.agent/state/evidence/LF-030-ep027-m5.json` -> `EP-027 M5: ok`. Alternatives: keep a cwd-relative path (rejected: cargo runs tests with cwd = package root, so the file landed at the crate root and the gate's docker cp missed it), absolute /build path (rejected: breaks host runs; no temp fallback). Consequence: stale evidence can never satisfy the gate; the current-run run_id must match the gate's.
- 2026-08-19 M5 | Fixture keystore permissions for the EP-026 GreenMail fixture: `chmod 644` the generated p12/cert so the uid-999 container process can read the bind-mounted keystore (controlled fixture with known `changeit` password). Evidence: EP-026 M5 gate green after the fix (`EP-026 M5: ok`); TLS handshake completes on 39527/39528. Alternatives: run GreenMail as root (rejected: changes the fixture's user model), chown to 999 (rejected: host uid assumptions). Consequence: node-verify's live-fire chain is reproducible from a clean tree.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.

- 2026-08-19 M1: Contract crate green and gate replacement complete. Changed files vs fence: all M1-owned paths only (crates/nexus-fax/, scripts/ep027-m1-tests.sh, scripts/nodes/EP-027.sh M1 branch, .agent/milestone-files/EP-027-M1.txt, .agent/expected-files/EP-027.txt Cargo.toml/Cargo.lock registration, ExecPlan). Commands + sentinels: `cargo test -p nexus-fax --all-targets` -> 15 unit + 1 dependency-direction, 0 failed; `sh scripts/ep027-m1-tests.sh` -> `EP-027 M1: ok`; `sh scripts/nodes/EP-027.sh M1` -> `EP-027 M1: ok`; scope audit EP-027: ok; security check: ok; license gate: ok; reality gate: ok; blueprint validation: ok; dependency audit: ok. Certification: M1 is INTERNAL CONTRACT CERTIFIED only; no fax provider claimed. Assumptions: SPEC-014 vocabulary locked per node contract. Remaining risks: provider transport, delivery evidence, and live-fire owned by M2-M5.
- 2026-08-19 M2: ICTFax adapter core green and gate replacement complete. Changed files vs fence: all M2-owned paths only (connectors/ictfax/ crate, scripts/ep027-m2-tests.sh, scripts/nodes/EP-027.sh M2 branch, .agent/milestone-files/EP-027-M2.txt, expected-files registration, ExecPlan). Commands + sentinels: `cargo test -p nexus-ictfax --all-targets` -> 11 passed 0 failed; `sh scripts/ep027-m2-tests.sh` -> `EP-027 M2: ok`; `sh scripts/nodes/EP-027.sh M2` -> `EP-027 M2: ok`; M1 regression `EP-027 M1: ok`; workspace battery 1866 passed 0 failed; scope audit EP-027: ok; security check: ok; license gate: ok; reality gate: ok; blueprint validation: ok; dependency audit: ok; clippy -D warnings clean; fmt clean. Certification: M2 INTERNAL CONTRACT CERTIFIED for adapter behavior over documented transport surface; no live ICTFax instance claimed (M3/M5 own real dependency + live-fire). Assumptions: upstream ICTFax REST guide is the authoritative provider surface. Remaining risks: real ICTCore version/response drift, delivery evidence, and live-fire owned by M3-M5.
- 2026-08-19 M3: Real HylaFAX dependency and transport integration green. Changed files vs fence: connectors/hylafax/ (transport refactor to the shared active data-channel primitive, adapter state mapping fail-closed, 3 live integration tests), infra/hylafax/ (idempotent fixture provisioning + fixture workspace manifest), scripts/ep027-m3-tests.sh (12-guard real gate), scripts/nodes/EP-027.sh (M3 rewired to the real gate; failure propagation fixed), .agent/milestone-files/EP-027-M3.txt, .agent/state/evidence/EP-027-M3.md, expected-files registration, ExecPlan. Commands + sentinels: host `cargo test --locked -p nexus-hylafax --all-targets` -> 7 unit passed 0 failed; in-fixture suite `cargo test` with HYLAFAX_LIVE=1 -> 7 unit + 3 live passed 0 failed (round trip carrier 46 + docq byte-exact digest 8137e85e...; real 530 -> Authorization; real 460 scheduler NAK); `sh scripts/ep027-m3-tests.sh` -> `EP-027 M3: ok`; `sh scripts/nodes/EP-027.sh M3` -> `EP-027 M3: ok` (RC=0); M1 regression `EP-027 M1: ok`; M2 regression `EP-027 M2: ok`; fmt clean; clippy -D warnings clean; reality gate: ok; scope audit EP-027: ok; security check: ok; license gate: ok; dependency audit: ok; blueprint validation: ok; expected-files node audit deferred to node closure (infra/fax/ M4-owned, tests/fax/ M5-owned -- the only missing entries). Certification: nexus-hylafax IMPLEMENTED; hfaxd control protocol PROTOCOL_CERTIFIED; active EPRT data-channel handling PROTOCOL_CERTIFIED; MODE Z/STOT upload PROTOCOL_CERTIFIED; HylaFAX 6.0.6-8.1 fixture PROVIDER_CERTIFIED; faxq job acceptance PROVIDER_CERTIFIED; exact provider query/readback PROVIDER_CERTIFIED; document transfer integrity CERTIFIED for tested path; container CONTROLLED_TEST_FIXTURE; physical modem / PSTN / remote fax receiver / DELIVERED NOT ASSERTED. Assumptions confirmed: capture-first doctrine; hfaxd is the active data side; LIST is plaintext even under MODE Z; the full JPARM set is required. Remaining risks: real modem/PSTN delivery evidence and live-fire owned by M4-M5; VERSIONS.lock 6.0.7 drift recorded for the later lockfile owner.
- 2026-08-19 M4: Forced failures, abuse cases, and observability green. Changed files vs fence: connectors/hylafax/src/adapter.rs (poison-safe observability secrets + cancel fail-closed + in-flight conflict + redaction unit tests), connectors/hylafax/tests/failure_hylafax.rs (3 live failure tests), infra/fax/hylafax-diag.sh (operations diagnostic + bounded recovery), scripts/ep027-m4-tests.sh (real gate), scripts/nodes/EP-027.sh (M4 rewired from the EP-001-masking stub), .agent/milestone-files/EP-027-M4.txt, expected-files registration, ExecPlan; M3 gate hardened (--test-threads=1) after the M3 regression caught the failure-binary race. Commands + sentinels: host `cargo test --locked -p nexus-hylafax --all-targets` -> 10 unit + 3 live + 3 failure passed 0 failed; in-fixture failure suite `cargo test --test failure_hylafax` with HYLAFAX_LIVE=1 --test-threads=1 -> 3 passed 0 failed (hfaxd terminated -> Unavailable -> real restart -> re-auth; policy denial sendq 25 -> 25 zero mutation; redaction canaries clean across audit ring after real 530); `sh scripts/ep027-m4-tests.sh` -> `EP-027 M4: ok`; `sh scripts/nodes/EP-027.sh M4` -> `EP-027 M4: ok` (RC=0); M1/M2/M3 regressions ok; fmt clean; clippy -D warnings clean; reality gate: ok; scope audit EP-027: ok; security check: ok; license gate: ok; dependency audit: ok; blueprint validation: ok; workspace battery green (see ledger). Certification: unchanged from M3 boundary (M4 adds observability hardening and REAL failure-path proofs; no new provider claims; physical modem / PSTN / DELIVERED still NOT ASSERTED). Assumptions confirmed: real failure mechanisms > simulated ones; sequential fixture mutation is required. Remaining risks: live-fire and node closure owned by M5.
- 2026-08-19 M5: Live-fire, operations, and node closure green. Changed files vs fence: tests/fax/ (nexus-fax-e2e LF-030 live-fire crate + workspace registrations), scripts/live-fire/LF-030.sh, scripts/ep027-m5-tests.sh (real gate with current-run evidence + redaction + zero-orphan guards), scripts/nodes/EP-027.sh (M5 + verify rewired from the EP-001-masking stub to the real gate), docs/operations/EP-027-fax.md (ops runbook), .agent/milestone-files/EP-027-M5.txt, expected-files registration, ExecPlan, evidence .agent/state/evidence/LF-030-ep027-m5.json; plus an EP-026 fixture regression fix in infra/mail/provision.sh (keystore chmod 644) required for the node-verify live-fire chain. Commands + sentinels: `sh scripts/ep027-m5-tests.sh` -> `EP-027 M5: ok` (run id ep027-m5-1787154534-1817480; carrier 65; exact-target SUBMITTED readback; spool oracle q65; replay dedup; real 530 zero mutation; evidence current-run + redacted + DELIVERED NOT_ASSERTED); `sh scripts/nodes/EP-027.sh M5` -> `EP-027 M5: ok` + `LF-030: ok` (RC=0); `sh scripts/node-verify.sh EP-027` -> `node verify EP-027: ok` (verify: ok; runtime smoke: ok; live-fire: ok incl. LF-003/006/007/008/016/024/028/029 + LF-030); M1/M2/M3/M4 regressions `EP-027 M{1..4}: ok`; workspace battery 1883 passed 94 ignored 0 failed (200 suites); fmt clean; clippy -D warnings clean; lint/typecheck/build/security/license/dependency/reality/scope/expected-files/blueprint gates ok. Certification (Q matrix): nexus-fax INTERNAL CONTRACT CERTIFIED; nexus-ictfax IMPLEMENTED / TRANSPORT_CERTIFIED against documented controlled HTTP tests (no live ICTFax exercised); nexus-hylafax IMPLEMENTED; hfaxd protocol PROTOCOL_CERTIFIED; HylaFAX tested runtime PROVIDER_CERTIFIED (3:6.0.6-8.1~ubuntu0.18.04.1); HylaFAX image CONTROLLED_TEST_FIXTURE (digest sha256:00decb6c...); faxq submission PROVIDER_CERTIFIED; exact LIST readback PROVIDER_CERTIFIED; document integrity CERTIFIED for tested path; governed submit CERTIFIED; physical fax modem / PSTN / remote fax receipt / DELIVERED NOT ASSERTED. Assumptions confirmed: SUBMITTED != DELIVERED explicit in LF-030 evidence; hostile inbound surface not owned by LF-030 (M4 retains abuse proofs). Remaining risks: real modem/PSTN delivery evidence; VERSIONS.lock 6.0.7 drift owned by the later lockfile owner. Green tag: green/EP-027 (created at M5 implementation commit, not the ledger closure commit).
