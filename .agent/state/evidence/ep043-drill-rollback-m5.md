# ROLLBACK DRILL EVIDENCE

Run: ep043-rollback-drill-1788008862
Git commit: 15194acd35d245b2dfdbbd6865185faed0a5b030
Generated: 2026-08-29T13:07:43Z

State A captured: report dce0c1db81237481828a2b526174c20dbebe1eeba6f32e789d3dca6e5e0ba616 (committed bytes)
State A manifest component digests:
  nexus-core=sha256:8f0890cc919c925cb12ab3d514dcc237a417a25a04bb33f79728cc8e2ed3f26a
  nexus-model=sha256:cceba172627079d46c81c6de0acf30ec1ade6b1878ff2cf292a432c1c612cc9c
State B applied: forged READY report + corrupted manifest (isolated)
Rollback executed: git restore committed report; manifest regenerated
Rollback verified: report dce0c1db81237481828a2b526174c20dbebe1eeba6f32e789d3dca6e5e0ba616 (exact A restored); component
  digests match state A; verify-manifest ok

Redaction: no secret-shaped content
