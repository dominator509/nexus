# ROLLBACK DRILL EVIDENCE

Run: ep043-rollback-drill-1787734418
Git commit: ddaa9c148f0abb5d3db93bbce02584f9a29e4ed7
Generated: 2026-08-26T08:53:39Z

State A captured: report 3eb0f35b061bdf936de66bf52d25cdf11e5abc75be058946e4bbb1182f4d2660 (committed bytes)
State A manifest component digests:
  nexus-core=sha256:8f0890cc919c925cb12ab3d514dcc237a417a25a04bb33f79728cc8e2ed3f26a
  nexus-model=sha256:cceba172627079d46c81c6de0acf30ec1ade6b1878ff2cf292a432c1c612cc9c
State B applied: forged READY report + corrupted manifest (isolated)
Rollback executed: git restore committed report; manifest regenerated
Rollback verified: report 3eb0f35b061bdf936de66bf52d25cdf11e5abc75be058946e4bbb1182f4d2660 (exact A restored); component
  digests match state A; verify-manifest ok

Redaction: no secret-shaped content
