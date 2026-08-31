# ROLLBACK DRILL EVIDENCE

Run: ep043-rollback-drill-1788198682
Git commit: 8276f12628756d1a98f25cd3cbc53f0ce62b5b4e
Generated: 2026-08-31T17:51:23Z

State A captured: report 7fed5b59f63ef7c9e4f883697565d7d3475a08364c461b9eeacd1cc4d51263aa (committed bytes)
State A manifest component digests:
  nexus-core=sha256:8f0890cc919c925cb12ab3d514dcc237a417a25a04bb33f79728cc8e2ed3f26a
  nexus-model=sha256:cceba172627079d46c81c6de0acf30ec1ade6b1878ff2cf292a432c1c612cc9c
State B applied: forged READY report + corrupted manifest (isolated)
Rollback executed: git restore committed report; manifest regenerated
Rollback verified: report 7fed5b59f63ef7c9e4f883697565d7d3475a08364c461b9eeacd1cc4d51263aa (exact A restored); component
  digests match state A; verify-manifest ok

Redaction: no secret-shaped content
