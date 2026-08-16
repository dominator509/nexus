//! EP-018 skill execution boundary (SPEC-010 behavior 7; ADR-025;
//! EP-018 M5 / LF-018).
//!
//! `SkillExecutor` runs an installed skill's payload through a REAL
//! subprocess boundary (std::process::Command), mirroring the
//! EP-017 ProcessRunner pattern: real spawn, capped output capture,
//! real exit status, fail-closed on spawn failure. The skill payload
//! is the executable; the caller passes the resolved, verified package
//! and the effective authority envelope.
//!
//! The boundary is deliberate:
//! - execution is only possible for a package already resolved by the
//!   registry (`resolve_for_execution` fails closed for revoked or
//!   missing skills);
//! - the executor NEVER re-derives authority from the manifest: the
//!   caller (registry/policy) supplies the effective permission set;
//! - the subprocess environment is scrubbed (no inherited secrets);
//!   the skill receives only a bounded, explicit environment;
//! - output is capped so a hostile skill cannot exhaust memory;
//! - non-zero exit / spawn failure map to typed SPEC-006 errors, never
//!   a fabricated success.

use crate::manifest::{SkillPackage, SkillPackageError, SkillPackageErrorCode};
use crate::signature::package_signing_message;
use crate::vocabulary::SkillPermission;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Cap on captured skill output per execution (bytes).
pub const SKILL_OUTPUT_CAP: usize = 1 << 20; // 1 MiB

/// The observable result of executing a skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Real subprocess skill execution boundary.
pub struct SkillExecutor {
    /// Scratch directory for materializing the payload before spawn
    /// (bounded; removed after execution).
    scratch: PathBuf,
}

impl SkillExecutor {
    pub fn new(scratch: impl Into<PathBuf>) -> Self {
        Self {
            scratch: scratch.into(),
        }
    }

    /// Execute the skill payload with the given input bytes.
    ///
    /// Fail-closed preconditions:
    /// 1. `package.validate()` (manifest + signature structure);
    /// 2. `verify_cryptographic` (real ring Ed25519) over the canonical
    ///    package identity digest;
    /// 3. the caller's authority envelope must actually allow every
    ///    declared permission (a skill can never grant itself
    ///    authority at runtime).
    ///
    /// The payload is written to a scratch file (mode 0700) and spawned
    /// as a real subprocess with a scrubbed environment containing only
    /// `NEXUS_SKILL_NAME`, `NEXUS_SKILL_VERSION`, and the granted
    /// permissions. stdout/stderr are captured (capped); the exit
    /// status is returned as typed output. A non-zero exit is an
    /// observable result, not an error by itself.
    pub fn execute(
        &self,
        package: &SkillPackage,
        payload: &[u8],
        input: &[u8],
        granted: &[SkillPermission],
    ) -> Result<SkillExecutionResult, SkillPackageError> {
        package.validate()?;
        package.manifest.signature.verify_cryptographic(package)?;
        // The declared permissions must be within the caller's granted
        // envelope: the manifest declares requirements, the caller
        // grants authority (ADR-025). No runtime self-grant.
        for permission in package.declared_permissions() {
            if !granted.contains(permission) {
                return Err(SkillPackageError::policy(
                    "skill requests a permission the caller did not grant",
                    Some(package.canonical_identity()),
                ));
            }
        }

        let _ = std::fs::create_dir_all(&self.scratch);
        let exe = self.scratch.join(format!(
            "skill-{}-{}.sh",
            package.manifest.name.replace('/', "_"),
            package.manifest.version
        ));
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&exe).map_err(|_| {
                SkillPackageError::unavailable(
                    "cannot materialize skill payload",
                    Some("skill-executor".into()),
                )
            })?;
            f.write_all(payload).map_err(|_| {
                SkillPackageError::unavailable(
                    "cannot write skill payload",
                    Some("skill-executor".into()),
                )
            })?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o700));
        }

        let mut cmd = Command::new(&exe);
        // Scrubbed environment: never inherit secrets from the parent.
        cmd.env_clear();
        cmd.env("NEXUS_SKILL_NAME", &package.manifest.name);
        cmd.env("NEXUS_SKILL_VERSION", &package.manifest.version);
        cmd.env(
            "NEXUS_SKILL_GRANTED_PERMISSIONS",
            granted
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(","),
        );
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|_| {
            SkillPackageError::new(
                SkillPackageErrorCode::Unavailable,
                "skill payload could not be spawned",
                Some("skill-executor".into()),
            )
        })?;

        if let Some(mut stdin) = child.stdin.take() {
            // Line-oriented skill protocol: terminate the final input
            // line so a blocking `read` in the payload returns the
            // full line (stdin is closed right after).
            let mut input_owned = input.to_vec();
            if !input_owned.ends_with(b"\n") {
                input_owned.push(b'\n');
            }
            let _ = std::io::Write::write_all(&mut stdin, &input_owned);
        }

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        if let Some(out) = child.stdout.take() {
            let _ = out.take(SKILL_OUTPUT_CAP as u64).read_to_end(&mut stdout);
        }
        if let Some(err) = child.stderr.take() {
            let _ = err.take(SKILL_OUTPUT_CAP as u64).read_to_end(&mut stderr);
        }
        let status = child.wait().map_err(|_| {
            SkillPackageError::new(
                SkillPackageErrorCode::Unavailable,
                "skill process wait failed",
                Some("skill-executor".into()),
            )
        })?;

        let _ = std::fs::remove_file(&exe);

        Ok(SkillExecutionResult {
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            exit_code: status.code().unwrap_or(-1),
        })
    }
}

/// Deterministic canary used by live-fire evidence (LF-018): the
/// canonical identity the signature binds to.
pub fn signing_message_for(package: &SkillPackage) -> String {
    String::from_utf8_lossy(&package_signing_message(package)).into_owned()
}
