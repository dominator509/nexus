# ROLLBACK DRILL EVIDENCE

Run: ep043-rollback-drill-1788197484
Git commit: c073d0f42809b9124df07dbef709d0e6b7a01d97
Generated: 2026-08-31T17:31:25Z

State A captured: report b206dd5da12a41d2dacd3667f2546ad304565a02a02860e20744ecb9bbc2035a (committed bytes)
State A manifest component digests:
  nexus-core=sha256:8f0890cc919c925cb12ab3d514dcc237a417a25a04bb33f79728cc8e2ed3f26a
  nexus-model=sha256:cceba172627079d46c81c6de0acf30ec1ade6b1878ff2cf292a432c1c612cc9c
State B applied: forged READY report + corrupted manifest (isolated)
Rollback executed: git restore committed report; manifest regenerated
Rollback verified: report b206dd5da12a41d2dacd3667f2546ad304565a02a02860e20744ecb9bbc2035a (exact A restored); component
  digests match state A; verify-manifest ok

Redaction: no secret-shaped content
