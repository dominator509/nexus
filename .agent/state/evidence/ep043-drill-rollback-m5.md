# ROLLBACK DRILL EVIDENCE

Run: ep043-rollback-drill-1788141153
Git commit: ca4eb2a9759bccba1e0c8788f86e153ef2ab1af4
Generated: 2026-08-31T01:52:35Z

State A captured: report a709eca241f610adc88fcdf0b4d5c092ef627287389bac86aaaa6a00d58ced0b (committed bytes)
State A manifest component digests:
  nexus-core=sha256:8f0890cc919c925cb12ab3d514dcc237a417a25a04bb33f79728cc8e2ed3f26a
  nexus-model=sha256:cceba172627079d46c81c6de0acf30ec1ade6b1878ff2cf292a432c1c612cc9c
State B applied: forged READY report + corrupted manifest (isolated)
Rollback executed: git restore committed report; manifest regenerated
Rollback verified: report a709eca241f610adc88fcdf0b4d5c092ef627287389bac86aaaa6a00d58ced0b (exact A restored); component
  digests match state A; verify-manifest ok

Redaction: no secret-shaped content
