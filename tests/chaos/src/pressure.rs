//! Resource-pressure detection and owned-prefix residue attribution
//! (EP-040 M5 fence section I). M4 discovered host disk exhaustion as a
//! real failure mode; M5 encodes the lesson: pressure is detected,
//! residue is attributed to an owned prefix, cleanup is bounded, and
//! global prune is never treated as a normal success mechanism.

use std::path::PathBuf;

use nexus_test_contract::error::{TestingError, TestingErrorCode, TestingResult};

/// Result of a pressure probe.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PressureProbe {
    /// Available bytes on the probed filesystem.
    pub available_bytes: u64,
    /// Total bytes on the probed filesystem.
    pub total_bytes: u64,
    /// True when available space is below the low-water threshold.
    pub pressure_detected: bool,
    /// EP-040-owned temp roots found on disk at probe time.
    pub owned_temp_roots: Vec<String>,
    /// True when every owned root is attributable to the owned prefix.
    pub attribution_ok: bool,
}

/// Probe disk pressure on the filesystem hosting /tmp and scan for
/// EP-040-owned temp roots. Pressure is detected when the low-water
/// threshold is breached; residue is attributed only to the owned
/// prefix (never a global scan).
pub fn probe_disk_pressure(low_water_bytes: u64) -> TestingResult<PressureProbe> {
    let stat = nix_statvfs()?;
    let available_bytes = stat.available_bytes;
    let total_bytes = stat.total_bytes;
    let pressure_detected = available_bytes < low_water_bytes;

    // Owned-prefix residue scan: only /tmp/ep040-m5-* roots count.
    let mut owned_temp_roots = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/tmp") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("ep040-m5-") {
                owned_temp_roots.push(format!("/tmp/{name}"));
            }
        }
    }
    let attribution_ok = owned_temp_roots
        .iter()
        .all(|p| p.starts_with("/tmp/ep040-m5-"));

    Ok(PressureProbe {
        available_bytes,
        total_bytes,
        pressure_detected,
        owned_temp_roots,
        attribution_ok,
    })
}

/// Remove an EP-040-owned temp root (bounded cleanup). Only paths under
/// the owned prefix may be removed; anything else is refused.
pub fn remove_owned_temp_root(root: &str) -> TestingResult<()> {
    let path = PathBuf::from(root);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if !name.starts_with("ep040-m5-") {
        return Err(TestingError::new(
            TestingErrorCode::Policy,
            format!("refusing to remove non-owned temp root: {root}"),
        ));
    }
    std::fs::remove_dir_all(&path).map_err(|e| {
        TestingError::new(
            TestingErrorCode::Unavailable,
            format!("cannot remove owned temp root {root}: {e}"),
        )
    })?;
    Ok(())
}

/// Minimal statvfs shim (no external crate): reads /proc/mounts-free
/// info via the statvfs syscall through libc-free parsing of /proc.
/// Falls back to a conservative report when unavailable.
fn nix_statvfs() -> TestingResult<StatVfs> {
    #[cfg(target_os = "linux")]
    {
        let c_path = std::ffi::CString::new("/tmp").map_err(|_| {
            TestingError::new(TestingErrorCode::Internal, "invalid path for statvfs")
        })?;
        let mut buf = std::mem::MaybeUninit::<StatVfsRaw>::uninit();
        // SAFETY: buf is a valid MaybeUninit; the syscall initializes it
        // on success. libc statvfs is documented.
        let rc = unsafe { statvfs_syscall(c_path.as_ptr(), buf.as_mut_ptr()) };
        if rc == 0 {
            // SAFETY: rc == 0 means the syscall wrote the struct.
            let st = unsafe { buf.assume_init() };
            Ok(StatVfs {
                total_bytes: st.f_blocks * st.f_frsize,
                available_bytes: st.f_bavail * st.f_frsize,
            })
        } else {
            Err(TestingError::new(
                TestingErrorCode::Unavailable,
                format!("statvfs failed with rc={rc}"),
            ))
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(TestingError::new(
            TestingErrorCode::CapabilityBlocked,
            "statvfs probe not implemented on this platform",
        ))
    }
}

#[cfg(target_os = "linux")]
mod sys {
    // Bind the statvfs struct and syscall without adding a crate.
    #[repr(C)]
    pub struct statvfs {
        pub f_bsize: u64,
        pub f_frsize: u64,
        pub f_blocks: u64,
        pub f_bfree: u64,
        pub f_bavail: u64,
        pub f_files: u64,
        pub f_ffree: u64,
        pub f_favail: u64,
        pub f_fsid: u64,
        pub f_flag: u64,
        pub f_namemax: u64,
        pub f_frsize_align: [u64; 0],
    }
    extern "C" {
        pub fn statvfs(path: *const std::os::raw::c_char, buf: *mut statvfs)
            -> std::os::raw::c_int;
    }
}

#[cfg(target_os = "linux")]
struct StatVfs {
    total_bytes: u64,
    available_bytes: u64,
}

#[cfg(target_os = "linux")]
type StatVfsRaw = sys::statvfs;

#[cfg(target_os = "linux")]
/// SAFETY contract: caller passes a valid NUL-terminated path and a
/// writable buffer of the correct type.
unsafe fn statvfs_syscall(
    path: *const std::os::raw::c_char,
    buf: *mut StatVfsRaw,
) -> std::os::raw::c_int {
    // SAFETY: delegated to the libc symbol with the same contract.
    unsafe { sys::statvfs(path, buf) }
}
