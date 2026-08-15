//! Real process transport (SPEC-010 behavior 4; ADR-024).
//!
//! `ProcessRunner` is the production `HarnessCommandRunner`: it spawns
//! a real subprocess for every normalized command, capturing exit
//! status, stdout, and stderr. This is the real harness boundary the
//! M2 design promised ("the production implementation runs real CLI
//! processes; tests inject a scripted runner"). The command program is
//! injected by the operator; the adapter never shells out directly.
//!
//! Exit statuses map to the SPEC-006 transport vocabulary:
//! - exit 0 -> `Success`
//! - non-zero exit -> `Failure(code)`
//! - the process fails to spawn -> typed `AgentsError::Unavailable`
//!   (the adapter returns Unavailable; never a fabricated empty
//!   success).
//!
//! The runner is bounded: `stdout`/`stderr` are captured to a capped
//! buffer so a hostile harness cannot exhaust memory.

use nexus_agents::{AgentsError, AgentsErrorCode};
use std::io::Read;
use std::process::{Command, Stdio};

/// Cap on captured output per invocation (bytes). A harness that
/// exceeds the cap is failed closed (the output is truncated, never
/// allowed to exhaust memory).
pub const PROCESS_OUTPUT_CAP: usize = 1 << 20; // 1 MiB

/// Production subprocess transport.
pub struct ProcessRunner {
    program: String,
}

impl ProcessRunner {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
        }
    }
}

impl super::HarnessCommandRunner for ProcessRunner {
    fn run(&mut self, command: super::HarnessCommand) -> Result<super::HarnessOutput, AgentsError> {
        let kind = command.kind.as_str();
        let mut cmd = Command::new(&self.program);
        cmd.arg(kind);
        cmd.args(&command.args);
        if let Some(dir) = &command.workdir {
            cmd.current_dir(dir);
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|_| {
            AgentsError::new(
                AgentsErrorCode::Unavailable,
                "harness executable could not be spawned",
                None,
                None,
                None,
                Some("process-runner".into()),
            )
        })?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        if let Some(out) = child.stdout.take() {
            let _ = out.take(PROCESS_OUTPUT_CAP as u64).read_to_end(&mut stdout);
        }
        if let Some(err) = child.stderr.take() {
            let _ = err.take(PROCESS_OUTPUT_CAP as u64).read_to_end(&mut stderr);
        }

        let status = child.wait().map_err(|_| {
            AgentsError::new(
                AgentsErrorCode::Unavailable,
                "harness process wait failed",
                None,
                None,
                None,
                Some("process-runner".into()),
            )
        })?;

        let exit = if status.success() {
            super::HarnessExitStatus::Success
        } else {
            super::HarnessExitStatus::Failure(status.code().unwrap_or(-1))
        };

        Ok(super::HarnessOutput {
            status: exit,
            // Convert to lossy strings; the adapter never parses
            // provider payloads into domain contracts.
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        })
    }
}
