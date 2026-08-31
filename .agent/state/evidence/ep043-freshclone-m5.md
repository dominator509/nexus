# FRESH-CLONE ACCEPTANCE EVIDENCE

Run: ep043-freshclone-1788141155
Git commit: ca4eb2a9759bccba1e0c8788f86e153ef2ab1af4
Generated: 2026-08-31T01:57:16Z

Checkout: git clone --depth 1 file:///root/nexus (HEAD == ca4eb2a9759bccba1e0c8788f86e153ef2ab1af4)
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
