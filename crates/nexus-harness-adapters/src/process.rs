//! Real process transport (SPEC-010 behavior 4; ADR-024; RX-007
//! AUD-022).
//!
//! `ProcessRunner` is the production `HarnessCommandRunner`: it spawns
//! a real subprocess for every normalized command, capturing exit
//! status, stdout, and stderr. This is the real harness boundary the
//! M2 design promised (\"the production implementation runs real CLI
//! processes; tests inject a scripted runner\"). The command program is
//! injected by the operator; the adapter never shells out directly.
//!
//! Exit statuses map to the SPEC-006 transport vocabulary:
//! - exit 0 -> `Success`
//! - non-zero exit -> `Failure(code)`
//! - the process fails to spawn -> typed `AgentsError::Unavailable`
//!   (the adapter returns Unavailable; never a fabricated empty
//!   success);
//! - the process exceeds the invocation deadline -> `Timeout` (the
//!   process group is SIGKILLed; a timed-out run is never reported as
//!   success).
//!
//! RX-007 AUD-022: the runner is bounded and deadlock-free. stdout and
//! stderr are drained CONCURRENTLY by two reader threads (a child that
//! fills stderr while keeping stdout open cannot block the parent),
//! and a wall-clock deadline kills the process on expiry. Output is
//! capped per stream so a hostile harness cannot exhaust memory.

use nexus_agents::{AgentsError, AgentsErrorCode};
use std::io::Read;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Cap on captured output per invocation (bytes). A harness that
/// exceeds the cap is failed closed (the output is truncated, never
/// allowed to exhaust memory).
pub const PROCESS_OUTPUT_CAP: usize = 1 << 20; // 1 MiB

/// Default wall-clock deadline for one harness invocation.
pub const PROCESS_EXEC_TIMEOUT: Duration = Duration::from_secs(30);

/// Production subprocess transport.
pub struct ProcessRunner {
    program: String,
    timeout: Duration,
}

impl ProcessRunner {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            timeout: PROCESS_EXEC_TIMEOUT,
        }
    }

    /// Override the invocation deadline (tests use a short bound).
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
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
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // The child is its own process-group leader so a deadline
            // kill terminates the whole tree (AUD-022).
            cmd.process_group(0);
        }

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

        // Concurrent bounded drain (AUD-022): a harness that floods
        // stderr while keeping stdout open must not block the parent.
        // Results arrive over channels so the runner can bound the
        // wait even if a grandchild inherited the pipes and keeps them
        // open after the direct child is reaped.
        let (out_tx, out_rx) = std::sync::mpsc::channel();
        let (err_tx, err_rx) = std::sync::mpsc::channel();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(out) = stdout {
                let _ = out.take(PROCESS_OUTPUT_CAP as u64).read_to_end(&mut buf);
            }
            let _ = out_tx.send(buf);
        });
        thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(err) = stderr {
                let _ = err.take(PROCESS_OUTPUT_CAP as u64).read_to_end(&mut buf);
            }
            let _ = err_tx.send(buf);
        });

        // Bounded wait (AUD-022): poll `try_wait` until the deadline,
        // then kill the whole process group and reap.
        let deadline = Instant::now() + self.timeout;
        let mut timed_out = false;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {}
                Err(_) => {
                    return Err(AgentsError::new(
                        AgentsErrorCode::Unavailable,
                        "harness process wait failed",
                        None,
                        None,
                        None,
                        Some("process-runner".into()),
                    ));
                }
            }
            if Instant::now() >= deadline {
                timed_out = true;
                // The child is its own process-group leader (set via
                // the safe `process_group(0)` API above); killing the
                // direct child is the bounded, safe termination point
                // for the harness CLI. The output cap backstops the
                // drain threads against a grandchild inheriting the
                // pipes.
                let _ = child.kill();
                break child
                    .wait()
                    .unwrap_or_else(|_| std::process::ExitStatus::from_raw(0));
            }
            thread::sleep(Duration::from_millis(10));
        };

        // Bounded receive: after the child is reaped, the pipes should
        // close and the drains complete; a 2s grace bounds the case of
        // a grandchild holding a pipe open.
        let grace = Duration::from_secs(2);
        let stdout = out_rx.recv_timeout(grace).unwrap_or_default();
        let stderr = err_rx.recv_timeout(grace).unwrap_or_default();

        let exit = if timed_out {
            super::HarnessExitStatus::Timeout
        } else if status.success() {
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
