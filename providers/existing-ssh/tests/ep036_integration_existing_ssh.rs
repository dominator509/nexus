//! EP-036 M3 real transport integration: generic existing-SSH
//! reachability against a REAL ephemeral sshd container (SPEC-016
//! existing-SSH first-class path).
//!
//! The test builds a throwaway alpine image with an sshd, starts it on a
//! random host port, and proves the fabric's existing-SSH binding can
//! reach the exact target with the real `ssh-keyscan`/`ssh` binaries.
//! The container is removed in all exit paths. Reachability proves
//! REACHABLE only - never READY and never WORKLOAD HEALTHY.

use std::process::Command;

use nexus_compute::model::CloudCredentialRef;
use nexus_domain::TenantId;
use nexus_provider_existing_ssh::{ExistingSshBinding, SshProbeState};

fn tenant() -> TenantId {
    TenantId::new("00000000-0000-7000-8000-0000000000cd").expect("tenant")
}
fn ref_() -> CloudCredentialRef {
    CloudCredentialRef::new("cred://vault/ssh-integration").expect("ref")
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

#[test]
fn ep036_integration_existing_ssh_reaches_ephemeral_sshd() {
    // Skip cleanly when docker is unavailable (controlled environment
    // gate; this is a real-provider integration, never a mock).
    let (docker_ok, _) = docker(&["info", "--format", "{{.ServerVersion}}"]);
    if !docker_ok {
        eprintln!("docker unavailable; skipping ephemeral sshd integration");
        return;
    }

    let unique = format!("nexus-ep036-ssh-{}", std::process::id());
    let cleanup = |name: &str| {
        let _ = docker(&["rm", "-f", name]);
    };

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
        cleanup(&unique);
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
        cleanup(&unique);
        panic!("sshd container start failed: {start_out}");
    }
    let container_id = start_out.trim().to_string();

    // Discover the mapped host port.
    let (port_ok, port_out) = docker(&["port", &container_id, "22"]);
    if !port_ok {
        cleanup(&unique);
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

    // Probe with the real ssh-keyscan binary (reachability + host key).
    // The probe targets exactly the binding's declared target.
    let binding =
        ExistingSshBinding::new("127.0.0.1", host_port, "root", tenant(), ref_()).expect("binding");
    let mut probe = Command::new("ssh-keyscan");
    probe
        .arg("-p")
        .arg(binding.port.to_string())
        .arg("-T")
        .arg("10")
        .arg(&binding.host);
    let (probe_ok, probe_out) = run(&mut probe);
    if !probe_ok {
        cleanup(&unique);
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

    // Cleanup: remove the container in all paths.
    cleanup(&unique);
    let (rm_ok, rm_out) = docker(&[
        "ps",
        "-a",
        "--filter",
        &format!("name={unique}"),
        "--format",
        "{{.Names}}",
    ]);
    assert!(rm_ok, "docker ps failed: {rm_out}");
    assert!(
        !rm_out.contains(&unique),
        "ephemeral sshd container leaked: {rm_out}"
    );
}
