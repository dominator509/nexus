//! Canonical service identity binding (EP-009 M4 directive C, ADR-014).
//!
//! Every certificate issued by the Nexus CA carries a deterministic
//! URI SAN derived from the `nexus-trust` `ServiceIdentity` contract:
//!
//! ```text
//! nexus://tenant/<tenant_id>/service/<identity_id>
//! ```
//!
//! This is the SINGLE canonical identity namespace. mTLS additionally
//! carries a deterministic transport DNS SAN
//! (`<identity_id>.<tenant_id>.svc.nexus.internal`) so rustls performs
//! standard hostname/SAN verification (directive H: never disable it);
//! the Nexus identity layer binds the authenticated peer to the
//! canonical URI SAN and rejects mismatch.

use nexus_trust::vocabulary::TrustZone;
use nexus_trust::{ServiceIdentity, TrustError};

/// Canonical identity URI namespace prefix.
pub const CANONICAL_URI_PREFIX: &str = "nexus://tenant/";
/// Transport DNS suffix for mTLS hostname verification.
pub const TRANSPORT_DNS_SUFFIX: &str = ".svc.nexus.internal";

/// Build the canonical URI SAN for a service identity.
///
/// Deterministic: `nexus://tenant/<tenant_id>/service/<identity_id>`.
pub fn canonical_service_uri(tenant_id: &str, identity_id: &str) -> String {
    format!("nexus://tenant/{}/service/{}", tenant_id, identity_id)
}

/// Build the deterministic transport DNS SAN for a service identity.
///
/// Used ONLY for rustls ServerName verification; the canonical identity
/// is the URI SAN above. Derivation is deterministic and reversible
/// from the same `ServiceIdentity` fields.
pub fn transport_dns_name(tenant_id: &str, identity_id: &str) -> String {
    // DNS labels are case-insensitive; lower-case both components and
    // replace colons (UUIDs never contain them, but be defensive).
    let t = tenant_id.to_ascii_lowercase().replace(':', "-");
    let i = identity_id.to_ascii_lowercase().replace(':', "-");
    format!("{}.{}{}", i, t, TRANSPORT_DNS_SUFFIX)
}

/// Parse a canonical `nexus://tenant/<tenant>/service/<identity>` URI.
///
/// Returns `(tenant_id, identity_id)` on success.
pub fn parse_canonical_uri(uri: &str) -> Result<(String, String), TrustError> {
    let rest = uri.strip_prefix(CANONICAL_URI_PREFIX).ok_or_else(|| {
        TrustError::invalid("certificate identity uri is not in the canonical namespace")
    })?;
    let (tenant, service) = rest
        .split_once("/service/")
        .ok_or_else(|| TrustError::invalid("certificate identity uri missing /service/ segment"))?;
    if tenant.is_empty() || service.is_empty() {
        return Err(TrustError::invalid(
            "certificate identity uri has an empty tenant or identity segment",
        ));
    }
    if tenant.contains('/') || service.contains('/') {
        return Err(TrustError::invalid(
            "certificate identity uri has unexpected extra segments",
        ));
    }
    Ok((tenant.to_string(), service.to_string()))
}

/// Identity binding extracted from a peer certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceIdentityBinding {
    /// Tenant segment of the canonical URI SAN.
    pub tenant_id: String,
    /// Identity segment of the canonical URI SAN.
    pub identity_id: String,
    /// Canonical URI SAN as found on the certificate.
    pub uri_san: String,
    /// Transport DNS SAN as found on the certificate.
    pub dns_san: Option<String>,
    /// Trust zone recorded by the issuing CA role.
    pub zone: TrustZone,
}

impl ServiceIdentityBinding {
    /// Whether this binding matches the expected logical service
    /// identity (identity id + tenant). The URI SAN is authoritative;
    /// the DNS SAN must agree when present (directive H.4/H.5).
    pub fn matches(&self, identity: &ServiceIdentity) -> bool {
        if self.tenant_id != identity.tenant_id || self.identity_id != identity.identity_id {
            return false;
        }
        let expected_dns = transport_dns_name(&identity.tenant_id, &identity.identity_id);
        match &self.dns_san {
            Some(dns) => dns == &expected_dns,
            None => true, // URI SAN is authoritative; DNS optional at parse time
        }
    }
}

/// Extract the canonical identity binding from a DER certificate.
///
/// Uses x509-parser (directive T: no custom crypto, parsing glue only).
pub fn parse_certificate_identity(
    der: &[u8],
) -> Result<ServiceIdentityBinding, nexus_trust::TrustError> {
    use x509_parser::prelude::{FromDer, X509Certificate};
    let (_, cert) = X509Certificate::from_der(der)
        .map_err(|e| TrustError::invalid(format!("cannot parse certificate: {}", e)))?;

    let mut uri_san: Option<String> = None;
    let mut dns_san: Option<String> = None;
    if let Ok(Some(ext)) = cert.subject_alternative_name() {
        for name in &ext.value.general_names {
            match name {
                x509_parser::extensions::GeneralName::URI(u) if uri_san.is_none() => {
                    uri_san = Some(u.to_string());
                }
                x509_parser::extensions::GeneralName::DNSName(d) if dns_san.is_none() => {
                    dns_san = Some(d.to_string());
                }
                _ => {}
            }
        }
    }

    let uri =
        uri_san.ok_or_else(|| TrustError::invalid("certificate has no canonical nexus URI SAN"))?;
    let (tenant_id, identity_id) = parse_canonical_uri(&uri)?;

    Ok(ServiceIdentityBinding {
        tenant_id,
        identity_id,
        uri_san: uri,
        dns_san,
        zone: TrustZone::PrivateMesh,
    })
}

/// Serial number of a DER certificate as a normalized hex string
/// (lower-case, no colons) for fingerprinting and revocation matching.
pub fn certificate_serial_hex(der: &[u8]) -> Result<String, nexus_trust::TrustError> {
    use x509_parser::prelude::{FromDer, X509Certificate};
    let (_, cert) = X509Certificate::from_der(der)
        .map_err(|e| TrustError::invalid(format!("cannot parse certificate: {}", e)))?;
    Ok(cert
        .raw_serial_as_string()
        .replace(':', "")
        .to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep009_unit_pki_canonical_uri_roundtrip() {
        let uri = canonical_service_uri("tenant-a", "svc-core");
        assert_eq!(uri, "nexus://tenant/tenant-a/service/svc-core");
        let (t, i) = parse_canonical_uri(&uri).unwrap();
        assert_eq!((t.as_str(), i.as_str()), ("tenant-a", "svc-core"));
    }

    #[test]
    fn ep009_unit_pki_canonical_uri_rejects_non_canonical() {
        assert!(parse_canonical_uri("spiffe://tenant-a/svc-core").is_err());
        assert!(parse_canonical_uri("nexus://tenant//service/svc-core").is_err());
        assert!(parse_canonical_uri("nexus://tenant/a/service/").is_err());
        assert!(parse_canonical_uri("nexus://tenant/a/service/x/y").is_err());
    }

    #[test]
    fn ep009_unit_pki_transport_dns_is_deterministic() {
        let a = transport_dns_name("tenant-a", "svc-core");
        let b = transport_dns_name("tenant-a", "svc-core");
        assert_eq!(a, b);
        assert!(a.ends_with(".svc.nexus.internal"));
        assert!(a.starts_with("svc-core.tenant-a"));
    }
}
