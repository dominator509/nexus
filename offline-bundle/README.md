# @nexus/offline-bundle - EP-042 M5

Real offline bundle production, digest-bound verification, offline
installation, and rollback drill for Nexus releases (SPEC-016 behavior 5,
SPEC-024).

## Component record

- Path: `offline-bundle/`
- Package: `@nexus/offline-bundle` (private workspace member)
- Runtime: Node 24 native TypeScript (type-stripping +
  `--experimental-transform-types`) with a resolution-only ESM loader
  (`scripts/ts-resolve-loader.mjs`) so the CLI executes the REAL canonical
  `@nexus/setup` + `@nexus/installers` code with zero bundler.
- Dependencies: `@nexus/setup` (workspace:_), `@nexus/installers`
  (workspace:_). No third-party runtime dependency.
- Replaces/extends: nothing. This is the offline distribution boundary;
  canonical release/update/install truth stays in M1 (`crates/nexus-release`),
  M2 (`apps/setup/src/update/`), M3 (`infra/release/`), M4 (`installers/`).
- Behavior:
  - `produce` - build a real bundle from real files; every payload copied
    with real bytes, every digest is real sha256; release manifest must be
    digest-bound before it may enter a bundle.
  - `verify` - digest-bound: missing/changed/malformed/duplicate/path-
    traversal/symlink/wrong-release denied; manifest binding + bundle
    self-digest must hold.
  - `install` - OFFLINE install composing the M4 transactional installer;
    artifact bytes come from the local bundle only (no transport).
  - `rollback-drill` - restore exact prior bytes + verify before writing
    the receipt.
  - `evidence` - current-run redacted evidence, stale/tampered rejected.

## Command surface

```sh
sh offline-bundle/scripts/bundle-produce.sh ...
sh offline-bundle/scripts/bundle-verify.sh ...
sh offline-bundle/scripts/bundle-install.sh ...
sh offline-bundle/scripts/bundle-rollback.sh ...
```

See `OPERATIONS.md` for full operational instructions and the honest
certification boundary.
