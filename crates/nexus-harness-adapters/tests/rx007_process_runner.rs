//! RX-007 AUD-022 hostile regressions for the EP-017 ProcessRunner
//! transport.
//!
//! The production ProcessRunner must be bounded and deadlock-free:
//! - a harness child that floods stderr while keeping stdout open
//!   completes normally (concurrent stdout/stderr drain);
//! - a harness child that never exits is killed at the invocation
//!   deadline and the result is `HarnessExitStatus::Timeout` - never a
//!   fabricated success.
//!
//! The harness executables are CONTROLLED_TEST_FIXTURE shell scripts
//! driven through the REAL ProcessRunner subprocess boundary.

use nexus_harness_adapters::{
    HarnessCommand, HarnessCommandKind, HarnessCommandRunner, ProcessRunner,
};
use std::time::Duration;

fn fixture(name: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/agents/fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

use std::path::PathBuf;

fn run_once(program: &str, kind: &str, timeout: Duration) -> nexus_harness_adapters::HarnessOutput {
    let mut runner = ProcessRunner::new(program).with_timeout(timeout);
    runner
        .run(HarnessCommand {
            kind: HarnessCommandKind::Start,
            args: vec![kind.into()],
            workdir: None,
            input: None,
        })
        .expect("runner returns output")
}

#[test]
fn rx007_process_runner_stderr_flood_completes() {
    // AUD-022: the ProcessRunner drains stdout and stderr
    // concurrently. A child that writes ~360 KB to stderr (filling the
    // 64 KB pipe several times) while keeping stdout open must
    // complete with exit 0 instead of deadlocking.
    let out = run_once(
        &fixture("stderr-flood-fixture.sh"),
        "FLOOD",
        Duration::from_secs(30),
    );
    assert_eq!(
        out.status,
        nexus_harness_adapters::HarnessExitStatus::Success
    );
    assert!(out.stdout.contains("stdout-done"), "got: {}", out.stdout);
    assert!(
        out.stderr.contains("stderr-line-19999"),
        "stderr drained: {} bytes",
        out.stderr.len()
    );
}

#[test]
fn rx007_process_runner_timeout_kills_hung_child() {
    // AUD-022: a harness child that never exits is SIGKILLed at the
    // invocation deadline and surfaces as Timeout, never Success.
    let out = run_once(
        &fixture("sleep-fixture.sh"),
        "SLEEP",
        Duration::from_millis(500),
    );
    assert_eq!(
        out.status,
        nexus_harness_adapters::HarnessExitStatus::Timeout
    );
}

#[test]
fn rx007_process_runner_existing_fail_closed_behavior_holds() {
    // The AUD-022 fix must not weaken the existing transport contract:
    // a forced non-zero exit still maps to Failure, and a missing
    // executable still fails closed as Unavailable.
    let out = run_once(
        &fixture("coding-agent-fixture.sh"),
        "FAIL",
        Duration::from_secs(10),
    );
    assert_eq!(
        out.status,
        nexus_harness_adapters::HarnessExitStatus::Failure(3)
    );

    let mut runner = ProcessRunner::new("/nonexistent/rx007-no-such-program")
        .with_timeout(Duration::from_secs(10));
    let err = runner
        .run(HarnessCommand {
            kind: HarnessCommandKind::Start,
            args: vec!["START".into()],
            workdir: None,
            input: None,
        })
        .expect_err("missing executable fails closed");
    assert_eq!(err.code, nexus_agents::AgentsErrorCode::Unavailable);
}
