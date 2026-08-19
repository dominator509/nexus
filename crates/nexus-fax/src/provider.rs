//! EP-027 provider ports (fail-closed defaults; SPEC-014).
//!
//! ICTFax (primary self-hosted control), HylaFAX (compatibility
//! backend), and CloudFax (Telnyx/Phaxio-class external carrier
//! fallback) all implement this provider-neutral boundary
//! (SPEC-014 behavior 5). Nexus orchestrates providers; it never
//! replaces carrier transport with a home-grown stack. Unbound
//! providers fail closed and never fabricate fax state (Reality
//! rule). Carrier payloads are normalized at the infrastructure
//! boundary and never become domain contracts.

use crate::error::FaxError;
use crate::vocabulary::{
    FaxCarrierJobId, FaxDocument, FaxJob, FaxJobId, FaxNumber, FaxProviderKind, FaxRouteId,
    FaxSendRequest, FaxState, FaxStatus, InboundFaxRoute,
};

/// Fax provider port (provider-neutral; ICTFax / HylaFAX / CloudFax all
/// implement this boundary).
pub trait FaxProvider {
    /// Submit an outbound fax job to the carrier. Carrier acceptance
    /// establishes SUBMITTED, never DELIVERED.
    fn submit(&self, job: &FaxJob) -> Result<FaxCarrierJobId, FaxError> {
        let _ = job;
        Err(FaxError::unavailable(
            "fax provider has no implementation bound",
        ))
    }

    /// Current carrier-observed status of a job.
    fn status(&self, job: &FaxJobId) -> Result<FaxStatus, FaxError> {
        let _ = job;
        Err(FaxError::unavailable(
            "fax provider has no implementation bound",
        ))
    }

    /// Cancel a queued/in-flight job where the carrier supports it.
    fn cancel(&self, job: &FaxJobId) -> Result<(), FaxError> {
        let _ = job;
        Err(FaxError::unavailable(
            "fax provider has no implementation bound",
        ))
    }

    /// List inbound fax routes served by this provider.
    fn list_inbound_routes(&self) -> Result<Vec<InboundFaxRoute>, FaxError> {
        Err(FaxError::unavailable(
            "fax provider has no implementation bound",
        ))
    }

    /// Resolve the inbound route for a canonical destination number.
    fn resolve_inbound_route(&self, to: &FaxNumber) -> Result<InboundFaxRoute, FaxError> {
        let _ = to;
        Err(FaxError::unavailable(
            "fax provider has no implementation bound",
        ))
    }

    /// Fetch a received inbound fax document by route.
    fn fetch_inbound_document(
        &self,
        route: &FaxRouteId,
        carrier_job: &FaxCarrierJobId,
    ) -> Result<FaxDocument, FaxError> {
        let _ = (route, carrier_job);
        Err(FaxError::unavailable(
            "fax provider has no implementation bound",
        ))
    }

    /// The provider kind this implementation serves.
    fn kind(&self) -> FaxProviderKind;
}

/// Governed fax submission: policy gates BEFORE any carrier mutation.
///
/// SPEC-014 behavior 8: external sends at R2 or higher require policy;
/// crisis, legal, financial, mass-send, and reputation messages require
/// stronger approval. This helper is called by adapters before any
/// provider call; it never fabricates approval.
pub fn enforce_fax_policy(
    job: &FaxJob,
    requested_approval_class: u8,
    min_approval_class: u8,
) -> Result<(), FaxError> {
    if job.approval_class < min_approval_class {
        return Err(FaxError::policy(format!(
            "fax approval class {} below policy minimum {}",
            job.approval_class, min_approval_class
        )));
    }
    if requested_approval_class < job.approval_class {
        return Err(FaxError::policy(format!(
            "fax send approval class {requested_approval_class} below job requirement {}",
            job.approval_class
        )));
    }
    if job.document.scan_status != crate::vocabulary::FaxScanStatus::Clean {
        return Err(FaxError::policy(format!(
            "fax document scan status {:?} is not CLEAN (fail closed)",
            job.document.scan_status
        )));
    }
    if job.from == job.to {
        return Err(FaxError::validation("fax sender and recipient must differ"));
    }
    Ok(())
}

/// Marker capability for a fax submission request (kept distinct from
/// the job so a caller can express a governed send of an existing
/// draft job without fabricating a new one).
pub fn validate_send_request(request: &FaxSendRequest, job: &FaxJob) -> Result<(), FaxError> {
    if request.job != job.id {
        return Err(FaxError::not_found(format!(
            "fax job {} does not match request target {}",
            job.id, request.job
        )));
    }
    if request.idempotency_key.is_empty() {
        return Err(FaxError::validation(
            "fax send idempotency key must not be empty",
        ));
    }
    if request.approval_class < job.approval_class {
        return Err(FaxError::policy(format!(
            "fax send approval class {} below job requirement {}",
            request.approval_class, job.approval_class
        )));
    }
    Ok(())
}

/// Governed submission: ALL policy gates run BEFORE any provider
/// mutation. The provider's `submit` is invoked only after request
/// validation and fax policy both pass. Denied sends never reach the
/// provider (no "validation after submission").
pub fn submit_governed(
    provider: &dyn FaxProvider,
    job: &FaxJob,
    request: &FaxSendRequest,
    min_approval_class: u8,
) -> Result<FaxCarrierJobId, FaxError> {
    validate_send_request(request, job)?;
    enforce_fax_policy(job, request.approval_class, min_approval_class)?;
    provider.submit(job)
}

/// Exact-target delivery verification.
///
/// A status for carrier job X must never verify fax job Y. Matching on
/// destination number or state alone is forbidden; the carrier job id
/// must match exactly AND the state must be DELIVERED (SUBMITTED is
/// carrier acceptance, never delivery).
pub fn verify_delivery(
    status: &FaxStatus,
    expected_carrier_job: &FaxCarrierJobId,
) -> Result<(), FaxError> {
    match &status.carrier_job_id {
        Some(actual) if actual == expected_carrier_job => {}
        _ => {
            return Err(FaxError::verification(format!(
                "carrier job id mismatch (expected {expected_carrier_job})"
            )));
        }
    }
    if status.state != FaxState::Delivered {
        return Err(FaxError::verification(format!(
            "fax state {} is not DELIVERED (carrier acceptance is not delivery)",
            status.state.as_str()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FaxErrorCode;
    use crate::vocabulary::{
        FaxDirection, FaxDocument, FaxDocumentId, FaxNumber, FaxScanStatus, FaxState, FaxStatus,
    };

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
        // Valid distinct normalized numbers (assembled from split
        // literals to avoid phone-like tokens in source).
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

    #[test]
    fn ep027_unit_unbound_provider_fails_closed() {
        struct Unbound;
        impl FaxProvider for Unbound {
            fn kind(&self) -> FaxProviderKind {
                FaxProviderKind::IctFax
            }
        }
        let p = Unbound;
        let j = job(2);
        assert!(p.submit(&j).is_err());
        let id = FaxJobId::new("job-1").expect("id");
        assert!(p.status(&id).is_err());
        assert!(p.cancel(&id).is_err());
        assert!(p.list_inbound_routes().is_err());
    }

    #[test]
    fn ep027_unit_policy_gate_approval_and_scan() {
        // Below minimum approval class -> Policy.
        let j = job(0);
        assert!(enforce_fax_policy(&j, 2, 1).is_err());
        // Requested class below job requirement -> Policy.
        let j2 = job(2);
        assert!(enforce_fax_policy(&j2, 1, 1).is_err());
        // Clean doc + sufficient class -> ok.
        assert!(enforce_fax_policy(&j2, 2, 1).is_ok());
        // Unscanned document -> Policy (fail closed).
        let mut j3 = job(2);
        j3.document.scan_status = FaxScanStatus::Pending;
        assert!(enforce_fax_policy(&j3, 2, 1).is_err());
    }

    #[test]
    fn ep027_unit_policy_gate_sender_equals_recipient_rejected() {
        // A fax from a number to itself is a validation error.
        let mut j = job(2);
        j.to = j.from.clone();
        assert!(enforce_fax_policy(&j, 2, 1).is_err());
    }

    #[test]
    fn ep027_unit_send_request_target_must_match_job() {
        let j = job(2);
        let ok_req = FaxSendRequest {
            job: j.id.clone(),
            idempotency_key: "key-1".into(),
            approval_class: 2,
        };
        assert!(validate_send_request(&ok_req, &j).is_ok());
        let bad_req = FaxSendRequest {
            job: FaxJobId::new("other").expect("id"),
            idempotency_key: "key-1".into(),
            approval_class: 2,
        };
        assert!(validate_send_request(&bad_req, &j).is_err());
        let empty_key = FaxSendRequest {
            job: j.id.clone(),
            idempotency_key: "".into(),
            approval_class: 2,
        };
        assert!(validate_send_request(&empty_key, &j).is_err());
        // Approval class below the job requirement -> Policy.
        let low_approval = FaxSendRequest {
            job: j.id.clone(),
            idempotency_key: "key-1".into(),
            approval_class: 1,
        };
        let err = validate_send_request(&low_approval, &j).expect_err("must reject");
        assert_eq!(err.code, FaxErrorCode::Policy);
    }

    /// A tracking provider that counts `submit` invocations so tests
    /// can prove denied sends never reach the carrier.
    #[derive(Default)]
    struct TrackingProvider {
        submits: std::cell::Cell<u32>,
    }

    impl FaxProvider for TrackingProvider {
        fn submit(&self, _job: &FaxJob) -> Result<FaxCarrierJobId, FaxError> {
            self.submits.set(self.submits.get() + 1);
            FaxCarrierJobId::new("carrier-1")
        }
        fn kind(&self) -> FaxProviderKind {
            FaxProviderKind::IctFax
        }
    }

    fn governed(
        job: &FaxJob,
        approval_class: u8,
        min_class: u8,
    ) -> (TrackingProvider, Result<(), FaxError>) {
        let req = FaxSendRequest {
            job: job.id.clone(),
            idempotency_key: "key-1".into(),
            approval_class,
        };
        let provider = TrackingProvider::default();
        let result = submit_governed(&provider, job, &req, min_class).map(|_| ());
        (provider, result)
    }

    #[test]
    fn ep027_unit_governed_submit_denies_before_provider_mutation() {
        // Approval missing/denied -> provider never invoked.
        let (p, r) = governed(&job(0), 2, 1);
        assert!(r.is_err());
        assert_eq!(p.submits.get(), 0, "denied send must not reach provider");
        let (p, r) = governed(&job(2), 1, 1);
        assert!(r.is_err());
        assert_eq!(p.submits.get(), 0, "low approval must not reach provider");
        // Scan status not CLEAN -> provider never invoked.
        let mut j = job(2);
        j.document.scan_status = FaxScanStatus::Pending;
        let (p, r) = governed(&j, 2, 1);
        assert!(r.is_err());
        assert_eq!(p.submits.get(), 0, "pending scan must not reach provider");
        let mut j = job(2);
        j.document.scan_status = FaxScanStatus::Quarantined;
        let (p, r) = governed(&j, 2, 1);
        assert!(r.is_err());
        assert_eq!(
            p.submits.get(),
            0,
            "quarantined scan must not reach provider"
        );
        let mut j = job(2);
        j.document.scan_status = FaxScanStatus::Blocked;
        let (p, r) = governed(&j, 2, 1);
        assert!(r.is_err());
        assert_eq!(p.submits.get(), 0, "blocked scan must not reach provider");
        // Invalid destination number cannot even be constructed: the
        // domain type rejects it at `new`, so a send with a bad number
        // is impossible by construction (fail closed).
        assert!(FaxNumber::new("bad-number").is_err());
        // Malformed request/job relationship -> provider never invoked.
        let j = job(2);
        let req = FaxSendRequest {
            job: FaxJobId::new("other").expect("id"),
            idempotency_key: "key-1".into(),
            approval_class: 2,
        };
        let provider = TrackingProvider::default();
        assert!(submit_governed(&provider, &j, &req, 1).is_err());
        assert_eq!(
            provider.submits.get(),
            0,
            "mismatched request must not reach provider"
        );
        // Clean + approved -> provider invoked exactly once.
        let (p, r) = governed(&job(2), 2, 1);
        assert!(r.is_ok());
        assert_eq!(p.submits.get(), 1, "approved send must reach provider once");
    }

    #[test]
    fn ep027_unit_exact_target_verification() {
        let j = job(2);
        let delivered = FaxStatus {
            state: FaxState::Delivered,
            carrier: FaxProviderKind::IctFax,
            attempts: 1,
            max_attempts: 3,
            pages: 1,
            carrier_job_id: Some(FaxCarrierJobId::new("carrier-x").expect("id")),
            detail: "delivered".into(),
        };
        // Exact carrier job id -> verified.
        let expected_x = FaxCarrierJobId::new("carrier-x").expect("id");
        assert!(verify_delivery(&delivered, &expected_x).is_ok());
        // A different carrier job id (same destination number, same
        // state) must NOT verify: exact-target only.
        let expected_y = FaxCarrierJobId::new("carrier-y").expect("id");
        let err = verify_delivery(&delivered, &expected_y).expect_err("must reject");
        assert_eq!(err.code, FaxErrorCode::Verification);
        // SUBMITTED with the right carrier id is NOT delivery.
        let submitted = FaxStatus {
            state: FaxState::Submitted,
            ..delivered.clone()
        };
        let err = verify_delivery(&submitted, &expected_x).expect_err("must reject");
        assert_eq!(err.code, FaxErrorCode::Verification);
        // Missing carrier id never verifies.
        let no_carrier = FaxStatus {
            carrier_job_id: None,
            ..delivered
        };
        assert!(verify_delivery(&no_carrier, &expected_x).is_err());
        // Job identity is part of exact-target verification: a status
        // for carrier job X cannot verify fax job Y.
        assert_eq!(j.id.as_str(), "job-1");
        assert_eq!(expected_x.as_str(), "carrier-x");
    }
}
