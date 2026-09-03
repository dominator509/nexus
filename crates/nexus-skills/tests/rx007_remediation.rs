//! RX-007 remediation battery: sandboxed skill execution truth
//! (AUD-011) and bounded, deadlock-free subprocess execution
//! (AUD-022).
//!
//! AUD-011 hostile regressions prove the SkillExecutor subprocess is a
//! REAL OS sandbox on Linux, not a convention:
//! - the payload runs as uid/gid 65534 (nobody) after a real
//!   setuid/setgid/setgroups drop;
//! - the host filesystem is read-only (a write to `/` fails) while a
//!   bounded tmpfs at `/tmp` is the only writable location;
//! - the network namespace is private: `/proc/net/dev` shows only
//!   loopback, never a host interface;
//! - seccomp is in filter mode (`Seccomp: 2`) and `NoNewPrivs: 1` in
//!   `/proc/self/status`.
//!
//! AUD-022 hostile regressions prove execution is bounded:
//! - a child that floods stderr while keeping stdout open completes
//!   (concurrent drain, no deadlock);
//! - a child that never exits is killed at the deadline and the
//!   result is observable as `timed_out` (never a fabricated success).
//!
//! Every link is REAL: a real signed bundle on disk, real ring
//! Ed25519 verification, the real SkillBundleLoader, the real registry,
//! and the real SkillExecutor subprocess boundary. The hostile
//! payloads are CONTROLLED_TEST_FIXTURE shell scripts.

use nexus_skills::manifest::SkillManifest;
use nexus_skills::vocabulary::SignatureAlgorithm;
use nexus_skills::{
    sha256_hex, sign_ed25519, SkillBundleLoader, SkillExecutor, SkillPackageErrorCode,
    SkillPermission, SkillSignature, SkillTrustLevel, TenantId,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/skills/fixtures")
        .canonicalize()
        .expect("canonical fixtures root")
}

fn tenant() -> TenantId {
    TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072").expect("valid tenant")
}

/// Write a REAL signed skill bundle whose payload is the supplied
/// hostile script. Returns the loader and the payload.
fn write_signed_bundle(tmp: &Path, name: &str, payload: &[u8]) -> SkillBundleLoader {
    let dir = tmp.join(name).join("1.0.0");
    std::fs::create_dir_all(&dir).expect("create bundle dir");
    let content_hash = sha256_hex(payload);
    let identity = format!("{name}@1.0.0:{content_hash}");
    let (public_hex, signature_hex) = sign_ed25519(identity.as_bytes()).expect("real sign");
    let manifest = SkillManifest {
        skill_id: nexus_skills::SkillId::new("0cb7d278-1ed7-7da3-867e-99cbef7f8f0c")
            .expect("valid skill id"),
        tenant_id: tenant(),
        name: name.into(),
        version: "1.0.0".into(),
        description: "rx007 hostile probe".into(),
        permissions: vec![SkillPermission::Read],
        dependencies: vec![],
        network_rules: vec![],
        license: "MIT".into(),
        provenance: nexus_skills::ArtifactId::new("567c3a2e-7be9-77c4-87f6-883ddcc7fd86")
            .expect("valid artifact id"),
        trust_level: SkillTrustLevel::Sandboxed,
        signature: SkillSignature {
            algorithm: SignatureAlgorithm::Ed25519,
            public_key_hex: public_hex.clone(),
            signature_hex: signature_hex.clone(),
            signer: Some("nexus-rx007".into()),
        },
    };
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");
    std::fs::write(dir.join("SKILL.md"), payload).expect("write payload");
    SkillBundleLoader::new(tmp)
}

fn exec_payload(
    payload: &[u8],
    input: &[u8],
    timeout: Duration,
) -> nexus_skills::SkillExecutionResult {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let unique = format!("{}-{seq}", std::process::id());
    let tmp = std::env::temp_dir().join(format!("rx007-skill-{unique}"));
    let _ = std::fs::remove_dir_all(&tmp);
    let loader = write_signed_bundle(&tmp, "nexus/rx007probe", payload);
    let bundle = loader
        .load("nexus/rx007probe", "1.0.0")
        .expect("real bundle loads");
    bundle.package.validate().expect("package valid");
    let executor = SkillExecutor::new(std::env::temp_dir().join(format!("rx007-scratch-{unique}")))
        .with_timeout(timeout);
    let result = executor
        .execute(&bundle.package, payload, input, &[SkillPermission::Read])
        .expect("execution returns observable result");
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

// ---------------------------------------------------------------------------
// AUD-011: real OS sandbox
// ---------------------------------------------------------------------------

/// The payload probes the sandbox from the inside: identity, seccomp,
/// no_new_privs, network namespace interfaces, and a host-write
/// attempt. Output is a single deterministic report.
const SANDBOX_PROBE: &[u8] = b"#!/usr/bin/env sh
set -eu
echo \"UID=$(id -u)\"
echo \"GID=$(id -g)\"
echo \"SECCOMP=$(grep '^Seccomp:' /proc/self/status | awk '{print $2}')\"
echo \"NONEWPRIVS=$(grep '^NoNewPrivs:' /proc/self/status | awk '{print $2}')\"
echo \"NETDEV:\"
cat /proc/net/dev
echo \"NETEND\"
if touch /rx007-host-write-probe 2>/dev/null; then
  echo \"HOST_WRITABLE\"
else
  echo \"HOST_READONLY\"
fi
if touch /tmp/rx007-tmp-write-probe 2>/dev/null; then
  echo \"TMP_WRITABLE\"
else
  echo \"TMP_READONLY\"
fi
exit 0
";

#[test]
fn rx007_sandbox_drops_privileges_to_nobody() {
    // AUD-011: the subprocess must run as uid/gid 65534 after a real
    // privilege drop, not as the host user.
    let r = exec_payload(SANDBOX_PROBE, b"", Duration::from_secs(10));
    assert_eq!(r.exit_code, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("UID=65534"), "got: {}", r.stdout);
    assert!(r.stdout.contains("GID=65534"), "got: {}", r.stdout);
}

#[test]
fn rx007_sandbox_host_filesystem_is_readonly() {
    // AUD-011: the host filesystem is read-only inside the sandbox; a
    // write to `/` must fail. The bounded tmpfs at `/tmp` is the only
    // writable location.
    let r = exec_payload(SANDBOX_PROBE, b"", Duration::from_secs(10));
    assert_eq!(r.exit_code, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("HOST_READONLY"), "got: {}", r.stdout);
    assert!(r.stdout.contains("TMP_WRITABLE"), "got: {}", r.stdout);
}

#[test]
fn rx007_sandbox_network_namespace_is_private() {
    // AUD-011: the network namespace is private; /proc/net/dev must
    // show only loopback and never a host interface (eth0/ens*/enp*).
    let r = exec_payload(SANDBOX_PROBE, b"", Duration::from_secs(10));
    assert_eq!(r.exit_code, 0, "stderr: {}", r.stderr);
    let net = r.stdout.split("NETDEV:").nth(1).unwrap_or("");
    let net = net.split("NETEND").next().unwrap_or("");
    assert!(net.contains("lo"), "loopback present: {net}");
    for host_iface in ["eth0", "ens", "enp", "wlan"] {
        assert!(!net.contains(host_iface), "host iface leaked: {net}");
    }
}

#[test]
fn rx007_sandbox_seccomp_filter_active() {
    // AUD-011: seccomp must be in filter mode (2) and no_new_privs
    // must be set (1) inside the sandbox.
    let r = exec_payload(SANDBOX_PROBE, b"", Duration::from_secs(10));
    assert_eq!(r.exit_code, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("SECCOMP=2"), "got: {}", r.stdout);
    assert!(r.stdout.contains("NONEWPRIVS=1"), "got: {}", r.stdout);
}

// ---------------------------------------------------------------------------
// AUD-022: bounded, deadlock-free execution
// ---------------------------------------------------------------------------

/// Floods stderr while keeping stdout open, then prints a final
/// marker and exits 0. ~360 KB of stderr fills the 64 KB pipe buffer
/// several times - enough to deadlock the pre-RX-007 drain order
/// (parent waits on stdout EOF while the child blocks on a full
/// stderr pipe) - but stays under the 1 MiB output cap so a correct
/// concurrent drain completes normally.
const STDERR_FLOOD: &[u8] = b"#!/usr/bin/env sh
set -eu
i=0
while [ $i -lt 20000 ]; do
  echo \"stderr-line-$i\" >&2
  i=$((i + 1))
done
echo \"stdout-done\"
exit 0
";

#[test]
fn rx007_stderr_flood_does_not_deadlock() {
    // AUD-022: concurrent drain means a child flooding stderr while
    // keeping stdout open completes normally instead of blocking the
    // parent forever.
    let r = exec_payload(STDERR_FLOOD, b"", Duration::from_secs(30));
    assert_eq!(
        r.exit_code,
        0,
        "stderr tail: {}",
        &r.stderr[r.stderr.len().saturating_sub(200)..]
    );
    assert!(r.stdout.contains("stdout-done"), "got: {}", r.stdout);
    assert!(!r.timed_out, "must complete without hitting the deadline");
}

/// A child that never exits on its own.
const HANG_FOREVER: &[u8] = b"#!/usr/bin/env sh
while true; do sleep 1; done
";

#[test]
fn rx007_hung_payload_is_killed_at_deadline() {
    // AUD-022: a payload that never exits is SIGKILLed at the
    // deadline and the result is observable as timed_out - never a
    // fabricated success.
    let r = exec_payload(HANG_FOREVER, b"", Duration::from_millis(500));
    assert!(r.timed_out, "expected timed_out, got exit={}", r.exit_code);
    assert_eq!(r.exit_code, -9, "SIGKILL exit code");
}

// ---------------------------------------------------------------------------
// Existing fail-closed preconditions must still hold
// ---------------------------------------------------------------------------

#[test]
fn rx007_tampered_payload_still_fails_verification() {
    // The sandbox must not weaken the existing cryptographic boundary:
    // a payload whose signature does not verify never executes.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("rx007-tamper-{}-{seq}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let loader = write_signed_bundle(&tmp, "nexus/rx007probe", SANDBOX_PROBE);
    let bundle = loader.load("nexus/rx007probe", "1.0.0").expect("loads");
    let mut tampered = bundle.package.clone();
    tampered.content_hash = "0".repeat(64);
    let executor = SkillExecutor::new(
        std::env::temp_dir().join(format!("rx007-tamper-scratch-{}-{seq}", std::process::id())),
    );
    let err = executor
        .execute(&tampered, SANDBOX_PROBE, b"", &[SkillPermission::Read])
        .expect_err("tampered execution denied");
    assert_eq!(err.code, SkillPackageErrorCode::Verification);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn rx007_ungranted_permission_still_denied_before_spawn() {
    // The sandbox must not weaken policy: a WRITE-declaring package
    // executed with only READ granted is denied before any subprocess
    // exists.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("rx007-perm-{}-{seq}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let loader = write_signed_bundle(&tmp, "nexus/rx007probe", SANDBOX_PROBE);
    let bundle = loader.load("nexus/rx007probe", "1.0.0").expect("loads");
    let mut escalated = bundle.package.clone();
    escalated.manifest.permissions = vec![SkillPermission::Write];
    let executor = SkillExecutor::new(
        std::env::temp_dir().join(format!("rx007-perm-scratch-{}-{seq}", std::process::id())),
    );
    let err = executor
        .execute(&escalated, SANDBOX_PROBE, b"", &[SkillPermission::Read])
        .expect_err("ungranted permission denied");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
    let _ = std::fs::remove_dir_all(&tmp);
}

// Ensure the fixture file referenced by the suite still exists (the
// live-fire transform fixture is the canonical LF-018 payload).
#[test]
fn rx007_livefire_fixture_still_present() {
    assert!(fixture_root().join("livefire-transform.sh").exists());
}
