# scripts/sbom/ - supply-chain / SBOM forced failures, abuse cases, and observability (EP-039 M4)

## Purpose

This directory owns the operational supply-chain observability surface and the
forced-failure / abuse-case proofs for EP-039. It is not decorative: every
script runs real checks against the REAL workspace Cargo.lock, the REAL cargo
registry cache, the checked-in `policies/licenses/` files, and the certified
M1/M2/M3 supply-chain machinery.

## Files

- `generate.sh` - real SBOM evidence generator. Computes current-run bindings
  (run_id, git_commit, lockfile fingerprint, policy fingerprint) and invokes
  the certified transport adapter
  (`policies/licenses/examples/sbom_generate.rs`) to evaluate the real
  inventory. Writes `evidence.json` + the `evidence.json.sha256` seal. Fails
  closed on missing/malformed Cargo.lock or a failed inventory evaluation.
- `verify.sh` - real SBOM evidence verifier. Recomputes every binding against
  the CURRENT repository state and rejects stale / empty / tampered /
  mismatched evidence with typed failure classes:
  `EMPTY_EVIDENCE`, `TAMPERED_EVIDENCE`, `MISMATCHED_RUN_ID`,
  `STALE_GIT_COMMIT`, `STALE_LOCKFILE`, `STALE_POLICY`, `STALE_EVIDENCE`,
  `REDACTION_FAILURE`. Writes `verification.json`.
- `observability.sh` - redacted operational evidence: run_id, git_commit,
  fingerprints, package/resolved/green/denied/unknown/missing counts, policy
  verdict, verification/provenance/advisory states, redaction result,
  failure class.
- `forced-failures.sh` - runs the `ep039_failure_*` Rust suite (26 proofs)
  plus shell-level evidence abuse checks (missing/malformed lockfile fail
  closed, tampered/stale/mismatched/empty evidence rejected, redaction
  proven).

## Honest verdicts

The evidence document distinguishes:

- GENERATED (written by generate.sh) != VERIFIED (proven by verify.sh)
- COMPLETE / POLICY_PASSED / LEGAL_APPROVED are NEVER asserted by these
  scripts. `policy_passed` is false and `policy_verdict` stays NON_GREEN
  while the real 16-denied-package finding (14 ids outside canonical tables
  - 2 license-less workspace manifests) remains.

## Certification boundary

- scripts/sbom/ BEHAVIOR CERTIFIED for the exact exercised local repository
  surface.
- forced-failure suite CERTIFIED for the exact abuse cases exercised.
- SBOM evidence/observability CERTIFIED for the exact generated/validated
  local evidence surface.

NOT ASSERTED: legal clearance, production artifact SBOM completeness,
container image provenance, SLSA/in-toto signing, external advisory feed
monitoring, GitHub dependency submission, remote synchronization.
