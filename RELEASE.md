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
