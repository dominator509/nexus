# ROLLBACK DRILL EVIDENCE

Run: ep043-rollback-drill-1788401369
Git commit: 667bb11b7dd6a3b4660ebb7e4260fee1c4e92a1f
Generated: 2026-09-03T02:09:31Z

State A captured: report 0136f0905308bfc9b9495dfe67541951c268de3fb9ec5a0b5b557713d297cd23 (committed bytes)
State A manifest component digests:
  nexus-container-seaweedfs=sha256:ffb4b1ee14edf638ece713978dbf56f7369bf22d4845248846653211113bbe0b
  nexus-providers-config=sha256:e8526bbf653c40adec0f61882a30e224c7e525a84e766f7d3488d2bd1cdbec4a
  nexus-router-policy=sha256:6581d8b9e3f736e90e237fad6f52e159351a6805b097598e5e738b5a6830cddb
  nexus-wake-manifest=sha256:95e9642c3050e27811a3a94310712e33e5d7b27ffbe2ce1fbca612028a1daf65
  nexus-wake-model=sha256:e715aca0bb51f9f26cb14d9dec2510d4843ef01d7a44a7bd908baa52315c2b08
State B applied: forged READY report + corrupted manifest (isolated)
Rollback executed: git restore committed report; manifest regenerated
Rollback verified: report 0136f0905308bfc9b9495dfe67541951c268de3fb9ec5a0b5b557713d297cd23 (exact A restored); component
  digests match state A; verify-manifest ok

Redaction: no secret-shaped content
