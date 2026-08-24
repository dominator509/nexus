//! Deterministic test runner: executes a real test command as a
//! subprocess, parses its real output into evidence, and aggregates a
//! GateResult with real counts.
//!
//! The parser understands the canonical cargo/rust test output shapes:
//!   test <name> ... ok
//!   test <name> ... FAILED
//!   test <name> ... ignored
//!   test <name> ... skipped
//!   test result: ok. N passed; M failed; K ignored; ...
//!
//! Zero collected tests is never green; skipped/ignored required tests
//! are never passes.

use std::collections::BTreeMap;
use std::process::Command;

use nexus_test_contract::error::{TestingError, TestingErrorCode, TestingResult};
use nexus_test_contract::model::{GateResult, TestEvidence};
use nexus_test_contract::vocabulary::{FlakeClassification, TestLayer, TestOutcome};

/// A real test command to execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCommand {
    pub program: String,
    pub args: Vec<String>,
    pub layer: TestLayer,
}

impl TestCommand {
    pub fn new(program: impl Into<String>, layer: TestLayer) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            layer,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }
}

/// The parsed outcome of one test line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedLine {
    Test {
        name: String,
        outcome: TestOutcome,
    },
    Summary {
        passed: usize,
        failed: usize,
        ignored: usize,
        skipped: usize,
    },
    Other,
}

/// Parse a single cargo-style test output line deterministically.
pub fn parse_line(line: &str) -> ParsedLine {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("test ") {
        // Split on the " ... " separator that cargo prints between the
        // test name and the result word.
        if let Some((name, tail)) = rest.split_once(" ... ") {
            let outcome = if tail.starts_with("ok") {
                TestOutcome::Passed
            } else if tail.starts_with("FAILED") {
                TestOutcome::Failed
            } else if tail.starts_with("ignored") {
                TestOutcome::Ignored
            } else if tail.starts_with("skipped") {
                TestOutcome::Skipped
            } else {
                TestOutcome::Blocked
            };
            return ParsedLine::Test {
                name: name.to_string(),
                outcome,
            };
        }
    }
    if line.starts_with("test result:") {
        let passed = capture_count(line, " passed");
        let failed = capture_count(line, " failed");
        let ignored = capture_count(line, " ignored");
        let skipped = capture_count(line, " skipped");
        return ParsedLine::Summary {
            passed,
            failed,
            ignored,
            skipped,
        };
    }
    ParsedLine::Other
}

fn capture_count(line: &str, needle: &str) -> usize {
    if let Some(pos) = line.find(needle) {
        let head = &line[..pos];
        if let Some(last_space) = head.rfind(' ') {
            if let Ok(n) = head[last_space + 1..].trim().parse::<usize>() {
                return n;
            }
        }
    }
    0
}

/// Parse a full cargo-style output buffer into a deterministic run
/// record: one TestEvidence per test line plus a GateResult aggregated
/// with real counts. A missing summary line fails closed (the run did
/// not complete cleanly).
pub fn parse_output(
    gate_name: &str,
    layer: TestLayer,
    output: &str,
    evidence_bound: bool,
) -> TestingResult<(Vec<TestEvidence>, GateResult)> {
    let mut evidence = Vec::new();
    let mut seen = BTreeMap::new();
    let mut summary_seen = false;
    let mut summary = (0usize, 0usize, 0usize, 0usize);

    for raw in output.lines() {
        match parse_line(raw) {
            ParsedLine::Test { name, outcome } => {
                if seen.insert(name.clone(), outcome).is_none() {
                    let mut ev = TestEvidence::new(name, layer).record_run(outcome);
                    // A passing parse is not behavior verification; only
                    // certify_production with real evidence marks it.
                    if outcome == TestOutcome::Failed || outcome == TestOutcome::Blocked {
                        ev.flake_classification = Some(FlakeClassification::Transient);
                    }
                    evidence.push(ev);
                }
            }
            ParsedLine::Summary {
                passed,
                failed,
                ignored,
                skipped,
            } => {
                summary_seen = true;
                summary = (passed, failed, ignored, skipped);
            }
            ParsedLine::Other => {}
        }
    }

    if !summary_seen {
        return Err(TestingError::verification(
            "test output has no summary line; run did not complete cleanly",
        ));
    }

    let (passed, failed, ignored, skipped) = summary;
    let collected = evidence.len();
    let mut gate = GateResult::new(gate_name);
    gate.collected = collected;
    gate.passed = passed;
    gate.failed = failed;
    gate.skipped = skipped;
    gate.ignored = ignored;
    gate.evidence_bound = evidence_bound;
    gate.evidence = evidence
        .iter()
        .map(|e| e.test_id.clone())
        .collect::<Vec<_>>();
    Ok((evidence, gate))
}

/// Run a real test command, capture its real output, and parse it.
/// Fails closed when the process cannot be spawned or exits non-zero
/// without a summary (a crashed run is never green).
pub fn run_tests(
    gate_name: &str,
    cmd: &TestCommand,
    evidence_bound: bool,
) -> TestingResult<(Vec<TestEvidence>, GateResult)> {
    let child = Command::new(&cmd.program)
        .args(&cmd.args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            TestingError::new(
                TestingErrorCode::Unavailable,
                format!("cannot spawn {}: {e}", cmd.program),
            )
        })?;

    let output = child.wait_with_output().map_err(|e| {
        TestingError::new(
            TestingErrorCode::Unavailable,
            format!("cannot read output: {e}"),
        )
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    let (evidence, mut gate) = parse_output(gate_name, cmd.layer, &combined, evidence_bound)?;
    if !output.status.success() {
        // A non-zero exit is a real failure even if the parser could not
        // attribute it to a specific test line.
        gate.failed = gate.failed.max(1);
    }
    Ok((evidence, gate))
}
