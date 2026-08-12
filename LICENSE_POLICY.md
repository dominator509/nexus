# NEXUS LICENSE POLICY

## Goals

Nexus is commercially sellable as self-hosted software and managed SaaS. Open-source reuse is mandatory where it is technically and legally sound. License obligations are treated as architecture, not release paperwork.

## Classes

- GREEN: MIT, Apache-2.0, BSD, ISC, PostgreSQL, PSF, and equivalent permissive licenses. May be linked or embedded after security review.
- REVIEW: MPL-2.0 and LGPL. May be used when file-level or dynamic-link obligations are documented and the boundary remains replaceable.
- SIDECAR: GPL and AGPL components (copyleft). Run as independent processes or external appliances, communicate through documented APIs, preserve notices and source-offer duties, and obtain legal review before distribution.
- EXTERNAL: commercial API or user-owned appliance governed by provider terms.
- PROHIBITED: noncommercial code or model weights, unclear provenance, source-available licenses incompatible with the intended offering, and packages that cannot satisfy redistribution obligations.

## Model and data licenses

Code license, model-weight license, training-data license, voice license, and generated-output terms are evaluated separately. No model is admitted because its repository code is permissive while its weights or data are not.

## Automated gates

EP-039 adds SBOM generation, cargo-deny, cargo-audit, pnpm lock verification, Python OSV audit, container scanning, secret scanning, provenance attestations, signature verification, and `scripts/license-gate.sh`. A waiver requires an ADR naming the package, version, exact obligation, scope, owner, expiration, and replacement plan.
