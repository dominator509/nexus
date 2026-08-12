# LIVE-FIRE PROOFS

| ID | Name | Owner node | Proof |
| --- | --- | --- | --- |
| LF-001 | one-package-deployment | EP-035 | Deploy Nexus Core and a home edge from Nexus Setup using the local provider profile; assert owner login, health, private mesh, and fleet registration. |
| LF-002 | restore-existing-nexus | EP-037 | Restore encrypted state onto a fresh deployment and prove identities, policies, memories, skills, and connectors reattach. |
| LF-003 | owner-passkey-onboarding | EP-007 | Create an owner, enroll a passkey and recovery material, sign in, revoke the session, and verify audit records. |
| LF-004 | multi-user-identity | EP-034 | Enroll two adults and one restricted user; prove separate context, permissions, preferences, and mobile devices. |
| LF-005 | cross-device-continuity | EP-033 | Start an objective by voice, continue in the web dashboard, approve on mobile, and receive the final artifact in the same task graph. |
| LF-006 | deterministic-home-control | EP-020 | Issue a known low-risk command; prove no model call occurred, Home Assistant changed state, Nexus verified state, and an audit event exists. |
| LF-007 | conditional-home-workflow | EP-020 | Create a time and occupancy conditional command; prove Temporal persistence and correct execution or cancellation. |
| LF-008 | visitor-response | EP-023 | Receive a camera person event, identify known or unknown, notify the right user, and play an approved response through two-way audio where certified. |
| LF-009 | sentinel-quarantine | EP-031 | Detect a synthetic but real network-lab scan from an unknown device, correlate telemetry, request or apply policy-authorized quarantine, and verify isolation. |
| LF-010 | network-diagnosis | EP-030 | Diagnose a controlled DNS or Wi-Fi fault from OPNsense or OpenWrt and AdGuard telemetry, explain evidence, and propose a reversible fix. |
| LF-011 | email-lifecycle | EP-026 | Receive, search, summarize, draft, approve, send, and verify a real message through a certified mail provider. |
| LF-012 | governed-phone-call | EP-025 | Place a real test call through Asterisk and a certified SIP provider, exchange speech with STT and TTS, honor disclosure, and store the governed transcript. |
| LF-013 | fax-lifecycle | EP-027 | Send a real test fax through the certified profile, receive status callbacks, route inbound fax, and archive the artifact. |
| LF-014 | social-campaign | EP-029 | Create platform-native variants, obtain approval, publish through a certified account, ingest engagement, and report attribution. |
| LF-015 | hydra-cross-crm-command | EP-028 | Ask for hot leads across businesses, receive canonical Hydra context, propose a governed update, execute it, and consume the resulting Hydra event. |
| LF-016 | coding-agent-cowork | EP-017 | Assign implementation to Codex, independent review to Claude Code, return an issue for correction, run tests, and produce a human-approved pull request artifact. |
| LF-017 | durable-human-approval | EP-006 | Start a workflow, restart the worker while waiting, approve later from mobile, and prove exactly-once continuation. |
| LF-018 | skill-install-and-run | EP-018 | Inspect, scan, approve, sign, install, discover, execute, and roll back a skill without granting undeclared capabilities. |
| LF-019 | self-healing-fix-loop | EP-019 | Trigger a controlled defect, detect it through telemetry, reproduce, patch, test, review, request approval, canary, verify, and close or roll back. |
| LF-020 | storage-backend-portability | EP-037 | Write versioned artifacts, migrate between local and one S3-compatible backend, verify hashes and metadata, and remove the old copy only after approval. |
| LF-021 | model-provider-failover | EP-015 | Return a valid NexusControlObject through DeepSeek, disable the primary provider, fail over to a configured secondary, and preserve schemas, budgets, and trace IDs. |
| LF-022 | mobile-step-up | EP-034 | Request a high-risk action by voice, refuse voice-only authorization, approve with mobile biometric and passkey, execute, and verify. |
| LF-023 | legacy-sidecar-connector | EP-011 | Wrap a real local legacy protocol fixture outside production paths, discover capabilities, read state, issue an idempotent write, and receive a change event. |
| LF-024 | offline-degraded-operation | EP-020 | Disconnect cloud AI and public internet while retaining local identity cache, low-risk home control, alerts, and queued synchronization. |
| LF-025 | ceo-business-brief | EP-028 | Combine Hydra, social, communications, and finance connector data into a permission-filtered executive brief with source provenance. |
| LF-026 | voice-endpoint-transfer | EP-022 | Start a conversation on a room satellite, move it to a Bluetooth headset or mobile endpoint, and maintain user, task, and privacy context. |
| LF-027 | social-lead-to-crm | EP-029 | Classify a real certified social inquiry, create or link the canonical Hydra person and lead, draft a response, and record attribution. |
| LF-028 | shared-room-private-response | EP-021 | Ask for sensitive personal information in an occupied room and prove Nexus routes the response privately rather than speaking it aloud. |

## Stage-aware rule

A proof becomes mandatory when its owning node is DONE. `scripts/live-fire.sh` derives the current stage from the append-only ledger and runs every active proof. EP-043 sets `NEXUS_REQUIRE_ALL_PROOFS=1`, making all twenty-eight proofs mandatory. This stage-aware law prevents an early foundation node from falsely claiming that later hardware or provider features already work while retaining the master prompt's real-dependency standard at the final ship gate.

## Provider certification

The core product may ship with an optional provider disabled. It may not display that provider as certified until its real provider proof is recorded in `provider-certification/RESULTS.md` with account, version, date, redacted evidence hash, and release compatibility range.
