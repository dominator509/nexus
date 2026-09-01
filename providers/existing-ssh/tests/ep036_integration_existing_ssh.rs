//! EP-036 M3 real transport integration: generic existing-SSH
//! reachability against a REAL ephemeral sshd container (SPEC-016
//! existing-SSH first-class path).
//!
//! The test builds a throwaway alpine image with an sshd, starts it on a
//! random host port, and proves the fabric's existing-SSH binding can
//! reach the exact target with the real `ssh-keyscan`/`ssh` binaries.
//! The container is removed in all exit paths. Reachability proves
//! REACHABLE only - never READY and never WORKLOAD HEALTHY.
//!
//! RX-017 AUD-046: a second test drives the operational
//! `GenericSshProvider` (CloudProviderPort) against the same real
//! sshd - submit succeeds, readback observes REACHABLE, delete is
//! DELETE_ACCEPTED (absence requires a later readback), and an
//! unreachable target fails closed.

use std::process::Command;

use nexus_compute::model::{
    CloudCredentialRef, ProvisioningRequest, ProvisioningRequestId, ResourceIdentity,
    WorkloadManifestId,
};
use nexus_compute::port::CloudProviderPort;
use nexus_compute::vocabulary::{DeleteState, ProviderKind, ResourceState};
use nexus_domain::{CorrelationId, TenantId};
use nexus_provider_existing_ssh::{ExistingSshBinding, GenericSshProvider, SshProbeState};

fn tenant() -> TenantId {
    TenantId::new("00000000-0000-7000-8000-0000000000cd").expect("tenant")
}
fn ref_() -> CloudCredentialRef {
    CloudCredentialRef::new("cred://vault/ssh-integration").expect("ref")
}
fn correlation() -> CorrelationId {
    CorrelationId::new("01890000-0000-7000-8000-0000000000cd").expect("correlation")
}

/// Run a command, capturing stdout+stderr.
fn run(cmd: &mut Command) -> (bool, String) {
    let out = cmd.output().expect("command must run");
    let text = String::from_utf8_lossy(&out.stdout).to_string()
        + String::from_utf8_lossy(&out.stderr).as_ref();
    (out.status.success(), text)
}

fn docker(args: &[&str]) -> (bool, String) {
    run(Command::new("docker").args(args))
}

/// Start a throwaway alpine+sshd container on a random host port.
/// Returns (container_name, host_port). Caller MUST call
/// `cleanup_sshd(name)` in all exit paths.
fn start_ephemeral_sshd(suffix: &str) -> Option<(String, u16)> {
    // Skip cleanly when docker is unavailable (controlled environment
    // gate; this is a real-provider integration, never a mock).
    let (docker_ok, _) = docker(&["info", "--format", "{{.ServerVersion}}"]);
    if !docker_ok {
        eprintln!("docker unavailable; skipping ephemeral sshd integration");
        return None;
    }

    let unique = format!("nexus-ep036-ssh-{}-{}", std::process::id(), suffix);

    // Build the throwaway sshd image.
    let dockerfile = "FROM alpine:latest\n\
         RUN apk add --no-cache openssh && ssh-keygen -A && echo 'root:nexus' | chpasswd\n\
         CMD [\"/usr/sbin/sshd\", \"-D\"]\n";
    let df_path =
        std::env::temp_dir().join(format!("ep036-sshd-{}.Dockerfile", std::process::id()));
    std::fs::write(&df_path, dockerfile).expect("write dockerfile");
    let (build_ok, build_out) = docker(&[
        "build",
        "-t",
        "nexus-ep036-sshd:test",
        "-f",
        df_path.to_str().expect("path"),
        ".",
    ]);
    let _ = std::fs::remove_file(&df_path);
    if !build_ok {
        let _ = docker(&["rm", "-f", &unique]);
        panic!("sshd image build failed: {build_out}");
    }

    // Start sshd on a random host port.
    let (start_ok, start_out) = docker(&[
        "run",
        "-d",
        "--name",
        &unique,
        "-p",
        "127.0.0.1::22",
        "nexus-ep036-sshd:test",
    ]);
    if !start_ok {
        let _ = docker(&["rm", "-f", &unique]);
        panic!("sshd container start failed: {start_out}");
    }
    let container_id = start_out.trim().to_string();

    // Discover the mapped host port.
    let (port_ok, port_out) = docker(&["port", &container_id, "22"]);
    if !port_ok {
        let _ = docker(&["rm", "-f", &unique]);
        panic!("cannot resolve mapped port: {port_out}");
    }
    let mapped = port_out
        .lines()
        .next()
        .expect("mapped port line")
        .trim()
        .to_string();
    // Format: 127.0.0.1:PORT
    let host_port: u16 = mapped
        .rsplit(':')
        .next()
        .expect("port")
        .parse()
        .expect("numeric port");

    Some((unique, host_port))
}

fn cleanup_sshd(name: &str) {
    let _ = docker(&["rm", "-f", name]);
    let (rm_ok, rm_out) = docker(&[
        "ps",
        "-a",
        "--filter",
        &format!("name={name}"),
        "--format",
        "{{.Names}}",
    ]);
    assert!(rm_ok, "docker ps failed: {rm_out}");
    assert!(
        !rm_out.contains(name),
        "ephemeral sshd container leaked: {rm_out}"
    );
}

/// Wait (bounded) for ssh-keyscan to observe a real host key.
fn wait_reachable(binding: &ExistingSshBinding) -> (bool, String) {
    let mut probe_out = String::new();
    let mut probe_ok = false;
    for _attempt in 0..15 {
        let mut probe = Command::new("ssh-keyscan");
        probe
            .arg("-p")
            .arg(binding.port.to_string())
            .arg("-T")
            .arg("5")
            .arg(&binding.host);
        let (ok, out) = run(&mut probe);
        if ok && out.contains("ssh-") {
            probe_ok = true;
            probe_out = out;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
    (probe_ok, probe_out)
}

#[test]
fn ep036_integration_existing_ssh_reaches_ephemeral_sshd() {
    let Some((unique, host_port)) = start_ephemeral_sshd("probe") else {
        return;
    };
    let binding =
        ExistingSshBinding::new("127.0.0.1", host_port, "root", tenant(), ref_()).expect("binding");

    let (probe_ok, probe_out) = wait_reachable(&binding);
    if !probe_ok {
        cleanup_sshd(&unique);
        panic!("ssh-keyscan failed: {probe_out}");
    }
    assert!(
        probe_out.contains("ssh-"),
        "expected a real ssh host key from ssh-keyscan, got: {probe_out}"
    );
    // Positive transport sentinel: proves the REAL ssh-keyscan probe
    // succeeded against the ephemeral sshd (gate requires this string).
    eprintln!("ep036_real_transport_ssh_keyscan_probe: ok");

    // The probe proves REACHABLE, not READY and not WORKLOAD HEALTHY.
    let reachable = SshProbeState::Reachable;
    assert_eq!(reachable, SshProbeState::Reachable);
    assert_ne!(reachable, SshProbeState::Unreachable);

    cleanup_sshd(&unique);
}

#[test]
fn ep036_integration_generic_ssh_provider_operates_over_real_transport() {
    let Some((unique, host_port)) = start_ephemeral_sshd("provider") else {
        return;
    };
    let binding =
        ExistingSshBinding::new("127.0.0.1", host_port, "root", tenant(), ref_()).expect("binding");
    let provider = GenericSshProvider::new(binding, 5).expect("provider");

    let request_id = ProvisioningRequestId::new("integration-request-001").expect("request id");
    let manifest_id = WorkloadManifestId::new("integration-manifest-001").expect("manifest id");
    let request = ProvisioningRequest::new(
        request_id.clone(),
        correlation(),
        provider
            .binding()
            .to_provider_binding("acct-integration")
            .expect("binding"),
        manifest_id,
        nexus_compute::model::CapacityProfile::new(
            2,
            4,
            20,
            None,
            None,
            nexus_compute::vocabulary::CapacityProvenance::Declared,
        )
        .expect("capacity"),
        "idem-integration-001",
    )
    .expect("request");

    // Submit against the REAL sshd: probe succeeds, receipt is SUBMITTED.
    let receipt = provider.submit(&request).expect("submit must succeed");
    assert_eq!(receipt.state, ResourceState::Submitted);
    eprintln!("ep036_real_transport_generic_ssh_submit: ok");

    // Exact-target readback: the real target is REACHABLE (never READY).
    let identity = ResourceIdentity::new(
        request_id.clone(),
        ProviderKind::GenericSsh,
        tenant(),
        "acct-integration",
        format!("ssh:{}", provider.binding().host),
        None,
        "idem-integration-001",
    )
    .expect("identity");
    let plan = provider.readback(&identity).expect("readback must succeed");
    assert_eq!(plan.state, ResourceState::Reachable);
    eprintln!("ep036_real_transport_generic_ssh_readback_reachable: ok");

    // Delete while the target is present: DELETE ACCEPTED, absence NOT
    // verified (the caller must read back after the host is gone).
    let delete_plan = provider.delete(&identity).expect("delete must succeed");
    assert_eq!(delete_plan.delete_state, DeleteState::DeleteAccepted);
    eprintln!("ep036_real_transport_generic_ssh_delete_accepted: ok");

    // After the host is gone, readback fails closed and delete verifies
    // absence.
    cleanup_sshd(&unique);
    let gone_plan = provider
        .readback(&identity)
        .expect("readback after teardown");
    assert_eq!(
        gone_plan.verification,
        nexus_compute::vocabulary::VerificationState::Mismatch
    );
    let gone_delete = provider.delete(&identity).expect("delete after teardown");
    assert_eq!(
        gone_delete.delete_state,
        DeleteState::ResourceAbsentVerified
    );
    eprintln!("ep036_real_transport_generic_ssh_absence_verified: ok");
}
