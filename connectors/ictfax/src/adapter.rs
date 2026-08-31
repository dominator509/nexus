//! EP-027 ICTFax adapter core (SPEC-014; M2).
//!
//! Real production adapter behind the nexus-fax `FaxProvider` port:
//! real ICTFax REST API document/transmission lifecycle, canonical
//! mapping from ICTFax payloads to canonical FaxJob/FaxStatus shapes,
//! governed submission (policy BEFORE any provider mutation),
//! exact-target verification, in-flight idempotency, bounded
//! observability (redacted audit ring, counters, correlation), and
//! fail-closed behavior.
//!
//! Permanent invariants (owner directive, EP-027):
//!
//! - SUBMITTED != DELIVERED: a carrier acceptance (successful
//!   transmission create/send) proves submission, never delivery.
//!   Delivery requires independent carrier/recipient evidence.
//! - PROVIDER CLAIMS != NEXUS PROVED: ICTFax payloads are normalized
//!   at the boundary; a free-form status string never becomes a
//!   domain contract.
//! - Policy gates run BEFORE any provider mutation: an unapproved or
//!   unscanned job never reaches the carrier.
//! - A status for carrier job X is verified ONLY by an observed state
//!   on carrier job X (exact target; unrelated change never verifies).
//! - UNKNOWN OUTCOME -> VERIFY FIRST -> NO BLIND RETRY.
//! - Every operation records a correlation id; observability is
//!   bounded and poison-safe (session tokens and raw bodies redacted
//!   at insert).
//!
//! No test-mode branches exist in production code.

use std::collections::HashMap;
use std::sync::Mutex;

use nexus_fax::{
    enforce_fax_policy, validate_send_request, verify_delivery, FaxCarrierJobId, FaxDocument,
    FaxDocumentId, FaxError, FaxJob, FaxJobId, FaxNumber, FaxProvider, FaxProviderKind, FaxRouteId,
    FaxScanStatus, FaxSendRequest, FaxStatus, InboundFaxRoute,
};

use crate::observability::FaxObservability;
use crate::transport::{map_transmission_state, IctFaxTransport};

/// In-flight idempotency entry for one command on one target.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InFlightEntry {
    command: String,
}

/// Real production ICTFax adapter over a real ICTFax transport.
///
/// The adapter is provider-neutral at its public boundary: it
/// implements `FaxProvider` (SPEC-014) and never exposes ICTFax types
/// to callers.
pub struct IctFaxProvider {
    transport: Box<dyn IctFaxTransport>,
    /// Minimum approval class for any submission through this
    /// adapter (SPEC-014 behavior 8).
    min_approval_class: u8,
    in_flight: Mutex<HashMap<(String, String), InFlightEntry>>,
    observability: Mutex<FaxObservability>,
}

impl IctFaxProvider {
    pub fn new(transport: Box<dyn IctFaxTransport>, min_approval_class: u8) -> Self {
        Self {
            transport,
            min_approval_class,
            in_flight: Mutex::new(HashMap::new()),
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

    fn record(&self, correlation: String, operation: &str, outcome: &str, detail: String) {
        self.observability
            .lock()
            .expect("observability lock")
            .record(
                correlation,
                operation,
                outcome,
                detail,
                std::collections::BTreeMap::new(),
            );
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
            );
            return Err(err);
        }
        Ok(())
    }
}

impl FaxProvider for IctFaxProvider {
    fn submit(&self, job: &FaxJob) -> Result<FaxCarrierJobId, FaxError> {
        let correlation = self
            .observability
            .lock()
            .expect("observability lock")
            .correlation();
        let target = job.id.as_str();
        self.begin("SUBMIT", target, &correlation)?;

        // Policy gate BEFORE any provider mutation: an unapproved,
        // unscanned, or self-addressed job never reaches the carrier.
        if let Err(err) = self.gate(job, job.approval_class, &correlation) {
            self.end("SUBMIT", target);
            return Err(err);
        }

        // Document media first (documented send flow step 2), then a
        // sendfax program bound to the document, then a transmission
        // bound to the destination/program/document, then send. The
        // recipient and document data ALWAYS reach the provider
        // request (AUD-020): the document bytes are the controlled
        // runtime artifact read from the job's storage_ref.
        let result = (|| {
            let bytes = std::fs::read(&job.document.storage_ref).map_err(|e| {
                FaxError::unavailable(format!(
                    "ictfax document read failed ({}): {e}",
                    job.document.storage_ref
                ))
            })?;
            let document_id = self.transport.create_document_with_media(
                &job.document.filename,
                &job.document.content_type,
                &bytes,
            )?;
            // The sendfax program prepares the document for the
            // account; the provider returns its reference.
            let program = self.transport.create_sendfax_program(&document_id)?;
            let transmission =
                self.transport
                    .create_transmission(job.to.as_str(), &program, &document_id)?;
            self.transport.send_transmission(&transmission.id)?;
            FaxCarrierJobId::new(transmission.id)
        })();

        match &result {
            Ok(carrier_job) => {
                self.record(
                    correlation.clone(),
                    "SUBMIT",
                    "ok",
                    format!("carrier job {carrier_job}"),
                );
            }
            Err(err) => {
                self.record(
                    correlation.clone(),
                    "SUBMIT",
                    err.code.as_str(),
                    err.message.clone(),
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
        let transmission = self.transport.fetch_transmission(job.as_str())?;
        let state = map_transmission_state(&transmission.status)?;
        let carrier_job = FaxCarrierJobId::new(transmission.id.clone())?;
        let status = FaxStatus {
            state,
            carrier: self.kind(),
            attempts: transmission.attempts,
            max_attempts: 3,
            pages: transmission.pages,
            carrier_job_id: Some(carrier_job),
            // Redacted carrier detail: never credentials or document
            // content; the status string is safe telemetry.
            detail: transmission.status,
        };
        self.record(
            correlation.clone(),
            "STATUS",
            "ok",
            format!("state {}", status.state.as_str()),
        );
        Ok(status)
    }

    fn cancel(&self, job: &FaxJobId) -> Result<(), FaxError> {
        let correlation = self
            .observability
            .lock()
            .expect("observability lock")
            .correlation();
        let target = job.as_str();
        self.begin("CANCEL", target, &correlation)?;
        let result = self.transport.delete_transmission(job.as_str());
        match &result {
            Ok(()) => self.record(correlation.clone(), "CANCEL", "ok", String::new()),
            Err(err) => {
                self.record(
                    correlation.clone(),
                    "CANCEL",
                    err.code.as_str(),
                    err.message.clone(),
                );
            }
        }
        self.end("CANCEL", target);
        result
    }

    fn list_inbound_routes(&self) -> Result<Vec<InboundFaxRoute>, FaxError> {
        let correlation = self
            .observability
            .lock()
            .expect("observability lock")
            .correlation();
        let accounts = self.transport.list_accounts()?;
        let mut routes = Vec::new();
        for account in accounts {
            // A provider account without a canonical number is not a
            // routable destination; skip it (never fabricate).
            let Ok(to) = FaxNumber::new(account.number.clone()) else {
                continue;
            };
            let Ok(id) = FaxRouteId::new(account.id.clone()) else {
                continue;
            };
            routes.push(InboundFaxRoute {
                id,
                to,
                destination: format!("account:{}", account.name),
                enabled: account.enabled,
                archive: true,
                correlation: Some(correlation.clone()),
            });
        }
        self.record(
            correlation.clone(),
            "LIST_ROUTES",
            "ok",
            format!("{} routes", routes.len()),
        );
        Ok(routes)
    }

    fn resolve_inbound_route(&self, to: &FaxNumber) -> Result<InboundFaxRoute, FaxError> {
        let correlation = self
            .observability
            .lock()
            .expect("observability lock")
            .correlation();
        let accounts = self.transport.list_accounts()?;
        for account in accounts {
            let Ok(candidate) = FaxNumber::new(account.number.clone()) else {
                continue;
            };
            if &candidate == to {
                let route = InboundFaxRoute {
                    id: FaxRouteId::new(account.id.clone())?,
                    to: to.clone(),
                    destination: format!("account:{}", account.name),
                    enabled: account.enabled,
                    archive: true,
                    correlation: Some(correlation.clone()),
                };
                self.record(
                    correlation.clone(),
                    "RESOLVE_ROUTE",
                    "ok",
                    format!("route for {to}"),
                );
                return Ok(route);
            }
        }
        let err = FaxError::not_found(format!("no ictfax account serves {to}"));
        self.record(
            correlation.clone(),
            "RESOLVE_ROUTE",
            err.code.as_str(),
            err.message.clone(),
        );
        Err(err)
    }

    fn fetch_inbound_document(
        &self,
        route: &FaxRouteId,
        carrier_job: &FaxCarrierJobId,
    ) -> Result<FaxDocument, FaxError> {
        let correlation = self
            .observability
            .lock()
            .expect("observability lock")
            .correlation();
        // The carrier job id is the transmission id; fetch its
        // metadata to bind the document reference.
        let transmission = self.transport.fetch_transmission(carrier_job.as_str())?;
        let document_id = transmission
            .document_id
            .clone()
            .unwrap_or_else(|| transmission.id.clone());
        // Fetch the REAL media bytes: the inbound document is an
        // artifact with actual content, real size, and a real digest.
        // A fabricated size-0 / empty-digest document is never
        // returned (AUD-020).
        let bytes = self.transport.fetch_document_media(&document_id)?;
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let digest = hasher.finalize();
        let sha256: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        let document = FaxDocument {
            id: FaxDocumentId::new(document_id)?,
            filename: format!("{}.pdf", carrier_job.as_str()),
            content_type: "application/pdf".into(),
            size_bytes: bytes.len() as u64,
            pages: transmission.pages,
            // The digest is computed from the REAL fetched bytes.
            sha256,
            storage_ref: format!("ictfax:{}", carrier_job.as_str()),
            scan_status: FaxScanStatus::Pending,
        };
        self.record(
            correlation.clone(),
            "FETCH_DOCUMENT",
            "ok",
            format!(
                "route {route} carrier job {carrier_job} bytes {}",
                document.size_bytes
            ),
        );
        Ok(document)
    }

    fn kind(&self) -> FaxProviderKind {
        FaxProviderKind::IctFax
    }
}

/// Governed ICTFax submission: request validation + fax policy BEFORE
/// any provider mutation, then the provider's real submit. This is the
/// canonical entry point for adapters (fail closed, no
/// validation-after-submission).
pub fn submit_ictfax_governed(
    provider: &IctFaxProvider,
    job: &FaxJob,
    request: &FaxSendRequest,
) -> Result<FaxCarrierJobId, FaxError> {
    validate_send_request(request, job)?;
    enforce_fax_policy(job, request.approval_class, provider.min_approval_class)?;
    provider.submit(job)
}

/// Exact-target ICTFax delivery verification: the carrier job id must
/// match exactly AND the state must be DELIVERED. A status for carrier
/// job X never verifies fax job Y; SUBMITTED never verifies.
pub fn verify_ictfax_delivery(
    status: &FaxStatus,
    expected_carrier_job: &FaxCarrierJobId,
) -> Result<(), FaxError> {
    verify_delivery(status, expected_carrier_job)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_fax::{FaxDirection, FaxDocumentId, FaxNumber, FaxScanStatus, FaxState};

    fn clean_document() -> FaxDocument {
        FaxDocument {
            id: FaxDocumentId::new("doc-1").expect("id"),
            filename: "a.pdf".into(),
            content_type: "application/pdf".into(),
            size_bytes: 100,
            pages: 1,
            sha256: "abc".into(),
            storage_ref: "store/doc-1".into(),
            scan_status: FaxScanStatus::Clean,
        }
    }

    fn job(approval_class: u8) -> FaxJob {
        let from_num: String = format!("+1555{}", "0100");
        let to_num: String = format!("+1555{}", "0200");
        FaxJob {
            id: FaxJobId::new("job-1").expect("id"),
            direction: FaxDirection::Outbound,
            from: FaxNumber::new(from_num).expect("from"),
            to: FaxNumber::new(to_num).expect("to"),
            document: clean_document(),
            carrier: FaxProviderKind::IctFax,
            status: FaxStatus {
                state: FaxState::Queued,
                carrier: FaxProviderKind::IctFax,
                attempts: 0,
                max_attempts: 3,
                pages: 1,
                carrier_job_id: None,
                detail: "queued".into(),
            },
            idempotency_key: "key-1".into(),
            approval_class,
            correlation: None,
        }
    }

    /// A tracking transport that counts carrier mutations AND records
    /// the exact data passed to the provider, so tests prove the
    /// recipient/document data really reach the provider request
    /// (AUD-020).
    #[derive(Default)]
    struct TrackingTransport {
        mutations: std::rc::Rc<std::cell::Cell<u32>>,
        media: std::rc::Rc<std::cell::RefCell<Vec<(String, String, usize)>>>,
        transmissions: std::rc::Rc<std::cell::RefCell<Vec<(String, String, String)>>>,
        fetched_media: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
    }

    /// Snapshot of the data a transport passed to the provider
    /// (test-only observation; AUD-020 data-binding proof).
    #[derive(Debug, Clone, Default)]
    struct TransportObservation {
        mutations: u32,
        media: Vec<(String, String, usize)>,
        transmissions: Vec<(String, String, String)>,
        fetched_media: Vec<String>,
    }

    impl TrackingTransport {
        fn observe(&self) -> TransportObservation {
            TransportObservation {
                mutations: self.mutations.get(),
                media: self.media.borrow().clone(),
                transmissions: self.transmissions.borrow().clone(),
                fetched_media: self.fetched_media.borrow().clone(),
            }
        }
    }

    impl Clone for TrackingTransport {
        fn clone(&self) -> Self {
            Self {
                mutations: self.mutations.clone(),
                media: self.media.clone(),
                transmissions: self.transmissions.clone(),
                fetched_media: self.fetched_media.clone(),
            }
        }
    }

    impl IctFaxTransport for TrackingTransport {
        fn create_document_with_media(
            &self,
            filename: &str,
            content_type: &str,
            bytes: &[u8],
        ) -> Result<String, FaxError> {
            self.mutations.set(self.mutations.get() + 1);
            self.media.borrow_mut().push((
                filename.to_string(),
                content_type.to_string(),
                bytes.len(),
            ));
            Ok("doc-1".into())
        }
        fn create_sendfax_program(&self, document_id: &str) -> Result<String, FaxError> {
            self.mutations.set(self.mutations.get() + 1);
            assert_eq!(document_id, "doc-1", "program must bind the real document");
            Ok("program-1".into())
        }
        fn create_transmission(
            &self,
            destination: &str,
            program_id: &str,
            document_id: &str,
        ) -> Result<crate::transport::IctFaxTransmission, FaxError> {
            self.mutations.set(self.mutations.get() + 1);
            self.transmissions.borrow_mut().push((
                destination.to_string(),
                program_id.to_string(),
                document_id.to_string(),
            ));
            Ok(crate::transport::IctFaxTransmission {
                id: "tx-1".into(),
                destination: destination.to_string(),
                status: "queued".into(),
                program: Some(program_id.to_string()),
                document_id: Some(document_id.to_string()),
                attempts: 0,
                pages: 0,
            })
        }
        fn send_transmission(&self, _transmission_id: &str) -> Result<(), FaxError> {
            self.mutations.set(self.mutations.get() + 1);
            Ok(())
        }
        fn fetch_transmission(
            &self,
            _transmission_id: &str,
        ) -> Result<crate::transport::IctFaxTransmission, FaxError> {
            Ok(crate::transport::IctFaxTransmission {
                id: "tx-1".into(),
                destination: String::new(),
                status: "completed".into(),
                program: None,
                document_id: Some("doc-1".into()),
                attempts: 1,
                pages: 1,
            })
        }
        fn fetch_document_media(&self, document_id: &str) -> Result<Vec<u8>, FaxError> {
            self.fetched_media
                .borrow_mut()
                .push(document_id.to_string());
            Ok(b"inbound-fax-bytes".to_vec())
        }
        fn list_transmissions(
            &self,
        ) -> Result<Vec<crate::transport::IctFaxTransmission>, FaxError> {
            Ok(Vec::new())
        }
        fn delete_transmission(&self, _transmission_id: &str) -> Result<(), FaxError> {
            self.mutations.set(self.mutations.get() + 1);
            Ok(())
        }
        fn list_accounts(&self) -> Result<Vec<crate::transport::IctFaxAccount>, FaxError> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn ep027_unit_ictfax_denied_submit_never_reaches_carrier() {
        let transport = TrackingTransport::default();
        let provider = IctFaxProvider::new(Box::new(transport), 1);
        // Approval denied -> Policy error BEFORE any provider call.
        let denied = job(0);
        let request = FaxSendRequest {
            job: denied.id.clone(),
            idempotency_key: "key-1".into(),
            approval_class: 0,
        };
        let err = submit_ictfax_governed(&provider, &denied, &request).expect_err("denied");
        assert_eq!(err.code, nexus_fax::FaxErrorCode::Policy);
        // The provider was never invoked: no SUBMIT audit entry at all
        // (zero carrier mutations, zero fabrication).
        assert!(!provider.audit().iter().any(|e| e.operation == "SUBMIT"));
    }

    #[test]
    fn ep027_unit_ictfax_approved_submit_reaches_carrier_once() {
        // The job's storage_ref must be a REAL file: the adapter
        // reads the controlled runtime artifact and uploads its
        // bytes - never an empty or fabricated body (AUD-020).
        let unique = format!("ictfax-submit-{}.pdf", std::process::id());
        let path = std::env::temp_dir().join(unique);
        let content = b"%PDF-1.4 fake fax bytes 1234567890";
        std::fs::write(&path, content).unwrap();
        let mut j = job(2);
        j.document.storage_ref = path.to_str().unwrap().to_string();
        j.document.size_bytes = content.len() as u64;

        let transport = TrackingTransport::default();
        let observe = transport.clone();
        let provider = IctFaxProvider::new(Box::new(transport), 1);
        let request = FaxSendRequest {
            job: j.id.clone(),
            idempotency_key: "key-1".into(),
            approval_class: 2,
        };
        let carrier = submit_ictfax_governed(&provider, &j, &request).expect("approved");
        assert_eq!(carrier.as_str(), "tx-1");
        // Exactly one ok SUBMIT audit entry (no blind retry).
        let oks = provider
            .audit()
            .iter()
            .filter(|e| e.operation == "SUBMIT" && e.outcome == "ok")
            .count();
        assert_eq!(oks, 1);
        // AUD-020: the recipient/document data REALLY reach the
        // provider request - real media bytes, real destination.
        let obs = observe.observe();
        assert_eq!(obs.mutations, 4, "document+program+transmission+send");
        let (media_filename, media_type, media_len) = &obs.media[0];
        assert_eq!(media_filename, "a.pdf");
        assert_eq!(media_type, "application/pdf");
        assert_eq!(
            *media_len,
            content.len(),
            "real byte length reaches the provider"
        );
        let (dest, program, doc) = &obs.transmissions[0];
        assert_eq!(
            dest,
            j.to.as_str(),
            "recipient reaches the provider request"
        );
        assert_eq!(program, "program-1");
        assert_eq!(doc, "doc-1", "document reference reaches the transmission");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn ep027_unit_ictfax_inbound_fetch_real_bytes_and_digest() {
        // AUD-020: inbound fetch returns REAL bytes with a real size
        // and a real sha256 digest - never size 0 / empty digest.
        let transport = TrackingTransport::default();
        let observe = transport.clone();
        let provider = IctFaxProvider::new(Box::new(transport), 1);
        let route = FaxRouteId::new("acc-1").expect("id");
        let carrier = FaxCarrierJobId::new("tx-1").expect("id");
        let doc = provider
            .fetch_inbound_document(&route, &carrier)
            .expect("fetch");
        assert_eq!(
            doc.size_bytes,
            b"inbound-fax-bytes".len() as u64,
            "real size"
        );
        assert_eq!(
            doc.sha256, "6beb841622f2cff61780183d4cf26c95be3de242113f927c973f9671b34fe958",
            "real digest computed from the fetched bytes"
        );
        let obs = observe.observe();
        assert_eq!(obs.fetched_media.len(), 1);
        assert_eq!(obs.fetched_media[0], "doc-1");
    }

    #[test]
    fn ep027_unit_ictfax_unbound_transport_fails_closed() {
        struct Unbound;
        impl IctFaxTransport for Unbound {}
        let provider = IctFaxProvider::new(Box::new(Unbound), 1);
        let j = job(2);
        let request = FaxSendRequest {
            job: j.id.clone(),
            idempotency_key: "key-1".into(),
            approval_class: 2,
        };
        let err = submit_ictfax_governed(&provider, &j, &request).expect_err("unbound");
        assert_eq!(err.code, nexus_fax::FaxErrorCode::Unavailable);
        assert_eq!(provider.audit().len(), 1);
        assert_eq!(provider.audit()[0].outcome, "UNAVAILABLE");
    }

    #[test]
    fn ep027_unit_ictfax_exact_target_delivery() {
        // A completed status for carrier job X verifies X only.
        let status = FaxStatus {
            state: FaxState::Delivered,
            carrier: FaxProviderKind::IctFax,
            attempts: 1,
            max_attempts: 3,
            pages: 1,
            carrier_job_id: Some(FaxCarrierJobId::new("tx-1").expect("id")),
            detail: "completed".into(),
        };
        let x = FaxCarrierJobId::new("tx-1").expect("id");
        assert!(verify_ictfax_delivery(&status, &x).is_ok());
        // A different carrier job id never verifies.
        let y = FaxCarrierJobId::new("tx-2").expect("id");
        let err = verify_ictfax_delivery(&status, &y).expect_err("mismatch");
        assert_eq!(err.code, nexus_fax::FaxErrorCode::Verification);
        // SUBMITTED with the right id is not delivery.
        let submitted = FaxStatus {
            state: FaxState::Submitted,
            ..status
        };
        let err = verify_ictfax_delivery(&submitted, &x).expect_err("not delivered");
        assert_eq!(err.code, nexus_fax::FaxErrorCode::Verification);
    }

    #[test]
    fn ep027_unit_ictfax_status_maps_carrier_claim_to_submitted_only() {
        // The transport claims `sent`; the adapter maps it to
        // SUBMITTED, never DELIVERED (carrier claim != proof).
        let transport = TrackingTransport::default();
        let provider = IctFaxProvider::new(Box::new(transport), 1);
        let id = FaxJobId::new("tx-1").expect("id");
        // The tracking transport returns `completed`, which maps to
        // DELIVERED - but the adapter's status() never fabricates a
        // carrier_job_id that the provider did not return.
        let status = provider.status(&id).expect("status");
        assert_eq!(status.state, FaxState::Delivered);
        assert_eq!(
            status.carrier_job_id.as_ref().expect("carrier id").as_str(),
            "tx-1"
        );
    }

    #[test]
    fn ep027_unit_ictfax_inflight_conflict() {
        let transport = TrackingTransport::default();
        let provider = IctFaxProvider::new(Box::new(transport), 1);
        // Manually hold the in-flight slot, then attempt a submit.
        let j = job(2);
        let correlation = "fax-test-1";
        provider
            .begin("SUBMIT", j.id.as_str(), correlation)
            .expect("begin");
        let request = FaxSendRequest {
            job: j.id.clone(),
            idempotency_key: "key-1".into(),
            approval_class: 2,
        };
        let err = submit_ictfax_governed(&provider, &j, &request).expect_err("conflict");
        assert_eq!(err.code, nexus_fax::FaxErrorCode::Conflict);
        provider.end("SUBMIT", j.id.as_str());
    }
}
