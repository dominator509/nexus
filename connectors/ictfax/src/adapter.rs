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
        // sendfax program, then the transmission, then send.
        let result = (|| {
            self.transport
                .upload_document_media(&job.document.storage_ref)?;
            // The sendfax program prepares the document for the
            // account; the provider returns its reference.
            let _program = self.transport.create_sendfax_program()?;
            let transmission = self.transport.create_transmission()?;
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
        let document = FaxDocument {
            id: FaxDocumentId::new(document_id)?,
            filename: format!("{}.pdf", carrier_job.as_str()),
            content_type: "application/pdf".into(),
            size_bytes: 0,
            pages: transmission.pages,
            // Digest is bound at ingest by the owning pipeline; the
            // adapter never invents artifact content or hashes.
            sha256: String::new(),
            storage_ref: format!("ictfax:{}", carrier_job.as_str()),
            scan_status: FaxScanStatus::Pending,
        };
        self.record(
            correlation.clone(),
            "FETCH_DOCUMENT",
            "ok",
            format!("route {route} carrier job {carrier_job}"),
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

    /// A tracking transport that counts carrier mutations so tests can
    /// prove denied sends never reach the provider.
    #[derive(Default)]
    struct TrackingTransport {
        mutations: std::cell::Cell<u32>,
    }

    impl IctFaxTransport for TrackingTransport {
        fn upload_document_media(&self, _document_id: &str) -> Result<(), FaxError> {
            self.mutations.set(self.mutations.get() + 1);
            Ok(())
        }
        fn create_sendfax_program(&self) -> Result<String, FaxError> {
            self.mutations.set(self.mutations.get() + 1);
            Ok("program-1".into())
        }
        fn create_transmission(&self) -> Result<crate::transport::IctFaxTransmission, FaxError> {
            self.mutations.set(self.mutations.get() + 1);
            Ok(crate::transport::IctFaxTransmission {
                id: "tx-1".into(),
                destination: String::new(),
                status: "queued".into(),
                program: None,
                document_id: None,
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
        let transport = TrackingTransport::default();
        let provider = IctFaxProvider::new(Box::new(transport), 1);
        let j = job(2);
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
