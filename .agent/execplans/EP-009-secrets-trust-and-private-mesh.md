NODE-META-BEGIN
ID: EP-009
DEPS: EP-008
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-009
VERIFY_SENTINEL: node verify EP-009: ok
GREEN_TAG: green/EP-009
NODE-META-END

# 1. Purpose / Big Picture

Implement OpenBao, SOPS and age bootstrap, device stores, certificate authority, Headscale, WireGuard, and mTLS. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-009.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-009.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-008` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-009.md`
- `.agent/specs/SPEC-005-authentication-authorization-secrets-trust-and-multi-user-privacy.md`
- `.agent/specs/SPEC-020-privacy-data-governance-retention-export-and-deletion.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-009.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-009-secrets-trust-and-private-mesh.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-009.txt`
- `.agent/node-contracts/EP-009.md`
- `scripts/nodes/EP-009.sh`
- `crates/nexus-trust/`
- `infra/openbao/`
- `infra/headscale/`
- `infra/pki/`
- `config/sops/`
- `tests/trust/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `SecretStore` | `nexus-trust` | Defined by EP-009; provider-neutral and versioned |
| `BootstrapSecretStore` | `nexus-trust` | Defined by EP-009; provider-neutral and versioned |
| `DeviceSecretStore` | `nexus-trust` | Defined by EP-009; provider-neutral and versioned |
| `CertificateAuthority` | `nexus-trust` | Defined by EP-009; provider-neutral and versioned |
| `ServiceIdentity` | `nexus-trust` | Defined by EP-009; provider-neutral and versioned |
| `MeshController` | `nexus-trust` | Defined by EP-009; provider-neutral and versioned |
| `CapabilityTokenIssuer` | `nexus-trust` | Defined by EP-009; provider-neutral and versioned |

Acceptance obligations:

1. No long-lived universal bearer token exists
2. Secrets are referenced by name and never enter model context
3. Services use mTLS and short-lived credentials
4. Headscale compatibility, raw WireGuard, and standard mTLS paths coexist

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement openbao, sops and age bootstrap, device stores, certificate authority, headscale, wireguard, and mtls.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-009-M1.txt`, `.agent/node-contracts/EP-009.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-009-secrets-trust-and-private-mesh.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-009.txt`, `.agent/node-contracts/EP-009.md`, `scripts/nodes/EP-009.sh`, `crates/nexus-trust/`, `tests/trust/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep009_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-009.sh M1`

EXPECT:

- `EP-009 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-009 MILESTONE_PASS "M1 EP-009 M1: ok"`

FALLBACK: Use SOPS and age plus mTLS for a fully local profile when OpenBao is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-009][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-009.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-009-M2.txt`, `.agent/node-contracts/EP-009.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/openbao/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep009_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-009.sh M2`

EXPECT:

- `EP-009 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-009 MILESTONE_PASS "M2 EP-009 M2: ok"`

FALLBACK: Use SOPS and age plus mTLS for a fully local profile when OpenBao is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-009][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-009 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-009-M3.txt`, `.agent/node-contracts/EP-009.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/headscale/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep009_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-009.sh M3`

EXPECT:

- `EP-009 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-009 MILESTONE_PASS "M3 EP-009 M3: ok"`

FALLBACK: Use SOPS and age plus mTLS for a fully local profile when OpenBao is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-009][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-009 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-009-M4.txt`, `.agent/node-contracts/EP-009.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/pki/`

CONTENT:

1. Create tests whose names begin `ep009_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-009.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-009 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-009 MILESTONE_PASS "M4 EP-009 M4: ok"`

FALLBACK: Use SOPS and age plus mTLS for a fully local profile when OpenBao is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-009][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-009.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-009-M5.txt`, `.agent/node-contracts/EP-009.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `config/sops/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-009` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-009.sh M5`
2. `sh scripts/node-verify.sh EP-009`
3. `sh scripts/scope-audit.sh EP-009`

EXPECT:

- `EP-009 M5: ok`
- `node verify EP-009: ok`
- `scope audit EP-009: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-009 MILESTONE_PASS "M5 EP-009 M5: ok"`

FALLBACK: Use SOPS and age plus mTLS for a fully local profile when OpenBao is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-009][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-009` and observe `node verify EP-009: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [x] M2: Core behavior and deterministic invariants
  - Gate: `sh scripts/nodes/EP-009.sh M2` -> `EP-009 M2: ok` (17 unit +
    19 integration + 13 failure + orphan audit ok)
  - Live proof: `cargo run --locked -p nexus-openbao --example
    sops_live_proof --offline` -> `EP-009 M2 SOPS adapter live proof: ok`
  - All gates: `scope audit EP-009: ok`, `security check: ok`,
    `license gate: ok`, `reality gate: ok`, `format check: ok`,
    `lint: ok`, `EP-009 orphan audit: ok`
- [x] M3: Real dependency and transport integration
  - Gate: `sh scripts/nodes/EP-009.sh M3` -> `EP-009 M3: ok` (12
    nexus-headscale unit + 10 ep009_integration_headscale_* pytest
    incl. Rust adapter live proof + orphan audit ok)
  - Live proof: `mesh_live_proof` example (REAL HeadscaleMeshController
    against REAL headscale/headscale:0.23.0 container over gRPC TLS +
    API key) -> `EP-009 M3 headscale live proof: ok`
  - All gates: `scope audit EP-009: ok`, `security check: ok`,
    `license gate: ok`, `reality gate: ok`, `format check: ok`,
    `lint: ok`, `EP-009 orphan audit: ok`
- [x] M4: Forced failures, abuse cases, and observability
  - Gate: `sh scripts/nodes/EP-009.sh M4` -> `EP-009 M4: ok` (14
    nexus-pki unit + 5 ep009_integration_pki_* + 6 ep009_failure_pki_*
    pytest + orphan audit ok)
  - Live proof: `pki_live_proof` example (REAL OpenBao 2.5.4 PKI
    engine as CA, real CSR issuance, real rustls 0.23.43 mTLS,
    revocation via CRL, rotation, capability boundary) ->
    `EP-009 M4 pki live proof: ok`
  - All gates: `scope audit EP-009: ok`, `security check: ok`,
    `license gate: ok`, `reality gate: ok`, `format check: ok`,
    `lint: ok`, `dependency audit: ok`, `EP-009 orphan audit: ok`
- [x] M5: Live-fire, operations, and node closure
  - Gate: `sh scripts/nodes/EP-009.sh M5` -> `EP-009 M5: ok` (nexus-trust
    + nexus-openbao + nexus-pki + nexus-headscale suites, 6
    ep009_livefire_* pytest driving the composed proof, orphan audit ok)
  - Live proof: `trust_chain_live_proof` example (REAL OpenBao 2.5.4 KV +
    AppRole least privilege + Transit capability tokens + PKI CA; REAL
    headscale 0.23.0 control-plane enrollment; REAL sops 3.13.0 + age
    1.1.1 encrypted bootstrap; REAL rustls 0.23.43 mTLS) ->
    `EP-009 M5 trust chain live proof: ok`; evidence
    `.agent/state/evidence/ep009-m5/ep009-m5-trust-chain.json`
  - One composed chain: bootstrap -> machine auth -> secret authority ->
    mesh enrollment -> PKI issuance -> mTLS -> capability -> cert
    revocation -> rotation -> mesh revocation -> response wrapping ->
    fail-closed -> state machine -> restart; kernel WireGuard dataplane
    explicitly NOT ASSERTED
  - All gates: `node verify EP-009: ok`, `scope audit EP-009: ok`,
    `expected files EP-009: ok`, adapter parity: ok (1 unique PRIME-BLOCK
    cksum), `security check: ok`, `license gate: ok`, `reality gate: ok`,
    `format check: ok`, `lint: ok`, `dependency audit: ok`,
    `EP-009 orphan audit: ok`, `verify: ok`

# 12. Surprises & Discoveries

- 2026-08-14 (M1): clippy runs with `-D warnings` in this workspace, so
  the vocabulary macro's doc comments must be declared inside the macro
  invocation (rustdoc emits "does not generate documentation for macro
  invocations" otherwise) and the `CapabilityTokenIssuer::issue` port
  (8 arguments) needs an explicit `#[allow(clippy::too_many_arguments)]`
  matching the EP-008 `capability.rs` precedent.
- 2026-08-14 (M1): re-exporting a vocabulary enum through a submodule
  that only `use`d it privately triggers E0603; the lib root re-exports
  state enums from `vocabulary` directly.
- 2026-08-14 (M2): sops 3.13.0 emits the SAME generic recovery footer
  (`Recovery failed because no master key was able to decrypt the
  file...`) for every decrypt failure, so stderr classification must
  match specific markers, never the footer. Also: the old broad
  `stderr.contains("failed to decrypt")` matcher silently misclassified
  corrupted documents (whose real stderr says `failed to decrypt and
  authenticate payload chunk`) as ProviderAuthorization -- a latent
  integrity-vs-authorization confusion fixed by the ordered classifier.
- 2026-08-14 (M3): the pinned `headscale/headscale:0.23.0` image is a
  scratch (distroless) image with NO shell -- `docker exec ... sh`
  fails, and the binary lives at `/ko-app/headscale` (ko layout). The
  CLI config search requires a config file to exist; `HEADSCALE_CONFIG`
  env makes the adapter self-contained. Remote CLI requires TLS
  (`cli.insecure` uses InsecureSkipVerify, NOT plaintext), so the test
  server must present a certificate even though
  `grpc_allow_insecure: true` is set -- the plaintext gRPC path is not
  reachable from the CLI's remote mode.
- 2026-08-14 (M3): `headscale users create` exits 1 with "user already
  exists" on repeat calls (not idempotent by default) -- the adapter
  treats "already exists" as success. `debug create-node -o json`
  omits the numeric `id` field (only register/list carry it). Node
  expiry (`nodes expire`) is observable as a non-negative `expiry`
  timestamp; `nodes delete --force` is terminal.
- 2026-08-14 (M4): OpenBao PKI `enforce_hostnames: true` rejects CSRs
  whose CN is not in `allowed_domains` -- the adapter sets CN to the
  transport DNS name and binds identity through the URI SAN (ADR-014).
- 2026-08-14 (M4): `ca/pem` returns RAW PEM, not JSON (the only
  non-JSON endpoint in the PKI surface); role `max_ttl` is an int in
  JSON; serial formats differ between sign response (lowercase,
  colon-separated) and CRL (uppercase, no colons).
- 2026-08-14 (M4): rustls 0.23 installs a CRL-carrying verifier via
  `WebPkiServerVerifier::builder(...).with_crls(...)` through the
  `.dangerous()` module -- standard API, verifier remains strict.
- 2026-08-14 (M4): `cargo deny check` flagged windows-sys 0.52.0 vs
  0.61.2 after `infra/pki` joined the workspace (rcgen -> ring made the
  0.52.0 line reachable); Windows-only crate, so a documented targeted
  skip was added (same class as untrusted/getrandom/thiserror skips).
  The dependency-audit gate had never passed on this lock lineage
  before because the split predates EP-009 (present in EP-008's lock)
  and the gate was not previously part of closure chains.
- 2026-08-14 (M5): OpenBao dev mode pre-mounts `transit/`; the concrete
  issuer's `ensure_key` treated the 400 "path is already in use" on the
  mount-enable POST as a hard failure even though its own doc comment
  promises idempotency ("re-create errors are treated as success").
  Fixed in `infra/openbao/src/token.rs` (only the "already in use"
  400 body is tolerated; a real denial still fails closed) + regression
  unit test `ep009_unit_transit_already_mounted_is_success`.
- 2026-08-14 (M5): the SecretStore adapter stores JSON payloads in
  OpenBao KV-v2 (`{"data": <json>}`); the composed proof initially
  wrote raw bytes and OpenBao rejected them ("expected type
  'map[string]interface {}'"). Proof fixed to store JSON objects.
- 2026-08-14 (M5): OpenBao response wrapping returns `"data": null` in
  the wrapped envelope; the adapter's plaintext-detection check used
  `v.get("data").is_some()` which wrongly flagged the null key as
  plaintext. Fixed to treat only NON-null data as plaintext.
- 2026-08-14 (M5): a revoked client certificate makes rustls fail the
  handshake with `Err(CertificateRevoked)` during the TLS read -- the
  proof's revocation stage must accept that Err as the expected
  fail-closed outcome (never Ok).
- 2026-08-14 (M5): headscale node ids are provider-assigned NUMERIC ids;
  the composed proof initially passed the logical name as `node_id`,
  which `wireguard_config`/`node_state` reject. Resolved by listing
  nodes and mapping names -> numeric ids (same pattern as the M3
  `mesh_live_proof` example).
- 2026-08-14 (M5): preflight requires `NEXUS_BOOTSTRAP_AGE_KEY_FILE`
  (canonical `./secrets/age-key.txt`, gitignored). The M2 "age identity
  never in repository" cleanup removed the stray file, which silently
  broke preflight for M2/M3/M4 (node-verify was not part of those
  closure chains). Regenerated the local bootstrap key at the canonical
  path; it is gitignored and never enters the repository.
- 2026-08-14 (M5): prettier wants to reformat `config/sops/*.enc.yaml`;
  ciphertext must never be reformatted (any byte change breaks the SOPS
  MAC). Added `config/sops/*.enc.yaml` to `.prettierignore` and amended
  the EP-009 fence to include `.prettierignore` (control-plane config,
  same class as deny.toml).

# 13. Decision Log

- 2026-08-14 | ADR-013 trust vocabulary | Accepted
  `TrustZone`, `TokenState`, `SecretState`, `CertificateState`,
  `ServiceIdentityState`, `MeshNodeState` as vocabulary-locked classes
  owned by `crates/nexus-trust`; `SecretReference` documented as a
  canonical term (SPEC-005 behavior 6). Evidence: ADR-013 file +
  `docs/vocabulary/README.md` trust section + `ep009_unit_vocabulary_*`
  tests. Alternatives: reusing raw strings. Consequence: every enum
  parses canonically and rejects unknown values; new synonyms require an
  ADR. Reversal: ADR supersession. Security: values never enter model
  context; `SecretValue`/`DeviceSecretValue` serialize as `<redacted>`
  and fail closed on deserialize. License: none. Compatibility:
  vocabulary update only, no wire change.
- 2026-08-14 | Fence amendment | Added `docs/vocabulary/README.md`,
  `references/ADR-013-trust-vocabulary.md`, `Cargo.toml`, `Cargo.lock`
  to `.agent/expected-files/EP-009.txt` (EP-008 M1 precedent added the
  same class of paths). Evidence: scope audit `scope audit EP-009: ok`.
  Consequence: M1's workspace registration and ADR are inside the
  machine fence. Reversal: fence edit. Security: none. License: none.
  Compatibility: none.
- 2026-08-14 | Contract surface shape | `nexus-trust` is a pure
  provider-neutral contract crate mirroring `nexus-policy`: vocabulary +
  ports + redaction wrappers; no I/O; forbidden dependency tree enforced
  by `tests/dependency_direction.rs` (SPEC-001). Evidence:
  `cargo test --locked -p nexus-trust` = 16 unit + 1 direction test
  green. Alternatives: putting ports in infra crates. Consequence: real
  adapters land in `infra/openbao`, `infra/pki`, `infra/headscale` in
  M2-M4. Reversal: none. Security: `SecretValue` never deserializes.
  License: none. Compatibility: workspace member registered in
  `Cargo.toml`.

- 2026-08-14 | OpenBao image pin | OpenBao 2.5.4 pinned by digest
  `sha256:436eaf9778cad75507ff70ea26ace30dcbe15606e619ac3823495663d7f7c115`
  in `VERSIONS.lock.yaml` + `COMPONENT_REGISTRY.yaml`; sops 3.13.0 and
  age 1.1.1 pinned as local toolchain binaries (`/usr/local/bin/sops`,
  `/usr/bin/age`, `/usr/bin/age-keygen`). Evidence: real container
  probe + M2 gate. Alternatives: none (EP-008 precedent). Reversal: pin
  update with digest re-verification. Security: no credentials stored.
  License: MPL-2.0 (OpenBao), MPL-2.0 (sops), Apache-2.0 (age).
  Compatibility: none.
- 2026-08-14 | AppRole least-privilege authentication | OpenBao
  adapter authenticates via AppRole with per-tenant policies and a
  bounded 15-minute token TTL; renewal is explicit only. No root token,
  no wildcard super-policy. Evidence: live probe -- tenant A allowed;
  tenant B, sys/auth, policy-creation all denied (403); token TTL
  bounded at 900s; wrong SecretID rejected (400). Alternatives: root
  token (rejected: unbounded authority). Reversal: policy edit.
  Security: client token never Debug-printed (redacted), never logged.
  License: none. Compatibility: none.
- 2026-08-14 | KV-v2 lifecycle mapping | Adapter maps the trust
  contract onto OpenBao KV-v2 at `secret/`: typed create/read/update/
  metadata/soft-delete/undelete/destroy; DELETE returns empty body;
  soft-delete keeps versions until destroy. Evidence: live probe
  (integration suite `ep009_integration_openbao_*`, 19 tests).
  Alternatives: KV-v1 (no versioning). Reversal: none. Security:
  `SecretValue` redacted; references carry fingerprints only.
  License: none. Compatibility: none.
- 2026-08-14 | Response wrapping one-time semantics | Secret handoff
  uses OpenBao response wrapping: a wrapped READ carries no plaintext;
  unwrap #1 returns the secret; unwrap #2 -> 400 (`wrapping token is
  not valid or does not exist`); expired wrap -> 400. Wrapping token
  never logged (redacted `Debug`). Evidence: live probe + failure
  suite (13 tests). Alternatives: plaintext handoff (rejected:
  reusable bearer). Reversal: none. Security: token single-use,
  TTL-bounded, never serialized. License: none. Compatibility: none.
- 2026-08-14 | OpenBao vs SOPS+age boundary | OpenBao is the online
  authority for runtime secrets. SOPS+age is bootstrap/offline/
  break-glass ONLY, reachable exclusively through explicit
  `BootstrapSecretStore` operations; it is NEVER a silent runtime
  fallback when OpenBao is unavailable (directive N). Evidence:
  routing rule in `infra/openbao/src/sops.rs`; unit + integration
  tests. Alternatives: automatic fallback (rejected: would continue
  as if authorization existed). Reversal: contract change. Security:
  fail closed. License: none. Compatibility: none.
- 2026-08-14 | Age private identity never in repository | The age
  PRIVATE identity is generated ephemerally outside the repo, held
  in memory (zeroed on drop), written only to a 0600 temp file that
  is removed immediately; never adjacent to ciphertext; never
  committed. A stray untracked `secrets/age-key.txt` was removed;
  repo scan green (detector + test files allowed). Evidence:
  `ep009_integration_sops_no_private_identity_in_repository` +
  security-check. Alternatives: none. Reversal: none. Security:
  hard invariant (directive M). License: none. Compatibility: none.
- 2026-08-14 | SOPS wrong-valid-identity -> ProviderAuthorization |
  Real sops 3.13.0 with a valid-but-wrong age identity exits 128 and
  emits `Failed to get the data key required to decrypt the SOPS
  file.` / `age: no identity matched any of the recipients.` The
  previous naive matcher (`stderr.contains("failed to decrypt")`)
  missed this shape and returned MalformedProviderResponse; it also
  misclassified corrupted documents (whose stderr contains `failed to
  decrypt and authenticate payload chunk`) as ProviderAuthorization.
  Fixed with an ORDERED classifier `classify_sops_decrypt_failure`
  that rules out structural/source failures first, then maps
  valid-identity-but-no-key to ProviderAuthorization. Evidence:
  `ep009_unit_sops_classifier_*` (9 tests) + live proof.
  Alternatives: broadening the match to any `identity` substring
  (rejected: directive B -- too broad). Reversal: none. Security:
  typed fail-closed distinction preserved. License: none.
  Compatibility: none.
- 2026-08-14 | SOPS classifier real failure shapes (captured) |
  Exact non-secret stderr shapes from pinned sops 3.13.0 / age 1.1.1:
  missing sealed document -> exit 100 `cannot operate on non-existent
  file` (NotFound); missing SOPS_AGE_KEY_FILE -> exit 128 `failed to
  open SOPS_AGE_KEY_FILE file: open <path>: no such file or
  directory` (NotFound); malformed identity -> exit 128 `failed to
  parse 'SOPS_AGE_KEY_FILE' age identities: unknown identity type`
  (MalformedProviderResponse); corrupted document -> exit 128 `failed
  to decrypt and authenticate payload chunk, file may be corrupted or
  tampered with` (MalformedProviderResponse); valid-but-wrong
  identity -> exit 128 `age: no identity matched any of the
  recipients` (ProviderAuthorization). The generic footer `Recovery
  failed because no master key was able to decrypt the file...`
  appears in EVERY failure and never drives classification. Evidence:
  capture scripts at `/tmp/nexus-sops-shapes.py` (not committed) +
  classifier unit tests. Reversal: none. Security: no secrets in
  captured shapes; identity material redacted.
- 2026-08-14 | Malformed identity material remains distinct | A
  syntactically invalid age identity (unknown identity type) maps to
  MalformedProviderResponse, never ProviderAuthorization, even when
  the same generic recovery footer is present (directive E.7
  regression test). Corrupted SOPS data remains MalformedProviderResponse
  (integrity), distinct from authorization. Evidence:
  `ep009_unit_sops_classifier_malformed_plus_same_footer_is_malformed`,
  `ep009_unit_sops_classifier_corrupted_document_is_malformed`.
  Reversal: none. Security: fail-closed typing preserved.
- 2026-08-14 | Secret/canary log-redaction proof | Telemetry events
  carry fingerprints only; `SecretValue`, wrapping tokens, client
  tokens, and age identities redact in `Debug`; canary values never
  appear in logs or evidence. Evidence:
  `ep009_unit_telemetry_never_contains_secrets`,
  `ep009_unit_secret_value_redaction_invariant`,
  `ep009_unit_wrapped_handoff_debug_redacts_wrapping_token`,
  canary scan in integration suite. Reversal: none. Security: core
  invariant. License: none. Compatibility: none.
- 2026-08-14 | Teardown/orphan behavior | The M2 gate runs
  `scripts/ep009-orphan-audit.sh` (containers, networks, volumes,
  temp identities, leftover processes) and requires its ok sentinel;
  temp fixture dirs are removed after each suite; the live proof
  removes its ephemeral dir. Evidence: `EP-009 orphan audit: ok` in
  gate output. Alternatives: leaving containers (rejected: explicit
  cleanup doctrine). Reversal: none. Security: no leftover identity
  material. License: none. Compatibility: none.
- 2026-08-14 | Headscale image pin (M3) | `headscale/headscale:0.23.0`
  pinned by digest
  `sha256:ffe793968ef6fbec78a8d095893fe03112e6a74231afe366eb504fbc822afea6`
  in `VERSIONS.lock.yaml` (replacing a speculative 0.28.0 blueprint
  pin); the pinned CLI binary is extracted from that exact image
  (`/ko-app/headscale`) to `/usr/local/bin/headscale` and version
  checked (v0.23.0). Evidence: M3 gate + live proof. Alternatives:
  0.28.0 (unproven). Reversal: pin update with digest re-verification.
  Security: API key in-memory only, never logged. License:
  BSD-3-Clause. Compatibility: none.
- 2026-08-14 | Headscale transport (M3) | The adapter drives the REAL
  headscale CLI binary over REAL gRPC (TLS + API key) to the REAL
  server -- the same transport the provider's own admin tooling uses.
  The CLI requires a config file; `HEADSCALE_CONFIG` env makes the
  adapter self-contained. Remote CLI always uses TLS; `cli.insecure`
  enables InsecureSkipVerify (test/self-host path). Evidence:
  `ep009_integration_headscale_wrong_api_key_fails_closed` (auth
  error) + `..._dead_server_fails_closed` (Unavailable, never
  succeeds). Alternatives: direct gRPC client in-process (heavier,
  duplicates provider tooling). Reversal: none. Security: API key
  redacted in Debug; keys never in evidence. License: none.
  Compatibility: none.
- 2026-08-14 | MeshController mapping (M3) | Contract port maps onto
  real headscale ops: `register_node` = ensure user (idempotent) +
  `debug create-node` (fresh mkey) + `nodes register` (real IP
  allocation from 100.64.0.0/10 + fd7a:...::/48); `list_nodes` =
  `nodes list -u`; `wireguard_config` = node addresses + other
  registered nodes as peers (private key reference points into the
  OpenBao store); `node_state(Revoked)` = `nodes expire`; `revoke_node`
  = expire + delete (terminal). Evidence: live proof (2 registered,
  wg_peers derived, 1 revoked) + integration suite. Alternatives:
  none. Reversal: none. Security: node keys never logged. License:
  none. Compatibility: `TrustZone::PrivateMesh` used (not `Mesh`).
- 2026-08-14 | Orphan audit extension (M3) | `scripts/ep009-orphan-audit.sh`
  now also scans headscale temp dirs and `headscale serve` processes;
  the headscale integration suite removes its container + network
  explicitly and asserts zero leftovers. Evidence:
  `ep009_integration_headscale_teardown_leaves_no_orphans` +
  `EP-009 orphan audit: ok`. Reversal: none. Security: no leftover
  certs/keys/data. License: none. Compatibility: none.
- 2026-08-14 | PKI responsibility split (M4) | OpenBao PKI engine is
  the real CA (issuance, policy, serial, revocation, CRL); nexus-trust
  holds provider-neutral contracts; `infra/pki` is the concrete
  adapter (HTTP client mirroring the openbao adapter pattern, CSR/leaf
  handling, rustls mTLS helpers, revocation verifier). Headscale stays
  mesh-only: mesh membership is never service identity. Evidence:
  `infra/pki/` crate + `pki_live_proof` example. Alternatives:
  in-process CA (rejected: custom cryptography / no custody story).
  Reversal: none. Security: CA key never leaves OpenBao; leaf keys
  generated locally, only CSR crosses the CA boundary. License: none.
  Compatibility: none.
- 2026-08-14 | Canonical service identity SAN namespace (M4) | ADR-014
  locks the URI SAN form `nexus://tenant/<tenant>/service/<identity>`
  plus a deterministic transport DNS SAN for standard rustls hostname
  verification. CN is set to the transport DNS name only to satisfy
  OpenBao `enforce_hostnames`. Logical identity != certificate
  instance (rotation keeps the same URI SAN, new serial/key).
  Evidence: `infra/pki/src/identity.rs` + ADR-014 +
  `ep009_unit_pki_identity_*` tests. Alternatives: CN-based identity
  (rejected: ambiguous). Reversal: ADR change. Security: identity
  binding is SAN-based, never CN alone. License: none. Compatibility:
  none.
- 2026-08-14 | OpenBao PKI wire facts (M4) | Internal EC root
  generated (`serial_number` + issuing CA confirmed); `enforce_hostnames:
  true` requires CN in `allowed_domains`; `ca/pem` returns RAW PEM not
  JSON; role `max_ttl` is int in JSON; serial formats differ between
  sign response (lowercase, colon-separated) and CRL (uppercase, no
  colons); revoke returns `revocation_time`; CRL DER via
  `Accept: application/pkix-crl`. Evidence: live probe + integration
  suite. Reversal: none. Security: no secrets captured. License: none.
  Compatibility: none.
- 2026-08-14 | rustls 0.23.43 mTLS + CRL (M4) | Real rustls server
  (requires client cert) + client (verifies chain/SAN/CRL);
  `RevocationVerifier` caches CRLs (30s TTL, fail-closed fetch);
  verifier built via `WebPkiServerVerifier::builder(...).with_crls(...)`
  installed through `.dangerous()` (standard 0.23 API; verifier stays
  strict, never permissive). Evidence: `ep009_unit_pki_revocation_verifier_*`
  + live proof negative matrix (missing client cert, wrong CA, wrong
  SAN, expired, not-yet-valid, wrong EKU, malformed all rejected).
  Alternatives: permissive verifier (rejected). Reversal: none.
  Security: strict chain + SAN + CRL verification. License: none.
  Compatibility: rustls pinned 0.23.43 (ring).
- 2026-08-14 | Capability boundary (M4) | Certificate identity does
  not grant authorization: the live proof asserts
  `CAPABILITY-BOUNDARY-PASS` (mTLS identity proven, capability token
  absent -> action denied). Application authority is never encoded in
  SANs. Evidence: `pki_live_proof` capability-boundary stage.
  Reversal: none. Security: hard boundary. License: none.
  Compatibility: none.
- 2026-08-14 | M3 debt: headscale crate license + path deps (M4) |
  `nexus-headscale` is Nexus-authored code, so its package license is
  the workspace license `Apache-2.0 OR MIT` (not MPL-2.0, which is the
  upstream Headscale component's license); wildcard path deps replaced
  with exact `version = "0.1.0"` path deps. Upstream component metadata
  (VERSIONS.lock: headscale BSD-3-Clause, deny.toml allowlist,
  COMPONENT_REGISTRY) was NOT relabeled. Evidence: `cargo deny check`
  -> `bans ok, licenses ok, sources ok`. Alternatives: relabeling
  upstream (rejected: dishonest). Reversal: none. License: workspace
  license on the adapter, upstream license preserved.
  Compatibility: none.
- 2026-08-14 | windows-sys duplicate skip (M4) | `cargo deny check`
  surfaced a two-version split for windows-sys (0.52.0 via ring
  0.17.14 pulled by rcgen/rustls/rustls-webpki in the nexus-pki +
  async-nats chains; 0.61.2 via mio/socket2/tokio). Windows-only
  target crate, never linked on Linux; cannot be unified without
  breaking pinned transitive pins. Added the documented targeted skip
  (same class as untrusted/getrandom/thiserror skips) and amended the
  EP-009 fence to include deny.toml (EP-005/EP-007/EP-008 precedent).
  Evidence: `dependency audit: ok`. Alternatives: unify versions
  (impossible without breaking pins); ignore gate (rejected).
  Reversal: skip removal when pins unify. License: none.
  Compatibility: none.
- 2026-08-14 | CapabilityTokenIssuer concrete ownership (M5) | Graph
  inspection confirmed no later ExecPlan owns the concrete issuer;
  M5 implements and proves it now. Implementation:
  `infra/openbao/src/token.rs` -- REAL OpenBao Transit Ed25519
  sign/verify (approved provider crypto; no custom primitives).
  Binding: subject/audience/tenant/action scoped, issued-at +
  expiration bounded, unique token id (fingerprint+timestamp), record
  in KV-v2 for revocation, Debug/telemetry redacted. Not a universal
  bearer super-token. Evidence: 20 nexus-openbao unit tests + M5
  live-fire capability matrix. Reversal: none. Security: provider
  crypto + fail-closed verify. License: none. Compatibility: none.
- 2026-08-14 | SOPS bootstrap vs OpenBao runtime boundary (M5) |
  SOPS+age is bootstrap/offline/break-glass ONLY through
  `SopsBootstrapStore`; OpenBao is the online runtime authority.
  OpenBao unavailable -> typed fail-closed error, NEVER a silent SOPS
  fallback (the dead-store proof exercises this with a real
  unreachable address). Evidence: M5 live-fire `fail_closed` stage +
  M2 routing rule. Reversal: none. Security: fail closed. License:
  none. Compatibility: none.
- 2026-08-14 | Headscale control-plane certification (M5) | Real
  enrollment + addressing + peer config proven; kernel WireGuard
  dataplane explicitly NOT ASSERTED (control-plane proof only; the
  kernel dataplane is a separate certification). Evidence: M5
  live-fire mesh stages + evidence JSON dataplane marker. Reversal:
  none. Security: no overclaim. License: none. Compatibility: none.
- 2026-08-14 | Node-local leaf keys + CA custody (M5) | Leaf key +
  CSR generated at the node (rcgen); the CA receives the CSR only;
  OpenBao internal EC root never exports its private key. Evidence:
  M5 live-fire PKI stage + `assert "private_key" not in body`.
  Reversal: none. Security: CA key non-exportable. License: none.
  Compatibility: none.
- 2026-08-14 | Revocation/rotation/mesh boundaries (M5) | Cert
  revocation through CRL: mesh membership intact, mTLS FAILS
  (`Err(CertificateRevoked)`); rotation keeps the SAME logical
  ServiceIdentity with a new key+serial; headscale revocation is a
  separate membership control from PKI revocation. Permanent
  hierarchy: NETWORK REACHABILITY != CRYPTOGRAPHIC IDENTITY !=
  AUTHORIZATION. Evidence: M5 live-fire cert_revocation / cert_rotation
  / mesh_revocation stages. Reversal: none. Security: hard boundaries.
  License: none. Compatibility: none.
- 2026-08-14 | Response wrapping one-time (M5) | Real OpenBao
  response wrapping: secret response -> wrapping token -> one unwrap
  succeeds -> second unwrap rejected; wrapping token never logged or
  persisted. Evidence: M5 live-fire `response_wrapping` stage.
  Reversal: none. Security: one-time handoff. License: none.
  Compatibility: none.
- 2026-08-14 | Teardown + private-material hygiene (M5) | Every M5
  resource (OpenBao/Headscale containers, networks, temp dirs, age
  identities, SOPS fixtures, CA PEM, API keys, leaf keys, capability
  token files) is removed at teardown; `scripts/ep009-orphan-audit.sh`
  extended with the M5 temp pattern; evidence carries fingerprints +
  serials only, never secret material. Evidence: `EP-009 orphan audit:
  ok` + M5 hygiene tests. Reversal: none. Security: zero residue.
  License: none. Compatibility: none.

# 14. Outcomes & Retrospective

CHANGED (within the EP-009 machine fence; see
`.agent/expected-files/EP-009.txt`):
- `infra/openbao/src/token.rs` (NEW): Transit-backed
  CapabilityTokenIssuer; `ensure_key` idempotency fix (already-mounted
  400 tolerated, real denial fails closed)
- `infra/openbao/src/store.rs`: wrapped-response plaintext detection
  treats `"data": null` as wrapped (M5 live-fire find)
- `infra/openbao/src/lib_tests.rs`: +1 regression test (20 total)
- `infra/openbao/examples/trust_chain_live_proof.rs` (NEW): composed
  proof; JSON payloads; numeric headscale id resolution; revoked-cert
  Err accepted as expected failure
- `tests/trust/test_ep009_live_fire.py` (NEW): 6 ep009_livefire_* tests
- `scripts/nodes/EP-009.sh`: M5|verify now runs the real live-fire
  driver (vacuity-protected: `EP-009 M5: ok` only after real proof)
- `scripts/ep009-orphan-audit.sh`: +M5 temp pattern
- `config/sops/nexus-bootstrap.enc.yaml` (NEW): encrypted bootstrap
  artifact (public recipients + ciphertext only, no private identity)
- `.prettierignore`: `config/sops/*.enc.yaml` (ciphertext never
  reformatted)
- `.agent/expected-files/EP-009.txt`: +`.prettierignore`
- `Cargo.lock`, `infra/openbao/Cargo.toml` (dev-deps), `infra/openbao/
  src/lib.rs` (module registration)

COMMANDS AND SENTINELS (all run now):
- `sh scripts/nodes/EP-009.sh M5` -> `EP-009 M5: ok`
- `sh scripts/node-verify.sh EP-009` -> `node verify EP-009: ok`
- `sh scripts/scope-audit.sh EP-009` -> `scope audit EP-009: ok`
- `sh scripts/expected-files.sh EP-009` -> `expected files EP-009: ok`
- adapter parity -> `ok` (single unique PRIME-BLOCK cksum)
- `security check: ok`, `license gate: ok`, `reality gate: ok`,
  `format check: ok`, `lint: ok`, `dependency audit: ok`,
  `EP-009 orphan audit: ok`, `verify: ok` (incl. live-fire: ok)

EVIDENCE: `.agent/state/evidence/ep009-m5/ep009-m5-trust-chain.json`
(correlation `nexus-ep009-m5-trust-chain-20260814`; stages all PASS;
kernel WireGuard dataplane NOT ASSERTED; distinct serial fingerprints).

PROVIDER/HARDWARE STATUS: OpenBao 2.5.4 + Headscale 0.23.0 + sops
3.13.0 + age 1.1.1 proven live; kernel WireGuard dataplane NOT
ASSERTED (separate certification).

REMAINING RISKS: kernel WireGuard dataplane not certified (by design);
`NEXUS_BOOTSTRAP_AGE_KEY_FILE` is a local gitignored bootstrap key
(regenerated after M2 cleanup, never committed).

GREEN TAG: `green/EP-009` created at the M5 verified commit.
