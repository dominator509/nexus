//! EP-009 M4 PKI failure probe: real fail-closed behavior.
//!
//! Drives the REAL `OpenBaoPkiAuthority` adapter against failure
//! scenarios and asserts the typed error code. The mode is selected by
//! `NEXUS_PKI_MODE`:
//!
//! - `expect-unavailable`: provider unreachable -> PKI_UNAVAILABLE
//! - `expect-permission-denied`: token denied on the role -> PKI_PERMISSION_DENIED
//! - `expect-csr-rejected`: malformed CSR -> PKI_CSR_REJECTED
//! - `expect-role-violation`: identity outside role constraints -> PKI_ROLE_VIOLATION / PKI_CSR_REJECTED
//! - `expect-ttl-violation`: TTL beyond role policy -> PKI_TTL_VIOLATION
//! - `expect-malformed-response`: provider returns non-JSON -> PKI_MALFORMED_RESPONSE
//! - `expect-crl-unavailable`: revocation provider unreachable -> PKI_UNAVAILABLE
//!
//! The probe exits 0 ONLY when the expected typed code is observed; any
//! fabricated success fails the run.

use nexus_trust::vocabulary::TrustZone;
use nexus_trust::ServiceIdentity;

use nexus_pki::ca::OpenBaoPkiAuthority;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn read_token() -> String {
    let path = std::env::var("NEXUS_PKI_TOKEN_FILE").expect("NEXUS_PKI_TOKEN_FILE must be set");
    let content = std::fs::read_to_string(&path).expect("read token file");
    let token = content.lines().next().unwrap_or("").trim().to_string();
    assert!(!token.is_empty(), "token file must not be empty");
    token
}

fn read_ca() -> String {
    let path = std::env::var("NEXUS_PKI_CA_FILE").expect("NEXUS_PKI_CA_FILE must be set");
    std::fs::read_to_string(&path).expect("read ca file")
}

fn main() {
    let mode = env_or("NEXUS_PKI_MODE", "expect-unavailable");
    let base = env_or("NEXUS_PKI_ADDR", "http://127.0.0.1:8200");
    let mount = env_or("NEXUS_PKI_MOUNT", "pki");
    let role = env_or("NEXUS_PKI_ROLE", "nexus-service");
    let token = read_token();
    let ca_pem = read_ca();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let authority = OpenBaoPkiAuthority::with_token(&base, &token, &ca_pem)
        .expect("construct authority")
        .with_mount_role(&mount, &role);

    let tenant = env_or("NEXUS_PKI_TENANT", "tenant-livefire");
    let identity_id = env_or("NEXUS_PKI_IDENTITY", "svc-alpha");
    let identity =
        ServiceIdentity::new(&identity_id, &tenant, &identity_id, TrustZone::PrivateMesh)
            .expect("identity");

    let result = match mode.as_str() {
        "expect-unavailable" => {
            let r = authority.issue_leaf(&identity, now, 3600);
            match r {
                Err(e) if e.code == nexus_trust::TrustErrorCode::Unavailable => {
                    println!("PKI_UNAVAILABLE");
                    Ok(())
                }
                other => Err(format!("expected unavailable, got {:?}", other.map(|_| ()))),
            }
        }
        "expect-permission-denied" => {
            let r = authority.issue_leaf(&identity, now, 3600);
            match r {
                Err(e) if e.code == nexus_trust::TrustErrorCode::ProviderAuthorization => {
                    println!("PKI_PERMISSION_DENIED");
                    Ok(())
                }
                other => Err(format!(
                    "expected permission denied, got {:?}",
                    other.map(|_| ())
                )),
            }
        }
        "expect-csr-rejected" => {
            let csr = std::env::var("NEXUS_PKI_CSR").expect("NEXUS_PKI_CSR must be set");
            let r = authority.sign_csr_raw(&csr, &identity_id, now, 3600);
            match r {
                Err(e)
                    if e.code == nexus_trust::TrustErrorCode::InvalidReference
                        || e.code == nexus_trust::TrustErrorCode::MalformedProviderResponse =>
                {
                    println!("PKI_CSR_REJECTED");
                    Ok(())
                }
                other => Err(format!(
                    "expected csr rejection, got {:?}",
                    other.map(|_| ())
                )),
            }
        }
        "expect-role-violation" => {
            let r = authority.issue_leaf(&identity, now, 3600);
            match r {
                Err(e)
                    if e.code == nexus_trust::TrustErrorCode::InvalidReference
                        || e.code == nexus_trust::TrustErrorCode::ProviderAuthorization =>
                {
                    println!("PKI_ROLE_VIOLATION");
                    Ok(())
                }
                other => Err(format!(
                    "expected role violation, got {:?}",
                    other.map(|_| ())
                )),
            }
        }
        "expect-ttl-violation" => {
            let r = authority.issue_leaf(&identity, now, 48 * 3600);
            match r {
                Err(e) if e.code == nexus_trust::TrustErrorCode::InvalidReference => {
                    println!("PKI_TTL_VIOLATION");
                    Ok(())
                }
                other => Err(format!(
                    "expected ttl violation, got {:?}",
                    other.map(|_| ())
                )),
            }
        }
        "expect-malformed-response" => {
            // The provider is a garbage HTTP server returning non-JSON.
            let r = authority.issue_leaf(&identity, now, 3600);
            match r {
                Err(e) if e.code == nexus_trust::TrustErrorCode::MalformedProviderResponse => {
                    println!("PKI_MALFORMED_RESPONSE");
                    Ok(())
                }
                other => Err(format!(
                    "expected malformed response, got {:?}",
                    other.map(|_| ())
                )),
            }
        }
        "expect-crl-unavailable" => {
            let r = authority.crl_der();
            match r {
                Err(e) if e.code == nexus_trust::TrustErrorCode::Unavailable => {
                    println!("PKI_UNAVAILABLE");
                    Ok(())
                }
                other => Err(format!(
                    "expected crl unavailable, got {:?}",
                    other.map(|_| ())
                )),
            }
        }
        other => Err(format!("unknown mode {}", other)),
    };

    match result {
        Ok(()) => println!("EP-009 M4 pki failure probe: ok"),
        Err(msg) => {
            eprintln!("probe failed: {}", msg);
            std::process::exit(1);
        }
    }
}
