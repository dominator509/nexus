# ROLLBACK DRILL EVIDENCE

Run: ep043-rollback-drill-1788208349
Git commit: d0f821275760f26adbd69083f5b5ce8754a65bb4
Generated: 2026-08-31T20:32:31Z

State A captured: report eb7e98e9ca625e3e6d4d29b3b921087812bac36e75de455c298d304220fd7c2a (committed bytes)
State A manifest component digests:
  nexus-container-seaweedfs=sha256:ffb4b1ee14edf638ece713978dbf56f7369bf22d4845248846653211113bbe0b
  nexus-providers-config=sha256:e8526bbf653c40adec0f61882a30e224c7e525a84e766f7d3488d2bd1cdbec4a
  nexus-router-policy=sha256:6581d8b9e3f736e90e237fad6f52e159351a6805b097598e5e738b5a6830cddb
  nexus-wake-manifest=sha256:95e9642c3050e27811a3a94310712e33e5d7b27ffbe2ce1fbca612028a1daf65
  nexus-wake-model=sha256:e715aca0bb51f9f26cb14d9dec2510d4843ef01d7a44a7bd908baa52315c2b08
State B applied: forged READY report + corrupted manifest (isolated)
Rollback executed: git restore committed report; manifest regenerated
Rollback verified: report eb7e98e9ca625e3e6d4d29b3b921087812bac36e75de455c298d304220fd7c2a (exact A restored); component
  digests match state A; verify-manifest ok

Redaction: no secret-shaped content
