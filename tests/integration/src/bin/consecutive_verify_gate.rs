//! AUD-062 harness: feed three real gate results into the canonical
//! ConsecutiveVerify policy. Reads one result per line from stdin:
//!   GREEN   (verify sentinel observed, exit 0)
//!   RED     (anything else)
//! Applies ConsecutiveVerify::new(3); exits 0 only when the policy
//! reports the sequence complete (three consecutive greens).
use nexus_test_contract::model::GateResult;
use nexus_test_execution::policy::ConsecutiveVerify;
use std::io::{self, BufRead};

fn main() {
    let mut seq = ConsecutiveVerify::new(3);
    let stdin = io::stdin();
    let mut seen = 0usize;
    for line in stdin.lock().lines() {
        let line = line.expect("read stdin");
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        seen += 1;
        let mut gate = GateResult::new("ep040-consecutive-verify");
        match line {
            "GREEN" => {
                gate.collected = 1;
                gate.passed = 1;
                gate.evidence_bound = true;
            }
            "RED" => {
                gate.collected = 1;
                gate.passed = 0;
                gate.failed = 1;
                gate.evidence_bound = true;
            }
            other => {
                eprintln!("unexpected gate result: {other}");
                std::process::exit(2);
            }
        }
        seq.record(gate);
    }
    if seen != 3 {
        eprintln!("consecutive-verify: expected 3 gate results, saw {seen}");
        std::process::exit(2);
    }
    if !seq.is_complete() {
        eprintln!(
            "consecutive-verify: NOT complete (consecutive_green={}, required={})",
            seq.consecutive_green, seq.required
        );
        std::process::exit(1);
    }
    println!("consecutive-verify: 3 consecutive greens observed (policy complete)");
}
