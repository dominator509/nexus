//! nexus-test-performance: EP-040 performance budget evaluation root
//! (SPEC-008; TESTING.md performance layer).
//!
//! M1 owns the deterministic evaluation model for PerformanceBudget:
//! a budget is met only when a real observed value is within the declared
//! bound. Missing observation, stale observation, and unobserved budgets
//! fail closed. BUILD PASSED != RUNTIME SAFE: compile success never
//! satisfies a performance budget.
//!
//! Live over-the-wire observation is owned by the leaf remediation node
//! (AUD-063, RX-021): [`RuntimeLatencyProbe`] measures the control-plane
//! healthz endpoint over a real TCP round trip and returns a
//! [`RealLatencyObservation`] that can certify a budget. A probe that
//! cannot reach a healthy endpoint never fabricates a value - it fails
//! closed, so hand-fed constants cannot masquerade as runtime evidence.

use nexus_test_contract::error::{TestingError, TestingResult};
use nexus_test_contract::model::PerformanceBudget;
use nexus_test_contract::PerformanceBudgetPort;
use serde::Serialize;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// Deterministic performance budget evaluator. Fail-closed on missing
/// observation; typed failure for over-budget evidence.
#[derive(Debug, Default)]
pub struct DeterministicBudgetEvaluator;

impl DeterministicBudgetEvaluator {
    pub fn new() -> Self {
        Self
    }

    /// Evaluate an observed value against a budget. Returns Ok only when
    /// the budget was observed and the observed value is within bound.
    pub fn evaluate_observed(
        &self,
        budget: &PerformanceBudget,
        observed: f64,
    ) -> TestingResult<()> {
        if budget.metric.trim().is_empty() {
            return Err(TestingError::validation("budget metric is required"));
        }
        if budget.max_value < 0.0 {
            return Err(TestingError::validation(
                "budget max_value must be non-negative",
            ));
        }
        if observed > budget.max_value {
            return Err(TestingError::policy(format!(
                "budget {} exceeded: observed {} > max {} {}",
                budget.id, observed, budget.max_value, budget.unit
            )));
        }
        Ok(())
    }
}

impl PerformanceBudgetPort for DeterministicBudgetEvaluator {
    fn evaluate(&self, budget: &PerformanceBudget) -> TestingResult<()> {
        match budget.observed_value {
            Some(v) => self.evaluate_observed(budget, v),
            None => Err(TestingError::missing_evidence(format!(
                "budget {} has no observed value; missing observation is never green",
                budget.id
            ))),
        }
    }
}

/// A budget is never met without a real observation (BUILD PASSED !=
/// RUNTIME SAFE). This is the model-level guard the evaluator enforces.
pub fn budget_met_fail_closed(budget: &PerformanceBudget) -> bool {
    budget.observed && budget.observed_value.is_some_and(|v| v <= budget.max_value)
}

/// Real over-the-wire latency observation (AUD-063; RX-021 leaf).
///
/// Produced only by [`RuntimeLatencyProbe`]; carries the measured value,
/// the endpoint that was actually probed, and the sample window so a
/// certification can be traced to a real runtime measurement.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RealLatencyObservation {
    /// Endpoint actually probed (scheme://host:port).
    pub endpoint: String,
    /// Path probed on the endpoint.
    pub path: String,
    /// Number of samples taken.
    pub samples: usize,
    /// Measured p95 latency in milliseconds.
    pub p95_ms: f64,
    /// Measured max latency in milliseconds.
    pub max_ms: f64,
    /// Whether every sample returned HTTP 200 healthy.
    pub healthy: bool,
}

impl RealLatencyObservation {
    /// Evaluate this real observation against a budget through the
    /// canonical deterministic evaluator. The budget's observed value is
    /// the real p95; a budget that was never observed still fails closed.
    pub fn evaluate(&self, budget: &PerformanceBudget) -> TestingResult<()> {
        if !self.healthy {
            return Err(TestingError::policy(
                "real latency observation recorded an unhealthy runtime; never certified",
            ));
        }
        if self.samples == 0 {
            return Err(TestingError::validation(
                "real latency observation requires at least one sample",
            ));
        }
        DeterministicBudgetEvaluator::new().evaluate_observed(budget, self.p95_ms)
    }

    /// Certify a budget against this real observation: the budget must be
    /// observed by this probe's p95 and within bound.
    pub fn certify(&self, budget: PerformanceBudget) -> TestingResult<PerformanceBudget> {
        let observed = budget.observe(self.p95_ms);
        self.evaluate(&observed)?;
        Ok(observed)
    }
}

/// Real TCP round-trip latency probe (AUD-063; RX-021 leaf).
///
/// Measures the control-plane healthz endpoint over real sockets with a
/// wall-clock window. Never fabricates a value: connect failure,
/// non-2xx response, or an unhealthy body fails closed.
#[derive(Debug, Clone)]
pub struct RuntimeLatencyProbe {
    /// Base URL of the runtime to probe (scheme://host:port).
    base_url: String,
    /// Path to probe for health.
    path: String,
    /// Samples to take.
    samples: usize,
    /// Per-request timeout.
    timeout: Duration,
}

impl Default for RuntimeLatencyProbe {
    fn default() -> Self {
        Self::new("http://127.0.0.1:8443")
    }
}

impl RuntimeLatencyProbe {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            path: "/healthz".to_string(),
            samples: 5,
            timeout: Duration::from_secs(2),
        }
    }

    /// Configure the health path (default `/healthz`).
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Configure sample count (default 5).
    pub fn with_samples(mut self, samples: usize) -> Self {
        self.samples = samples;
        self
    }

    /// Configure per-request timeout (default 2s).
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Endpoint under test (scheme://host:port + path).
    fn url(&self) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), self.path)
    }

    /// Parse `http://host:port` into host and port.
    fn parse_base(&self) -> TestingResult<(String, u16)> {
        let stripped = self.base_url.trim_end_matches('/');
        let rest = stripped
            .strip_prefix("http://")
            .ok_or_else(|| TestingError::validation("probe supports http:// endpoints only"))?;
        let (host, port) = match rest.rsplit_once(':') {
            Some((h, p)) => {
                let port: u16 = p.parse().map_err(|_| {
                    TestingError::validation(format!("invalid port in probe endpoint: {rest}"))
                })?;
                (h.to_string(), port)
            }
            None => (rest.to_string(), 80),
        };
        if host.is_empty() {
            return Err(TestingError::validation("probe endpoint host is empty"));
        }
        Ok((host, port))
    }

    /// One real healthz round trip; returns elapsed ms and whether the
    /// body indicated health. Fails closed on any transport error.
    fn sample_once(&self) -> TestingResult<(f64, bool)> {
        let (host, port) = self.parse_base()?;
        let started = Instant::now();
        let mut stream = TcpStream::connect((host.as_str(), port))
            .map_err(|e| TestingError::policy(format!("probe connect failed: {e}")))?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|e| TestingError::policy(format!("probe timeout config failed: {e}")))?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(|e| TestingError::policy(format!("probe timeout config failed: {e}")))?;
        let request = format!(
            "GET {} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
            self.path, host, port
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|e| TestingError::policy(format!("probe write failed: {e}")))?;
        let mut buf = Vec::with_capacity(1024);
        let mut chunk = [0u8; 1024];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut
                    {
                        return Err(TestingError::policy(format!("probe read timed out: {e}")));
                    }
                    return Err(TestingError::policy(format!("probe read failed: {e}")));
                }
            }
        }
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        let text = String::from_utf8_lossy(&buf);
        let status_ok = text.starts_with("HTTP/1.1 200") || text.starts_with("HTTP/1.0 200");
        let healthy = status_ok && text.contains("healthy");
        Ok((elapsed, healthy))
    }

    /// Run the probe. Fails closed: a single unhealthy sample or transport
    /// error aborts the whole probe.
    pub fn probe(&self) -> TestingResult<RealLatencyObservation> {
        let mut latencies: Vec<f64> = Vec::with_capacity(self.samples);
        for _ in 0..self.samples {
            let (ms, healthy) = self.sample_once()?;
            if !healthy {
                return Err(TestingError::policy(format!(
                    "probe endpoint {} did not report healthy",
                    self.url()
                )));
            }
            latencies.push(ms);
        }
        latencies.sort_by(|a, b| a.partial_cmp(b).expect("latency is a finite number"));
        let p95_idx = ((latencies.len() as f64) * 0.95).ceil() as usize - 1;
        let p95_ms = latencies[p95_idx.min(latencies.len() - 1)];
        let max_ms = latencies[latencies.len() - 1];
        Ok(RealLatencyObservation {
            endpoint: self.base_url.trim_end_matches('/').to_string(),
            path: self.path.clone(),
            samples: latencies.len(),
            p95_ms,
            max_ms,
            healthy: true,
        })
    }
}
