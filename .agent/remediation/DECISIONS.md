# Remediation Decision Log (generation 2)

## D-001 (RX-001) — chacha20 0.10.1 yanked: security gate denied; minimal bump

**Date:** 2026-08-29
**Discovery:** EP-043 M5 gate regression (required to demonstrate AUD-080
non-reproducibility) failed at `security-check.sh` → `cargo audit` with
`Crate: chacha20 Version: 0.10.1 Warning: yanked / error: 1 denied warning found!`.
The RustSec advisory database moved after the audit baseline; the locked
transitive dependency (via rand 0.10.2) is yanked on crates.io.

**Why kept inside RX-001's fence:** the M5 gate cannot reach the readiness
section (the AUD-080 proof) while the security gate denies a yanked crate.
Ignoring the denial or weakening the gate is forbidden by doctrine. The fix is
the minimal real remediation: `cargo update -p chacha20 --precise 0.10.2`
(Cargo.lock only; API-compatible patch bump; 0.10.2 is the current max stable).

**Evidence:** `crates.io/api/v1/crates/chacha20` lists 0.10.2 as max stable;
lockfile checksum changed d52445… → 65c35e…; gate re-run must now fail for the
AUD-080 readiness reason instead of the yanked-crate denial.

**Note:** this is a genuine post-audit supply-chain finding. The register is
frozen at exactly AUD-001…AUD-090, so it is recorded here and owned by the
RX-009 supply-chain scope (dependency/license gate truth) rather than as a new
register leaf.
