# ADR-025 - Skill Registry Vocabulary and Authority Semantics

Status: Accepted
Date: 2026-08-15
Owner: EP-018 (Skill Registry and Skill Factory)

## Context

SPEC-010 defines Agent Skills as a canonical class: portable skills,
Skill Trust, and the Skill Factory. The canonical terms include Skill
Registry, Skill Package, Skill Manifest, Skill Signature, Skill Trust,
Skill Evaluator, Skill Composer, and Skill Proposal. None of these
vocabulary classes existed in `crates/nexus-domain` or a skills crate.
EP-018 owns the skill contracts and must encode several authority
distinctions the specs require: a skill can never grant itself tools or
secrets, community skills begin inspect-only or sandboxed, factory
output must pass evals and human promotion, and skills are immutable by
version.

## Decision

Add the EP-018-owned vocabulary in `crates/nexus-skills` (vocabulary
module), documented in `docs/vocabulary/README.md`, with unknown-value
rejection at parse time:

- `SkillTrustLevel`: `INSPECT_ONLY`, `SANDBOXED`, `TRUSTED`, `SYSTEM`.
  Trust tiers are earned through evals and human promotion.
- `SkillPermission`: `NONE`, `READ`, `WRITE`, `EXECUTE`, `NETWORK`,
  `SECRETS`. Declared REQUESTS, never grants.
- `SignatureAlgorithm`: `ED25519`, `ECDSA_P256`.
- `SkillProposalState`: `PROPOSED`, `EVAL_PENDING`, `EVAL_PASSED`,
  `EVAL_FAILED`, `AWAITING_PROMOTION`, `PROMOTED`, `REJECTED`,
  `ROLLED_BACK`. Canonical lifecycle, fail closed, no resurrection.
- `SkillId`: typed UUIDv7 identifier in `crates/nexus-domain`
  (Rust-only at M1; no generated wire binding ripple).

Authority semantics locked by this ADR:

1. **Package identity is immutable by version.** Canonical identity is
   `name@version:content_hash`. Same id + version + content always
   produce the same identity; changed content under the same id/version
   is a registration conflict, never a silent mutation. There is no
   mutable "latest" content under an immutable version.

2. **Integrity, trust, and authorization are distinct states.** A valid
   signature is an integrity/authenticity statement. It is NOT a trusted
   signer, NOT an authorized installation, and NOT an execution
   permission. The presence of a signature never sets `SkillTrustLevel`.

3. **Manifest permissions are requested requirements, not grants.** A
   manifest declares what a skill would like; authorization still
   requires the caller's grant, tenant policy, and the trust ceiling.
   Declared permissions never self-grant tools or secrets.

4. **Effective permission ceiling.** Effective authority is the
   INTERSECTION of the closure's declared requirements, the caller's
   explicit grants, the tenant policy allowance, and the trust-tier
   ceiling. Composition never manufactures authority the parent
   execution context did not already possess, and nested composition
   never exceeds the root authority envelope. A raw union of effective
   permissions is forbidden.

5. **Community skill sandbox rule.** Community/untrusted skills begin
   inspect-only or sandboxed and can never request privileged host
   authority beyond their ceiling. Higher trust levels permit broader
   eligibility but never bypass EP-008 authorization, caller grant,
   tenant policy, or a skill-specific permission grant.

6. **Signature semantics.** Signature validation at the M1 contract
   boundary is structural (algorithm, hex encoding, key/signature
   lengths). Cryptographic verification is owned by the M2/M3 behavior
   boundary and the real scan-before-install proof.

7. **Deterministic composition.** Dependency resolution is
   deterministic: the available pool is sorted by (name, version), the
   lowest available version is the canonical resolution, traversal is
   post-order (dependencies before dependents), and repeated input
   orders produce identical compositions.

8. **Dependency cycle rejection.** Direct self-cycles, direct cycles,
   and transitive cycles are rejected. Maximum composition depth is
   bounded (`MAX_COMPOSITION_DEPTH = 16`); deeper graphs fail closed so
   recursive skill loading cannot be infinite. Duplicate dependencies
   are rejected at manifest validation.

9. **SkillProposal lifecycle.** Canonical transitions only:
   `PROPOSED -> EVAL_PENDING -> EVAL_PASSED | EVAL_FAILED`,
   `EVAL_PASSED -> AWAITING_PROMOTION`,
   `AWAITING_PROMOTION -> REJECTED`, and `PROMOTED` reached only
   through human approval. Terminal states (`PROMOTED`, `REJECTED`,
   `EVAL_FAILED`, `ROLLED_BACK`) never move; resurrection is rejected.

10. **A model/agent may propose, never self-approve.** Proposals record
    their proposer; promotion requires a distinct, non-empty human
    approver. A model cannot self-approve installation.

11. **Network rules are requested constraints, not automatic network
    access.** `network_rules` declare what a skill would like; they do
    not open the network. The execution/sandbox policy enforces them.

## Alternatives considered

- Treating manifest permissions as effective authority (rejected:
  would let skills self-grant, violating SPEC-010 behavior 7).
- Allowing version mutation (rejected: breaks scan-before-install and
  rollback integrity; immutable-by-version is the node contract).
- Allowing any proposal transition (rejected: terminal resurrection
  would let a rejected skill silently re-enter the pipeline).

## Consequences

Contracts are fail closed and unambiguous: later implementation cannot
confuse skill existence, integrity, trust, authorization, execution, or
result verification. Certification of external/public skill registries
is not claimed by this node and remains DEFERRED to an explicit later
owner. The vocabulary README and the skills crate must stay in sync;
new public names require a new ADR and schema/vocabulary update.

## Reversal

Reversing requires a new ADR demonstrating that the authority
distinctions are preserved by an equivalent or stronger model.
