# FRESH-CLONE ACCEPTANCE EVIDENCE

Run: ep043-freshclone-1788198684
Git commit: 8276f12628756d1a98f25cd3cbc53f0ce62b5b4e
Generated: 2026-08-31T17:56:21Z

Checkout: git clone --depth 1 file:///root/nexus (HEAD == 8276f12628756d1a98f25cd3cbc53f0ce62b5b4e)
Tree at checkout: clean
Dependency restore: pnpm install --frozen-lockfile (prefer-offline)
EP-043 gates in clone: ep043-m1-tests.sh ok, ep043-m2-tests.sh ok,
  ep043-m3-tests.sh ok, ep043-m4-tests.sh ok
Readiness CLI in clone: ok (honest NOT_READY report)
Manifest CLI in clone: ok
Verify-manifest CLI in clone: ok
Source-tree leakage: none (no development-tree path in acceptance logs)
Hidden local state: none (clone is self-contained; pnpm store is a
  global package cache, not working-tree state)

Redaction: no secret-shaped content
