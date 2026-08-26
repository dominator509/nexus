# RELEASE

## Versioning

Semantic versioning for Nexus contracts and product releases. Component and provider compatibility is recorded independently. Schema and event breaking changes require new major contract versions and compatibility windows.

## Branches

Protected `main`, short-lived node branches or worktrees, release candidates tagged `vX.Y.Z-rc.N`, stable releases tagged `vX.Y.Z`, and graph tags `green/EP-XXX`. Forced pushes and history rewrites are forbidden.

## Release types

- Patch: security and compatible fixes.
- Minor: backward-compatible capabilities and providers.
- Major: intentional contract, migration, or compatibility break.
- Security hotfix: expedited patch with the same signing and rollback evidence.

## Checklist

All graph nodes done, verify, production readiness, live-fire, provider and hardware matrices, SBOM, license, signatures, provenance, migrations, backup, restore, update, rollback, accessibility, security, privacy, performance, observability, release copy, changelog, and known risks.

## Approvals

Production deployment is not authorized. A release may be built and signed after all gates. A human must execute the manual deployment command. Security, identity, permissions, destructive migration, legal, and R4 changes require named reviewers even before release.

## Post-release

Observe error, latency, action verification, event lag, workflow backlog, cache ratio, costs, provider health, edge fleet, backups, Sentinel, and user reports through the declared window. Roll back on threshold breach.

## Release procedure (EP-043 M4)

The release procedure is driven by the canonical production readiness engine in `release-evidence/`. All commands below are real and resolve to tracked repository paths; run them from the repository root.

1. Generate the production readiness report:

   ```sh
   node --experimental-transform-types --import file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs release-evidence/src/cli.ts readiness --output PRODUCTION_READINESS.md
   ```

   Exit 0 means the report was written. The report decision is `READY` only when every acceptance obligation is met; otherwise `NOT_READY` with the exact blocking reasons.

2. Build the release manifest with real sha256 digests:

   ```sh
   node --experimental-transform-types --import file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs release-evidence/src/cli.ts manifest --output-dir dist/release
   ```

3. Verify the manifest against the real artifact bytes:

   ```sh
   node --experimental-transform-types --import file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs release-evidence/src/cli.ts verify-manifest --manifest dist/release/RELEASE_MANIFEST.json
   ```

   This command fails closed on tamper (`VERIFICATION_FAILED`), missing artifacts (`NOT_FOUND`), malformed JSON or unknown fields (`VALIDATION_FAILED`).

4. Inspect the ship gate:

   ```sh
   node --experimental-transform-types --import file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs release-evidence/src/cli.ts ship-gate-status
   ```

5. Inspect certification rows:

   ```sh
   node --experimental-transform-types --import file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs release-evidence/src/cli.ts certification-rows
   ```

6. Run the required gates: `sh scripts/security-check.sh`, `sh scripts/license-gate.sh`, `sh scripts/reality-gate.sh`, and the canonical node verify `sh scripts/node-verify.sh EP-043`.

7. Refresh evidence and commit it bound to the exact verified commit. A report regenerated on a different commit is stale evidence and must be regenerated and recommitted.

## Release-blocking conditions

A release is BLOCKED and the readiness decision remains `NOT_READY` while any of these hold:

- any graph node is not DONE (including EP-043),
- any certification row is `RELEASE-BLOCKING-PENDING` or `PENDING` or `SIGNED` without verified evidence,
- the fresh-clone acceptance rerun has not passed,
- the release manifest is missing, malformed, tampered, or unverifiable,
- any artifact digest does not match the manifest,
- the signature state is `PRESENT_NOT_VERIFIED` and cryptographic verification is required,
- the ship gate reports `BLOCKED`,
- the runtime smoke is unavailable,
- a documented command resolves to a missing path,
- a blocked dependency, denied read, cancelled work, or partial side effect is observed in the operational path.

Editing `PRODUCTION_READINESS.md` by hand changes nothing. The readiness decision is computed from canonical repository state (GRAPH, LEDGER, live-fire registry, certification results, evidence directory), never from the rendered report. A forged `READY` document is rejected because the engine does not read it.

## Ship-gate semantics

The ship gate states are distinct and are never collapsed:

- `BLOCKED`: at least one release-blocking condition holds.
- `PASSED`: every acceptance obligation is met and no blocking condition holds.
- `AUTHORIZED`: a named human reviewer has approved the release after `PASSED`.
- `SIGNED`: release artifacts carry a verified cryptographic signature.

The engine reports `BLOCKED` today. Command existence, previous unrelated verify runs, or a manifest's presence do not imply `PASSED`. The gate is never inferred; it is computed from real state.

## Signing boundary

The current release evidence boundary is `SIGNATURE_PRESENT_NOT_VERIFIED`: the manifest records signature presence but no key store exists and no cryptographic verification has been performed. Do not claim "signed release verified" from presence alone. Cryptographic signing verification, signed certification rows, and a signed release are NOT ASSERTED until a real verifier proves valid signature, invalid signature, wrong key, tampered manifest, and missing signature cases.

## Emergency abort and rollback

Abort a release attempt at any step by stopping the procedure; a blocked step never proceeds to the next. Rollback triggers and the rollback procedure are owned by `ROLLBACK.md` and `OPERATIONS.md` (Rollback section). Restore and rollback drills require dated evidence before release per SPEC-008.

## Fresh-clone prerequisite

Final acceptance uses a fresh-clone-equivalent environment: dependencies restore from frozen files, setup and build commands run, the readiness CLI executes, and generated evidence is current with no hidden local state. The final fresh-clone acceptance rerun is owned by EP-043 M5 and remains PENDING until it passes.

## Artifact checklist

Before a release candidate: report generated from current canonical state, manifest built and verified against real bytes, signatures honestly labeled, security check green, license gate green, reality gate green, node verify green on the committed tree, evidence committed bound to the verified commit, certification rows inspected, ship gate inspected.

## Readiness observability

Every generated report records: `Run` (run_id with a monotonic timestamp), `Git commit` (exact HEAD of the run), `Generated` (timestamp), the decision, and every blocking reason with its code. These fields are the incident correlation surface: a report is traceable to the exact repository state that produced it. A report bound to a different commit or a hand-edited report is stale or forged evidence and is not trusted by the decision engine.

## Operations diagnostic and bounded recovery

The operations diagnostic for the readiness service is `ship-gate-status` (real repository state, verdict, exact blocking reasons) combined with `certification-rows` (real certification rows). Bounded recovery is:

- `readiness --output PRODUCTION_READINESS.md` regenerates evidence from current canonical state,
- `manifest --output-dir dist/release` rebuilds the manifest from real component bytes,
- `verify-manifest` re-verifies after any recovery step.

A blocked dependency, denied read, or malformed input is never silently skipped; the CLI exits nonzero with a structured redacted error (`code`, `class`, `message`, `redacted`).
