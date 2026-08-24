//! Real Cargo.lock parsing (EP-039 M3).
//!
//! The transport reads the REAL workspace Cargo.lock (TOML via the
//! `toml` crate) and extracts every locked package, including
//! transitives. It never hard-codes a dependency list: the inventory IS
//! the lockfile. TRANSITIVE DEPENDENCY != OUT OF SCOPE (M1 invariant) is
//! preserved because every locked package is returned.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// One package in the real Cargo.lock.
#[derive(Debug, Clone, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub checksum: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// The parsed lockfile.
#[derive(Debug, Clone, Deserialize)]
pub struct Lockfile {
    pub package: Vec<LockedPackage>,
}

/// Read and parse the real Cargo.lock. Fails loudly on malformed input;
/// an unreadable lockfile is a hard error, never a silent empty set.
pub fn read_lockfile(path: &Path) -> Result<Lockfile, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("Cargo.lock unreadable at {}: {e}", path.display()))?;
    let lock: Lockfile = toml::from_str(&raw).map_err(|e| format!("Cargo.lock malformed: {e}"))?;
    if lock.package.is_empty() {
        return Err("Cargo.lock contains zero packages - refusing empty inventory".to_string());
    }
    Ok(lock)
}

/// Index locked packages by `name@version` (lockfile identity).
pub fn index_packages(lock: &Lockfile) -> HashMap<String, &LockedPackage> {
    let mut m = HashMap::new();
    for p in &lock.package {
        m.insert(format!("{}@{}", p.name, p.version), p);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_minimal_lockfile() {
        let path = tempfile_path();
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(
                f,
                "version = 3\n\n[[package]]\nname = \"a\"\nversion = \"1.0.0\"\nchecksum = \"abc\"\n\n[[package]]\nname = \"b\"\nversion = \"2.0.0\"\n"
            )
            .unwrap();
        }
        let lock = read_lockfile(&path).unwrap();
        assert_eq!(lock.package.len(), 2);
        assert_eq!(lock.package[0].name, "a");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn refuses_empty() {
        let path = tempfile_path();
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "version = 3").unwrap();
        }
        assert!(read_lockfile(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }

    fn tempfile_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "ep039-m3-lock-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
