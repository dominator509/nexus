//! AUD-063 leaf hook (RX-021): real runtime latency probe, callable from
//! shell gates. Measures the control-plane healthz endpoint over real TCP
//! round trips and certifies a declared budget through the canonical
//! evaluator path. Exits 0 only when a real healthy observation is within
//! bound; exits non-zero (never fabricating) when the runtime is
//! unreachable, unhealthy, or over budget.
//!
//! Usage: runtime_latency_probe [base-url] [max-ms] [samples]
//!   base-url defaults to http://127.0.0.1:8443
//!   max-ms   defaults to 5000
//!   samples  defaults to 5

use nexus_test_contract::model::PerformanceBudget;
use nexus_test_performance::RuntimeLatencyProbe;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let base = args
        .next()
        .unwrap_or_else(|| "http://127.0.0.1:8443".to_string());
    let max_ms: f64 = args
        .next()
        .map(|s| s.parse().expect("max-ms must be a number"))
        .unwrap_or(5000.0);
    let samples: usize = args
        .next()
        .map(|s| s.parse().expect("samples must be a number"))
        .unwrap_or(5);

    let probe = RuntimeLatencyProbe::new(base)
        .with_samples(samples)
        .with_timeout(std::time::Duration::from_secs(3));

    match probe.probe() {
        Ok(obs) => {
            println!(
                "AUD-063 runtime latency probe: endpoint={} path={} samples={} p95={:.2}ms max={:.2}ms healthy={}",
                obs.endpoint, obs.path, obs.samples, obs.p95_ms, obs.max_ms, obs.healthy
            );
            let budget =
                PerformanceBudget::new("aud063-live-healthz", "RX-021", "p95", max_ms, "ms");
            match obs.certify(budget) {
                Ok(certified) => {
                    println!(
                        "AUD-063 runtime latency certify: ok observed={:?} max={}ms",
                        certified.observed_value, max_ms
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("AUD-063 runtime latency certify: FAIL {e}");
                    ExitCode::from(2)
                }
            }
        }
        Err(e) => {
            eprintln!("AUD-063 runtime latency probe: FAIL {e}");
            ExitCode::from(1)
        }
    }
}
