# PROJECT BRIEF - NEXUS

## Problem

People and businesses are surrounded by disconnected smart-home platforms, AI agents, communications tools, CRMs, security products, cloud providers, and automation systems. Each maintains separate state, identity, permissions, memories, and user interfaces. Current assistants are usually model-centric chat surfaces rather than governed operating systems.

## Product

Nexus is one logical intelligence and control plane spanning a distributed fleet. The VPS or cloud node hosts durable identity, policy, memory, events, workflows, model and agent routing, business context, and remote access. Home-edge and device nodes execute low-latency local work, protect privacy, and continue safe functions during internet outages. The proprietary product is the orchestration and experience layer that turns mature open-source components into one coherent system.

## Target users

Primary: technically ambitious individuals, households, founders, multi-business owners, professionals, and small teams seeking one private control plane for home and work. Secondary: managed-service installers, privacy-conscious families, SMB operators, consultants, developers, and eventually enterprise or regulated deployments using stricter profiles.

## Core outcomes

The binding ship outcomes are the named proofs in LIVE_FIRE_PROOFS.md. Every outcome has an owning ExecPlan, a real entry point, observable effects, cleanup, and final ship-gate execution.

## Business goals

- Eliminate avoidable recurring SaaS and API fees.
- Make powerful self-hosting approachable to non-specialists through one wizard.
- Preserve the ability to offer managed SaaS without degrading self-hosted users.
- Minimize bespoke development by adapting hardened open-source engines.
- Keep every engine replaceable behind a Nexus contract.
- Create commercially defensible value in orchestration, memory, policy, UX, lifecycle automation, and integration.

## Technical goals

- Rust-first secure control plane.
- One canonical world model and governed memory fabric.
- Deterministic authority with model-selected intelligence.
- Durable event and workflow execution.
- Fast local paths and graceful offline operation.
- Universal connectors, sidecars, SDKs, skills, and agent adapters.
- Signed supply chain and one-package deployment.

## Out of scope

Training a foundation model from scratch; granting models unrestricted physical or financial authority; replacing mature open-source projects merely to own more code; requiring Kubernetes for a household; requiring Nexus-operated cloud services for self-hosted installations; bypassing vendor authentication, DRM, platform terms, or device secure boot; promising universal Roku local streaming before verified; shipping noncommercial wake-word model weights; certifying every optional provider without real live-fire evidence; building a robotic body in V1.

## Success metrics

- Every required live-fire proof passes in a fresh release candidate environment.
- Cacheable DeepSeek reflex traffic sustains at least 0.97 prompt-token cache hit ratio.
- No high-risk action executes from voice recognition or model confidence alone.
- A new Tier 1 connector can be implemented from the SDK and pass conformance without editing Nexus Core.
- A user can deploy the reference profile without editing YAML or shell scripts.
- Backup restore to a fresh host completes within the documented recovery objective and passes smoke.
- The product operates in core local mode if Nexus-operated infrastructure is unavailable.

Production readiness is defined only by PRODUCTION_READINESS.md and EP-043.
