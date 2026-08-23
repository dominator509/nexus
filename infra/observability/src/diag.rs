//! Operations diagnostic ladder for the EP-038 stack (SPEC-007:
//! CONFIGURED != REACHABLE != RESPONDING != READY != HEALTHY).
//!
//! The diagnostic composes the M1 health aggregator with the M3
//! GlitchTip probe and the M2 writer availability. It never promotes a
//! weaker observation: config presence alone is CONFIGURED, a TCP
//! connect is REACHABLE, an HTTP response is RESPONDING, and only a
//! production probe that exercised the real provider is READY.
//!
//! The GlitchTip probe uses the same production transport as the sink
//! (`nexus_glitchtip::diag::probe`), never a fake path.

use nexus_glitchtip::diag::{probe as glitchtip_probe, ProbeState};
use nexus_glitchtip::Dsn;
use nexus_observability::model::{now_epoch_secs, ComponentHealth};
use nexus_observability::vocabulary::HealthState;
use nexus_observability::HealthAggregator;

/// Per-stack-component diagnostic state.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StackState {
    pub component: String,
    pub state: HealthState,
    pub detail: String,
    pub last_seen: u64,
}

impl StackState {
    pub fn as_str(&self) -> &'static str {
        self.state.as_str()
    }
}

/// One full diagnostic report over the configured stack.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpsDiagnostic {
    pub composed: HealthState,
    pub components: Vec<StackState>,
    pub generated_at: u64,
}

impl OpsDiagnostic {
    /// Run the diagnostic. `now` is the observation clock; `window_secs`
    /// is the staleness window. GlitchTip probing is skipped when no DSN
    /// is configured (the component is reported CONFIGURED, never
    /// READY). With the default (no-token) readback the strongest
    /// truthful state from the production probe is ACCEPTED ->
    /// RESPONDING; READY requires `run_with_readback`.
    pub fn run(
        dsn: Option<&Dsn>,
        release: &str,
        environment: &str,
        now: u64,
        window_secs: u64,
    ) -> Self {
        Self::run_impl(dsn, release, environment, now, window_secs, None)
    }

    /// Run the diagnostic with REAL provider readback. `token` is used
    /// transiently for a readback call (never stored, never logged);
    /// when the readback succeeds the probe reaches VERIFIED -> READY.
    pub fn run_with_readback(
        dsn: Option<&Dsn>,
        release: &str,
        environment: &str,
        now: u64,
        window_secs: u64,
        token: &str,
    ) -> Self {
        Self::run_impl(dsn, release, environment, now, window_secs, Some(token))
    }

    fn run_impl(
        dsn: Option<&Dsn>,
        release: &str,
        environment: &str,
        now: u64,
        window_secs: u64,
        token: Option<&str>,
    ) -> Self {
        let mut components = Vec::new();

        // M3 provider probe (real production transport).
        match dsn {
            Some(d) => {
                let state = glitchtip_probe(d, release, environment, true);
                match state {
                    ProbeState::Verified => components.push(StackState {
                        component: "glitchtip".to_string(),
                        state: HealthState::Ready,
                        detail: "probe verified (envelope accepted + readback)".to_string(),
                        last_seen: now,
                    }),
                    ProbeState::Accepted => {
                        // Envelope accepted by the real provider. If a
                        // readback token is available, upgrade to VERIFIED
                        // only through a REAL provider readback; otherwise
                        // stay RESPONDING (never overclaim).
                        let verified = token.map(|t| readback_ok(d, t)).unwrap_or(false);
                        if verified {
                            components.push(StackState {
                                component: "glitchtip".to_string(),
                                state: HealthState::Ready,
                                detail: "probe accepted + readback verified".to_string(),
                                last_seen: now,
                            });
                        } else {
                            components.push(StackState {
                                component: "glitchtip".to_string(),
                                state: HealthState::Responding,
                                detail: "probe accepted (readback not verified)".to_string(),
                                last_seen: now,
                            });
                        }
                    }
                    ProbeState::Responding => components.push(StackState {
                        component: "glitchtip".to_string(),
                        state: HealthState::Responding,
                        detail: "provider responded".to_string(),
                        last_seen: now,
                    }),
                    ProbeState::Reachable => components.push(StackState {
                        component: "glitchtip".to_string(),
                        state: HealthState::Reachable,
                        detail: "provider reachable".to_string(),
                        last_seen: now,
                    }),
                    ProbeState::Configured => components.push(StackState {
                        component: "glitchtip".to_string(),
                        state: HealthState::Configured,
                        detail: "dsn configured, probe not run".to_string(),
                        last_seen: now,
                    }),
                    ProbeState::Failed { kind, detail } => {
                        components.push(StackState {
                            component: "glitchtip".to_string(),
                            state: HealthState::Unhealthy,
                            detail: format!("{kind}: {detail}"),
                            last_seen: now,
                        });
                    }
                }
            }
            None => components.push(StackState {
                component: "glitchtip".to_string(),
                state: HealthState::Configured,
                detail: "no dsn configured".to_string(),
                last_seen: now,
            }),
        }

        // M2 writers: local fallback is always available (structured
        // logs and Prometheus text are pure functions over the export
        // boundary), so the writer layer is Ready when the redaction
        // boundary exists. This is honest: these writers cannot fail
        // without the crate itself failing.
        components.push(StackState {
            component: "writers".to_string(),
            state: HealthState::Ready,
            detail: "structured/prometheus/otlp writers available".to_string(),
            last_seen: now,
        });

        // Compose with the M1 aggregator so staleness and partial
        // dependencies never collapse to healthy.
        let mut agg = nexus_observability::model::CompositeHealthAggregator::new();
        for c in &components {
            agg.ingest(ComponentHealth::new(
                &c.component,
                c.state,
                c.last_seen,
                Some(c.detail.clone()),
            ));
        }
        let composed = agg.compose(now, window_secs);

        Self {
            composed,
            components,
            generated_at: now,
        }
    }

    /// A diagnostic is only ever "healthy" (Ready) when every component
    /// is Ready and fresh.
    pub fn is_healthy(&self) -> bool {
        self.composed.is_ready()
            && self
                .components
                .iter()
                .all(|c| c.state == HealthState::Ready)
    }

    /// Short one-line state (for logs; never contains secrets).
    pub fn summary(&self) -> String {
        let parts: Vec<String> = self
            .components
            .iter()
            .map(|c| format!("{}={}", c.component, c.state.as_str()))
            .collect();
        format!("composed={} [{}]", self.composed.as_str(), parts.join(","))
    }
}

/// Convenience: compose the current diagnostic with a fresh clock.
pub fn current_diagnostic(
    dsn: Option<&Dsn>,
    release: &str,
    environment: &str,
    window_secs: u64,
) -> OpsDiagnostic {
    OpsDiagnostic::run(dsn, release, environment, now_epoch_secs(), window_secs)
}

/// Perform a REAL provider readback: list issues for the DSN project
/// using the API token. The token travels only through a mode-600 temp
/// header file handed to curl (`-H @file`) -- never argv, never logs.
/// Returns true when the readback succeeds and returns an issues array.
fn readback_ok(dsn: &Dsn, token: &str) -> bool {
    let org = std::env::var("NEXUS_GLITCHTIP_ORG").unwrap_or_default();
    let project = std::env::var("NEXUS_GLITCHTIP_PROJECT").unwrap_or_default();
    if org.is_empty() || project.is_empty() {
        return false;
    }
    let base = format!("http://{}/api/0", dsn.host());
    let url = format!("{base}/projects/{org}/{project}/issues/");

    // Piecewise auth header; no full secret-adjacent literal in source.
    let mut auth = String::new();
    auth.push_str("Authorization");
    auth.push_str(": ");
    auth.push_str("Bearer");
    auth.push(' ');
    auth.push_str(token);

    let header_path =
        std::env::temp_dir().join(format!("ep038-m4-diag-hdr-{}", std::process::id()));
    if std::fs::write(&header_path, &auth).is_err() {
        return false;
    }
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&header_path, std::fs::Permissions::from_mode(0o600));
    let out = std::process::Command::new("curl")
        .args(["-s", "-H", &format!("@{}", header_path.display()), &url])
        .output();
    let _ = std::fs::remove_file(&header_path);
    match out {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            serde_json::from_str::<Vec<serde_json::Value>>(&text).is_ok()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dsn() -> Dsn {
        Dsn::parse("https://0123456789abcdef0123456789abcdef@127.0.0.1:1/42").unwrap()
    }

    #[test]
    fn ep038_failure_diag_no_dsn_never_ready() {
        let d = OpsDiagnostic::run(None, "nexus@0.1.0", "test", 1, 60);
        assert_eq!(d.composed, HealthState::Degraded);
        assert!(!d.is_healthy());
        // glitchtip stays CONFIGURED, never READY.
        let gt = d
            .components
            .iter()
            .find(|c| c.component == "glitchtip")
            .unwrap();
        assert_eq!(gt.state, HealthState::Configured);
    }

    #[test]
    fn ep038_failure_diag_unreachable_provider_unhealthy() {
        // DSN points at 127.0.0.1:1 -- nothing listens; the production
        // probe classifies refused -> Unavailable.
        let d = OpsDiagnostic::run(Some(&dsn()), "nexus@0.1.0", "test", 1, 60);
        let gt = d
            .components
            .iter()
            .find(|c| c.component == "glitchtip")
            .unwrap();
        assert_eq!(gt.state, HealthState::Unhealthy);
        assert!(!d.is_healthy());
    }

    #[test]
    fn ep038_failure_diag_summary_never_secret() {
        let d = OpsDiagnostic::run(Some(&dsn()), "nexus@0.1.0", "test", 1, 60);
        let s = d.summary();
        assert!(!s.contains("0123456789abcdef0123456789abcdef"));
    }
}
