//! EP-007 M5 live-fire harness: validate a REAL Keycloak token through the
//! production verification boundary.
//!
//! Reads a JWT, the real JWKS document, and the expected issuer/audience
//! from argv, runs the production `nexus_keycloak::verify` path, and prints
//! a machine-readable JSON verdict. Used by the M5 passkey live-fire to
//! prove that a token minted by the real Keycloak ceremony is accepted by
//! the production validator (directive B step 12), and that tampered or
//! wrong-issuer tokens are rejected.
//!
//! Usage:
//!   verify_access_token <token> <jwks-file> <issuer> <audience> <strength> <principal> <correlation>
//!
//! Prints one JSON object to stdout:
//!   {"accepted": true, "subject": "...", "issuer": "...", "audiences": [...],
//!    "scopes": [...], "strength": "..."}
//! or {"accepted": false, "reason": "..."}

use std::env;
use std::process::ExitCode;

use nexus_auth::TokenValidator;
use nexus_domain::{CorrelationId, PrincipalType};
use nexus_keycloak::verify_and_validate;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 7 {
        eprintln!(
            "usage: verify_access_token <token> <jwks-file> <issuer> <audience> <strength> <principal> <correlation>"
        );
        return ExitCode::from(2);
    }

    let token = &args[0];
    let jwks_path = &args[1];
    let issuer = &args[2];
    let audience = &args[3];
    let strength = match args[4].as_str() {
        "NONE" => nexus_auth::AuthenticationStrength::None,
        "SINGLE_FACTOR" => nexus_auth::AuthenticationStrength::SingleFactor,
        "MULTI_FACTOR" => nexus_auth::AuthenticationStrength::MultiFactor,
        "STEP_UP" => nexus_auth::AuthenticationStrength::StepUp,
        other => {
            println!("{{\"accepted\": false, \"reason\": \"unknown strength {other}\"}}");
            return ExitCode::from(1);
        }
    };
    let principal = match args[5].as_str() {
        "HUMAN" => PrincipalType::Human,
        "SERVICE" => PrincipalType::Service,
        other => {
            println!("{{\"accepted\": false, \"reason\": \"unknown principal {other}\"}}");
            return ExitCode::from(1);
        }
    };
    let correlation = match CorrelationId::new(&args[6]) {
        Ok(c) => c,
        Err(_) => {
            println!("{{\"accepted\": false, \"reason\": \"invalid correlation id\"}}");
            return ExitCode::from(1);
        }
    };

    let jwks_json = match std::fs::read_to_string(jwks_path) {
        Ok(j) => j,
        Err(e) => {
            println!("{{\"accepted\": false, \"reason\": \"jwks read: {e}\"}}");
            return ExitCode::from(1);
        }
    };

    let validator = match TokenValidator::new(
        issuer.clone(),
        audience.clone(),
        vec!["openid".to_string(), "nexus.read".to_string()],
        None,
    ) {
        Ok(v) => v,
        Err(e) => {
            println!("{{\"accepted\": false, \"reason\": \"validator: {e}\"}}");
            return ExitCode::from(1);
        }
    };

    match verify_and_validate(
        token,
        &jwks_json,
        &validator,
        correlation,
        strength,
        principal,
    ) {
        Ok(validated) => {
            println!(
                "{{\"accepted\": true, \"subject\": \"{}\", \"issuer\": \"{}\", \
                 \"audiences\": {:?}, \"scopes\": {:?}, \"strength\": \"{}\"}}",
                validated.subject,
                validated.issuer,
                validated.audiences,
                validated.scopes,
                validated.strength.as_str(),
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("{{\"accepted\": false, \"reason\": \"{e}\"}}");
            ExitCode::from(1)
        }
    }
}
