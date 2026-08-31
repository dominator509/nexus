//! evidence_sign: real Ed25519 evidence seal adapter (RX-009; AUD-059).
//!
//! Invoked by scripts/sbom/generate.sh and scripts/sbom/verify.sh with:
//!
//!   evidence_sign sign <evidence.json> <pkcs8_der_file> <sig_out> <pub_out>
//!   evidence_sign verify <evidence.json> <pub_file> <sig_file>
//!
//! The SHA-256 checksum-only seal is insufficient: anyone able to change
//! evidence can change its checksum. This adapter replaces the seal with
//! a REAL Ed25519 signature over the canonical evidence digest:
//! - `sign` reads the private key (PKCS#8 v2 DER), signs the evidence
//!   file bytes' sha256 digest via the real ArtifactSigner, and writes
//!   the raw 64-byte signature and the 32-byte public key.
//! - `verify` verifies the signature with ONLY the public key (fail
//!   closed: any tamper, wrong key, or malformed signature exits non-zero
//!   with a typed message).
//!
//! The private key is never written next to the evidence by this
//! adapter; callers keep it out of band (env/file outside the evidence
//! dir) so an attacker who rewrites evidence cannot re-seal it.

use std::path::Path;

use nexus_supply_chain::model::ArtifactDigest;
use nexus_supply_chain::signer::Ed25519ArtifactSigner;

fn sha256_hex(bytes: &[u8]) -> String {
    use ring::digest::{Context, SHA256};
    let mut ctx = Context::new(&SHA256);
    ctx.update(bytes);
    let out = ctx.finish();
    let mut s = String::with_capacity(64);
    for b in out.as_ref() {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn evidence_digest(path: &Path) -> Result<ArtifactDigest, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read evidence: {e}"))?;
    Ok(ArtifactDigest {
        algorithm: "sha256".to_string(),
        hex: sha256_hex(&bytes),
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: evidence_sign keygen <pkcs8_der_out> <pub_out> | sign <evidence.json> <pkcs8_der_file> <sig_out> <pub_out> | verify <evidence.json> <pub_file> <sig_file>");
        std::process::exit(2);
    }
    match args[1].as_str() {
        "keygen" => {
            if args.len() != 4 {
                eprintln!("keygen requires 2 arguments: keygen <pkcs8_der_out> <pub_out>");
                std::process::exit(2);
            }
            let key_out = Path::new(&args[2]);
            let pub_out = Path::new(&args[3]);
            let signer = match Ed25519ArtifactSigner::generate() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("evidence_sign: FAIL - keygen failed: {e}");
                    std::process::exit(1);
                }
            };
            let der = signer.to_pkcs8_der();
            let pubkey = signer.public_key();
            if let Err(e) = std::fs::write(key_out, &der) {
                eprintln!("evidence_sign: FAIL - write private key: {e}");
                std::process::exit(1);
            }
            if let Err(e) = std::fs::write(pub_out, &pubkey) {
                eprintln!("evidence_sign: FAIL - write public key: {e}");
                std::process::exit(1);
            }
            println!(
                "evidence_sign: keygen ok priv={} bytes pub={} bytes",
                der.len(),
                pubkey.len()
            );
            println!("evidence_sign: ok");
        }
        "sign" => {
            if args.len() != 6 {
                eprintln!("sign requires 4 arguments");
                std::process::exit(2);
            }
            let evidence = Path::new(&args[2]);
            let key_path = Path::new(&args[3]);
            let sig_out = Path::new(&args[4]);
            let pub_out = Path::new(&args[5]);
            let der = match std::fs::read(key_path) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("evidence_sign: FAIL - cannot read private key {key_path:?}: {e}");
                    std::process::exit(1);
                }
            };
            let signer = match Ed25519ArtifactSigner::from_pkcs8(&der) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("evidence_sign: FAIL - invalid private key: {e}");
                    std::process::exit(1);
                }
            };
            let digest = match evidence_digest(evidence) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("evidence_sign: FAIL - {e}");
                    std::process::exit(1);
                }
            };
            let sig = match signer.sign_digest(&digest) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("evidence_sign: FAIL - signing failed: {e}");
                    std::process::exit(1);
                }
            };
            let pubkey = signer.public_key();
            if let Err(e) = std::fs::write(sig_out, &sig) {
                eprintln!("evidence_sign: FAIL - write signature: {e}");
                std::process::exit(1);
            }
            if let Err(e) = std::fs::write(pub_out, &pubkey) {
                eprintln!("evidence_sign: FAIL - write public key: {e}");
                std::process::exit(1);
            }
            println!(
                "evidence_sign: signed {} (sha256:{}) sig={} bytes pub={} bytes",
                evidence.display(),
                digest.hex,
                sig.len(),
                pubkey.len()
            );
            println!("evidence_sign: ok");
        }
        "verify" => {
            if args.len() != 5 {
                eprintln!("verify requires 3 arguments");
                std::process::exit(2);
            }
            let evidence = Path::new(&args[2]);
            let pub_path = Path::new(&args[3]);
            let sig_path = Path::new(&args[4]);
            let pubkey = match std::fs::read(pub_path) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("evidence_sign: FAIL - cannot read public key {pub_path:?}: {e}");
                    std::process::exit(1);
                }
            };
            let sig = match std::fs::read(sig_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("evidence_sign: FAIL - cannot read signature {sig_path:?}: {e}");
                    std::process::exit(1);
                }
            };
            let digest = match evidence_digest(evidence) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("evidence_sign: FAIL - {e}");
                    std::process::exit(1);
                }
            };
            match Ed25519ArtifactSigner::verify_with_public_key(&pubkey, &digest, &sig) {
                Ok(()) => {
                    println!(
                        "evidence_sign: signature VERIFIED (ed25519) for {}",
                        evidence.display()
                    );
                    println!("evidence_sign: ok");
                }
                Err(e) => {
                    eprintln!("evidence_sign: FAIL - {e}");
                    std::process::exit(1);
                }
            }
        }
        other => {
            eprintln!("evidence_sign: FAIL - unknown mode {other}");
            std::process::exit(2);
        }
    }
}
