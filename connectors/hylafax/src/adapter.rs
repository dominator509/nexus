//! EP-027 HylaFAX adapter core (SPEC-014; M3).
//!
//! Real production adapter behind the nexus-fax `FaxProvider` port:
//! real hfaxd client-server protocol transport (control channel +
//! EPRT data channel + MODE Z zlib document transfer) against a real
//! HylaFAX server, canonical mapping, governed submission (policy
//! BEFORE any provider mutation), exact-target verification,
//! ambiguity-safe idempotency, bounded observability (redacted audit
//! ring, counters, correlation), and fail-closed behavior.
//!
//! Permanent invariants (owner directive, EP-027):
//!
//! - SUBMITTED != DELIVERED: a successful JSUBM (job id issuance)
//!   proves submission, never delivery. DELIVERED requires later real
//!   terminal delivery evidence (physical modem / PSTN / remote fax
//!   machine receipt; all NOT ASSERTED in this controlled fixture).
//! - PROVIDER CLAIMS != NEXUS PROVED: hfaxd payloads are normalized at
//!   the boundary; a raw queue status string never becomes a domain
//!   contract.
//! - Policy gates run BEFORE any provider mutation: an unapproved or
//!   unscanned job never reaches hfaxd (zero connection/job mutation
//!   on denied sends).
//! - A status for carrier job X is verified ONLY by an observed state
//!   on carrier job X (exact target; unrelated change never verifies).
//! - UNKNOWN OUTCOME -> VERIFY FIRST -> NO BLIND RETRY: if the
//!   connection disappears after JSUBM is transmitted but before the
//!   authoritative response, the outcome is classified Verification /
//!   ambiguous and a blind retry is refused.
//! - Every operation records a correlation id; observability is
//!   bounded and poison-safe (credentials and raw bodies redacted at
//!   insert).
//!
//! No test-mode branches exist in production code.

use std::collections::HashMap;
use std::sync::Mutex;

use nexus_fax::{
    enforce_fax_policy, FaxCarrierJobId, FaxError, FaxJob, FaxJobId, FaxProvider, FaxProviderKind,
    FaxState, FaxStatus, InboundFaxRoute,
};

use crate::observability::FaxObservability;
use crate::transport::HylaFaxTransport;

/// In-flight idempotency entry for one command on one target.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InFlightEntry {
    command: String,
}

/// Real production HylaFAX adapter over a real hfaxd transport.
pub struct HylaFaxProvider {
    transport: Box<dyn HylaFaxTransport>,
    /// hfaxd host/port/credentials for the controlled fixture.
    host: String,
    port: u16,
    username: String,
    password: String,
    /// Minimum approval class for any submission (SPEC-014 behavior 8).
    min_approval_class: u8,
    in_flight: Mutex<HashMap<(String, String), InFlightEntry>>,
    /// Completed submissions by idempotency key -> provider job id.
    /// Confirmed completed replays return the SAME provider job id
    /// with zero second hfaxd mutation.
    completed: Mutex<HashMap<String, FaxCarrierJobId>>,
    observability: Mutex<FaxObservability>,
}

impl HylaFaxProvider {
    pub fn new(
        transport: Box<dyn HylaFaxTransport>,
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        min_approval_class: u8,
    ) -> Self {
        Self {
            transport,
            host: host.into(),
            port,
            username: username.into(),
            password: password.into(),
            min_approval_class,
            in_flight: Mutex::new(HashMap::new()),
            completed: Mutex::new(HashMap::new()),
            observability: Mutex::new(FaxObservability::default()),
        }
    }

    /// Redacted audit accessor (test/ops surface).
    pub fn audit(&self) -> Vec<crate::observability::FaxAuditEntry> {
        self.observability
            .lock()
            .expect("observability lock")
            .recent()
    }

    fn record(
        &self,
        correlation: String,
        operation: &str,
        outcome: &str,
        detail: String,
        fields: std::collections::BTreeMap<String, String>,
    ) {
        self.observability
            .lock()
            .expect("observability lock")
            .record(correlation, operation, outcome, detail, fields);
    }

    fn begin(&self, command: &str, target: &str, correlation: &str) -> Result<(), FaxError> {
        let mut inflight = self.in_flight.lock().expect("in_flight lock");
        let key = (command.to_string(), target.to_string());
        if inflight.contains_key(&key) {
            let err =
                FaxError::conflict(format!("command {command} already in flight for {target}"))
                    .with_correlation(correlation);
            self.record(
                correlation.to_string(),
                command,
                "CONFLICT",
                "duplicate in-flight command rejected".to_string(),
                std::collections::BTreeMap::new(),
            );
            return Err(err);
        }
        inflight.insert(
            key,
            InFlightEntry {
                command: command.into(),
            },
        );
        Ok(())
    }

    fn end(&self, command: &str, target: &str) {
        let mut inflight = self.in_flight.lock().expect("in_flight lock");
        inflight.remove(&(command.to_string(), target.to_string()));
    }

    fn gate(
        &self,
        job: &FaxJob,
        requested_approval_class: u8,
        correlation: &str,
    ) -> Result<(), FaxError> {
        if let Err(err) = enforce_fax_policy(job, requested_approval_class, self.min_approval_class)
        {
            self.record(
                correlation.to_string(),
                "SUBMIT",
                "POLICY",
                err.message.clone(),
                std::collections::BTreeMap::new(),
            );
            return Err(err);
        }
        Ok(())
    }

    /// One full hfaxd submission session: connect -> auth -> transfer
    /// prepare -> document upload -> JNEW -> JPARM* -> attach ->
    /// JSUBM -> provider job id.
    fn submit_session(&self, job: &FaxJob, correlation: &str) -> Result<FaxCarrierJobId, FaxError> {
        self.transport.connect_authenticate(
            &self.host,
            self.port,
            &self.username,
            &self.password,
        )?;
        self.transport.prepare_transfer()?;
        // The governed path guarantees the document is CLEAN-scanned
        // and approved before this point (M1 gates). The document
        // bytes are the controlled runtime artifact; digest is
        // recorded in telemetry, never the content.
        let bytes = std::fs::read(&job.document.storage_ref).map_err(|e| {
            FaxError::unavailable(format!(
                "hylafax document read failed ({}): {e}",
                job.document.storage_ref
            ))
        })?;
        let server_file = self.transport.upload_document(&bytes)?;
        let job_id = self.transport.create_job()?;
        // Job parameters observed in the real sendfax trace. The full
        // parameter set is REQUIRED by faxq: a job submitted without
        // page geometry (VRES/PAGEWIDTH/PAGELENGTH) and scheduler
        // controls is NAK'd by the scheduler (observed 460).
        let dialstring = job.to.as_str();
        let notify_addr = format!("{}@localhost", self.username);
        let params: [(&str, String); 13] = [
            ("FROMUSER", self.username.clone()),
            ("LASTTIME", "000259".to_string()),
            ("MAXDIALS", "12".to_string()),
            ("MAXTRIES", "3".to_string()),
            ("SCHEDPRI", "127".to_string()),
            ("DIALSTRING", dialstring.to_string()),
            ("NOTIFYADDR", notify_addr),
            ("VRES", "98".to_string()),
            ("PAGEWIDTH", "209".to_string()),
            ("PAGELENGTH", "296".to_string()),
            ("NOTIFY", "none".to_string()),
            ("PAGECHOP", "default".to_string()),
            ("CHOPTHRESHOLD", "3".to_string()),
        ];
        for (key, value) in params {
            self.transport.set_job_parameter(key, &value)?;
        }
        self.transport.attach_document(&server_file)?;
        let carrier = self.transport.submit_job()?;
        self.record(
            correlation.to_string(),
            "SUBMIT",
            "ok",
            format!(
                "carrier job {carrier} hfaxd job {job_id} digest {}",
                digest_fingerprint(&job.document.sha256)
            ),
            fields(Some(&carrier), Some(job_id.as_str()), "SUBMITTED"),
        );
        let _ = self.transport.quit();
        Ok(carrier)
    }
}

impl FaxProvider for HylaFaxProvider {
    fn submit(&self, job: &FaxJob) -> Result<FaxCarrierJobId, FaxError> {
        let correlation = self
            .observability
            .lock()
            .expect("observability lock")
            .correlation();
        let target = job.id.as_str();
        self.begin("SUBMIT", target, &correlation)?;

        // Policy gate BEFORE any provider mutation: an unapproved,
        // unscanned, or self-addressed job never reaches hfaxd.
        if let Err(err) = self.gate(job, job.approval_class, &correlation) {
            self.end("SUBMIT", target);
            return Err(err);
        }

        // Confirmed completed replay: return the SAME provider job id
        // with zero second hfaxd mutation.
        {
            let completed = self.completed.lock().expect("completed lock");
            if let Some(prior) = completed.get(&job.idempotency_key) {
                let prior = prior.clone();
                self.record(
                    correlation.clone(),
                    "SUBMIT",
                    "ok",
                    format!("replay deduplicated to carrier job {prior}"),
                    fields(Some(&prior), None, "SUBMITTED"),
                );
                self.end("SUBMIT", target);
                return Ok(prior);
            }
        }

        let result = self.submit_session(job, &correlation);

        match &result {
            Ok(carrier_job) => {
                if let Ok(mut completed) = self.completed.lock() {
                    completed.insert(job.idempotency_key.clone(), carrier_job.clone());
                }
            }
            Err(err) => {
                self.record(
                    correlation.clone(),
                    "SUBMIT",
                    err.code.as_str(),
                    err.message.clone(),
                    fields(None, None, "FAILED"),
                );
            }
        }
        self.end("SUBMIT", target);
        result
    }

    fn status(&self, job: &FaxJobId) -> Result<FaxStatus, FaxError> {
        let correlation = self
            .observability
            .lock()
            .expect("observability lock")
            .correlation();
        self.begin("STATUS", job.as_str(), &correlation)?;
        // Open a session and query the exact job id on the control
        // plane. The provider queue text is raw provider detail; the
        // canonical state is mapped conservatively (never DELIVERED).
        let result = (|| -> Result<FaxStatus, FaxError> {
            self.transport.connect_authenticate(
                &self.host,
                self.port,
                &self.username,
                &self.password,
            )?;
            let raw = self.transport.query_job(job.as_str())?;
            let _ = self.transport.quit();
            let state = map_queue_state(&raw)?;
            Ok(FaxStatus {
                state,
                carrier: self.kind(),
                attempts: 0,
                max_attempts: 3,
                pages: 0,
                carrier_job_id: Some(FaxCarrierJobId::new(job.as_str())?),
                // Redacted provider detail: the queue line may contain
                // the destination; store only the status fragment.
                detail: status_fragment(&raw),
            })
        })();
        match &result {
            Ok(status) => self.record(
                correlation.clone(),
                "STATUS",
                "ok",
                format!("state {}", status.state.as_str()),
                fields(status.carrier_job_id.as_ref(), None, status.state.as_str()),
            ),
            Err(err) => self.record(
                correlation.clone(),
                "STATUS",
                err.code.as_str(),
                err.message.clone(),
                std::collections::BTreeMap::new(),
            ),
        }
        self.end("STATUS", job.as_str());
        result
    }

    fn cancel(&self, job: &FaxJobId) -> Result<(), FaxError> {
        // hfaxd job cancellation is available through JKILL on the
        // control plane. Not implemented for the M3 controlled fixture
        // (no physical modem); fail closed rather than fabricate.
        let _ = job;
        Err(FaxError::unavailable(
            "hylafax cancel not implemented for the M3 controlled fixture",
        ))
    }

    fn list_inbound_routes(&self) -> Result<Vec<InboundFaxRoute>, FaxError> {
        // No inbound route table exists in the controlled fixture
        // (no modem/recvq). Fail closed; never fabricate routes.
        Err(FaxError::unavailable(
            "hylafax inbound routes not asserted in the M3 controlled fixture",
        ))
    }

    fn resolve_inbound_route(
        &self,
        _to: &nexus_fax::FaxNumber,
    ) -> Result<InboundFaxRoute, FaxError> {
        Err(FaxError::unavailable(
            "hylafax inbound routes not asserted in the M3 controlled fixture",
        ))
    }

    fn kind(&self) -> FaxProviderKind {
        FaxProviderKind::HylaFax
    }
}

/// Map a raw hfaxd LIST sendq row to the canonical fax state.
///
/// Observed/pinned JOBFMT row shape:
///   <jobid> <priority> <state-letter> <owner> <dest> <time...> <status...>
/// e.g. `22   127 W nexust +155****0200     0:0   0:12`
///
/// Only the state letter is mapped. Observed letters in the fixture:
/// W (waiting), B (blocked). Documented letters are mapped to the
/// SUBMITTED ceiling (never DELIVERED: no modem/PSTN evidence) except
/// F (failed). Unknown letters fail closed (provider vocabulary
/// change); a raw queue row is provider detail and never becomes a
/// domain contract.
fn map_queue_state(raw: &str) -> Result<FaxState, FaxError> {
    let mut tokens = raw.split_whitespace();
    let _job_id = tokens
        .next()
        .ok_or_else(|| FaxError::external("hylafax queue row missing job id"))?;
    let _priority = tokens
        .next()
        .ok_or_else(|| FaxError::external("hylafax queue row missing priority"))?;
    let state_letter = tokens
        .next()
        .ok_or_else(|| FaxError::external("hylafax queue row missing state"))?;
    match state_letter {
        // S sleeping, W waiting, R running, B blocked: queued/active.
        // D done, E done with error: completed at the provider, but
        // still at most SUBMITTED without real terminal evidence.
        "S" | "W" | "R" | "B" | "D" | "E" => Ok(FaxState::Submitted),
        "F" => Ok(FaxState::Failed),
        other => Err(FaxError::external(format!(
            "hylafax unknown job state letter {other:?}"
        ))),
    }
}

/// Extract a redacted status fragment from a raw queue line (never the
/// destination number or spool content).
fn status_fragment(raw: &str) -> String {
    // The queue line shape is: jobid state ... (provider detail). Keep
    // only the first two whitespace tokens as a bounded fingerprint.
    raw.split_whitespace().take(2).collect::<Vec<_>>().join(" ")
}

/// Safe telemetry fields: carrier job id, hfaxd job id, canonical
/// state. Never destination numbers or credentials.
fn fields(
    carrier: Option<&FaxCarrierJobId>,
    hfaxd_job: Option<&str>,
    state: &str,
) -> std::collections::BTreeMap<String, String> {
    let mut map = std::collections::BTreeMap::new();
    if let Some(c) = carrier {
        map.insert("carrier_job_id".into(), c.as_str().to_string());
    }
    if let Some(h) = hfaxd_job {
        map.insert("hfaxd_job_id".into(), h.to_string());
    }
    map.insert("state".into(), state.to_string());
    map
}

/// A bounded digest fingerprint for telemetry (never the full digest
/// chain; the first 12 hex chars are enough for correlation).
fn digest_fingerprint(sha256: &str) -> String {
    let s: String = sha256.chars().take(12).collect();
    format!("{s}...")
}

/// Build a HylaFaxProvider with the real TCP transport.
pub fn build_hylafax_provider(
    host: impl Into<String>,
    port: u16,
    username: impl Into<String>,
    password: impl Into<String>,
    min_approval_class: u8,
) -> HylaFaxProvider {
    let host: String = host.into();
    let username: String = username.into();
    let password: String = password.into();
    HylaFaxProvider::new(
        Box::new(crate::transport::HylaFaxTcpTransport::new(
            host.clone(),
            port,
            username.clone(),
            password.clone(),
        )),
        host,
        port,
        username,
        password,
        min_approval_class,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_fax::{
        submit_governed, FaxDirection, FaxDocument, FaxDocumentId, FaxNumber, FaxScanStatus,
        FaxSendRequest,
    };

    /// Scripted in-memory hfaxd transport for unit tests. The real
    /// transport is exercised by the live integration tests against
    /// the controlled fixture.
    #[derive(Default)]
    struct ScriptedTransport {
        authenticated: bool,
        _uploaded: bool,
        _created: bool,
        _submitted: bool,
        _uploaded_bytes: Vec<u8>,
    }

    impl HylaFaxTransport for ScriptedTransport {
        fn connect_authenticate(
            &self,
            _host: &str,
            _port: u16,
            _username: &str,
            _password: &str,
        ) -> Result<(), FaxError> {
            if self.authenticated {
                return Err(FaxError::unavailable("already authenticated"));
            }
            Ok(())
        }
        fn prepare_transfer(&self) -> Result<(), FaxError> {
            if !self.authenticated {
                return Err(FaxError::unavailable("not authenticated"));
            }
            Ok(())
        }
        fn upload_document(&self, _data: &[u8]) -> Result<String, FaxError> {
            if !self.authenticated {
                return Err(FaxError::unavailable("not authenticated"));
            }
            Ok("/tmp/scripted.ps".into())
        }
        fn create_job(&self) -> Result<String, FaxError> {
            Ok("99".into())
        }
        fn set_job_parameter(&self, _key: &str, _value: &str) -> Result<(), FaxError> {
            Ok(())
        }
        fn attach_document(&self, _server_file: &str) -> Result<(), FaxError> {
            Ok(())
        }
        fn submit_job(&self) -> Result<FaxCarrierJobId, FaxError> {
            FaxCarrierJobId::new("99")
        }
        fn query_job(&self, job_id: &str) -> Result<String, FaxError> {
            Ok(format!("{job_id} blocked by concurrent calls"))
        }
        fn quit(&self) -> Result<(), FaxError> {
            Ok(())
        }
    }

    fn clean_document() -> FaxDocument {
        FaxDocument {
            id: FaxDocumentId::new("doc-h").expect("id"),
            filename: "a.pdf".into(),
            content_type: "application/pdf".into(),
            size_bytes: 64,
            pages: 1,
            sha256: "abc123def456abc123def456abc123def456abc123def456abc123def456abc1".into(),
            storage_ref: "/tmp/does-not-matter.txt".into(),
            scan_status: FaxScanStatus::Clean,
        }
    }

    fn job(approval_class: u8) -> FaxJob {
        let from_num: String = format!("+1555{}", "0100");
        let to_num: String = format!("+1555{}", "0200");
        FaxJob {
            id: FaxJobId::new("job-h").expect("id"),
            direction: FaxDirection::Outbound,
            from: FaxNumber::new(from_num).expect("from"),
            to: FaxNumber::new(to_num).expect("to"),
            document: clean_document(),
            carrier: FaxProviderKind::HylaFax,
            status: FaxStatus {
                state: FaxState::Queued,
                carrier: FaxProviderKind::HylaFax,
                attempts: 0,
                max_attempts: 3,
                pages: 1,
                carrier_job_id: None,
                detail: "queued".into(),
            },
            idempotency_key: "key-h".into(),
            approval_class,
            correlation: None,
        }
    }

    #[test]
    fn ep027_unit_hylafax_provider_kind() {
        let p = HylaFaxProvider::new(
            Box::new(ScriptedTransport::default()),
            "127.0.0.1",
            4559,
            "u",
            "p",
            1,
        );
        assert_eq!(p.kind(), FaxProviderKind::HylaFax);
    }

    #[test]
    fn ep027_unit_hylafax_governed_denied_never_reaches_transport() {
        // A denied send must not even open a session (zero provider
        // mutation). We use a transport that panics on any call to
        // prove the gate is BEFORE the transport.
        struct PanicTransport;
        impl HylaFaxTransport for PanicTransport {
            fn connect_authenticate(
                &self,
                _h: &str,
                _p: u16,
                _u: &str,
                _p2: &str,
            ) -> Result<(), FaxError> {
                panic!("denied send must not reach transport");
            }
            fn prepare_transfer(&self) -> Result<(), FaxError> {
                panic!("denied send must not reach transport");
            }
            fn upload_document(&self, _d: &[u8]) -> Result<String, FaxError> {
                panic!("denied send must not reach transport");
            }
            fn create_job(&self) -> Result<String, FaxError> {
                panic!("denied send must not reach transport");
            }
            fn set_job_parameter(&self, _k: &str, _v: &str) -> Result<(), FaxError> {
                panic!("denied send must not reach transport");
            }
            fn attach_document(&self, _f: &str) -> Result<(), FaxError> {
                panic!("denied send must not reach transport");
            }
            fn submit_job(&self) -> Result<FaxCarrierJobId, FaxError> {
                panic!("denied send must not reach transport");
            }
            fn query_job(&self, _j: &str) -> Result<String, FaxError> {
                panic!("denied send must not reach transport");
            }
            fn quit(&self) -> Result<(), FaxError> {
                panic!("denied send must not reach transport");
            }
        }
        let p = HylaFaxProvider::new(Box::new(PanicTransport), "127.0.0.1", 4559, "u", "p", 1);
        let j = job(0); // below minimum approval class
        let req = FaxSendRequest {
            job: j.id.clone(),
            idempotency_key: "key-h".into(),
            approval_class: 0,
        };
        let err = submit_governed(&p, &j, &req, 1).expect_err("must be denied");
        assert_eq!(err.code, nexus_fax::FaxErrorCode::Policy);
    }

    #[test]
    fn ep027_unit_hylafax_submitted_is_not_delivered() {
        // The canonical mapping NEVER produces DELIVERED from a queue
        // row; the ceiling is SUBMITTED. State is the JOBFMT letter.
        // Observed fixture letters: W (waiting), B (blocked).
        assert_eq!(
            map_queue_state("6   127 W nexust 15551234567   0:0   0:12"),
            Ok(FaxState::Submitted)
        );
        assert_eq!(
            map_queue_state("6   127 B nexust 15551234567   0:0   0:12"),
            Ok(FaxState::Submitted)
        );
        assert_eq!(
            map_queue_state("6   127 F nexust 15551234567   0:0   0:12"),
            Ok(FaxState::Failed)
        );
        // Unknown provider vocabulary fails closed.
        assert!(map_queue_state("6   127 X nexust 15551234567").is_err());
        assert!(map_queue_state("6").is_err());
        assert!(map_queue_state("").is_err());
    }
}
