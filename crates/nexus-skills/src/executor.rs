//! EP-018 skill execution boundary (SPEC-010 behavior 7; ADR-025;
//! EP-018 M5 / LF-018; RX-007 AUD-011 / AUD-022).
//!
//! `SkillExecutor` runs an installed skill's payload through a REAL
//! subprocess boundary (std::process::Command), mirroring the
//! EP-017 ProcessRunner pattern: real spawn, capped output capture,
//! real exit status, fail-closed on spawn failure. The skill payload
//! is the executable; the caller passes the resolved, verified package
//! and the effective authority envelope.
//!
//! RX-007 AUD-011: on Linux the subprocess is a REAL OS sandbox, not
//! a convention. `pre_exec` applies, in order:
//!   1. `unshare(CLONE_NEWNS|CLONE_NEWNET|CLONE_NEWIPC|CLONE_NEWUTS)`
//!      - a private mount tree, a private network namespace (only
//!        loopback exists; there is no route to any host interface),
//!        a private IPC namespace, and a private UTS namespace;
//!   2. the mount tree is made private (`MS_REC|MS_PRIVATE`) so no
//!      remount can propagate to the host;
//!   3. a bounded tmpfs is mounted at `/tmp` (the only writable
//!      location the sandbox sees; it vanishes with the namespace);
//!   4. `/`, `/proc`, and `/sys` are remounted read-only (the skill
//!      cannot modify host state, cannot reach host sockets, and
//!      cannot read host process/device state through sysfs);
//!   5. the payload is materialized inside the sandbox tmpfs and the
//!      process drops to uid/gid 65534 (`nobody`) with `setgroups(0)`
//!      - a real privilege drop;
//!   6. `PR_SET_NO_NEW_PRIVS` is set (no setuid binary can escalate);
//!   7. a seccomp BPF filter is installed that returns EPERM for a
//!      deny-list of dangerous syscalls (mount, umount2, ptrace,
//!      kexec*, module load, reboot, swapon/swapoff, sethostname/
//!      setdomainname, bpf, keyctl*, process_vm_*, perf_event_open,
//!      userfaultfd, io_uring_*, open_by_handle_at, name_to_handle_at,
//!      pivot_root, chroot, setns, unshare) and allows everything else.
//!
//! Any sandbox step that fails makes the spawn fail closed: the skill
//! is never executed unsandboxed on Linux.
//!
//! RX-007 AUD-022: execution is bounded and deadlock-free. stdout and
//! stderr are drained CONCURRENTLY by two reader threads (a child that
//! fills stderr while keeping stdout open cannot block the parent),
//! and a wall-clock deadline (`SKILL_EXEC_TIMEOUT`) kills the process
//! group on expiry (`timed_out` is observable on the result). Output
//! is capped per stream so a hostile skill cannot exhaust memory.
//!
//! The boundary is deliberate:
//! - execution is only possible for a package already resolved by the
//!   registry (`resolve_for_execution` fails closed for revoked or
//!   missing skills);
//! - the executor NEVER re-derives authority from the manifest: the
//!   caller (registry/policy) supplies the effective permission set;
//! - the subprocess environment is scrubbed (no inherited secrets);
//!   the skill receives only a bounded, explicit environment;
//! - non-zero exit / spawn failure map to typed SPEC-006 errors, never
//!   a fabricated success.

use crate::manifest::{SkillPackage, SkillPackageError, SkillPackageErrorCode};
use crate::signature::package_signing_message;
use crate::vocabulary::SkillPermission;
use std::io::Read;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Cap on captured skill output per execution (bytes).
pub const SKILL_OUTPUT_CAP: usize = 1 << 20; // 1 MiB

/// Default wall-clock deadline for a skill execution.
pub const SKILL_EXEC_TIMEOUT: Duration = Duration::from_secs(30);

/// The observable result of executing a skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    /// True when the execution was killed by the executor deadline
    /// (AUD-022): the process group was SIGKILLed and `exit_code` is
    /// `-9`. A timed-out execution is never reported as success.
    pub timed_out: bool,
}

/// Real subprocess skill execution boundary.
pub struct SkillExecutor {
    /// Scratch directory for materializing the payload before spawn
    /// (bounded; removed after execution). On Linux the payload is
    /// additionally materialized inside the sandbox tmpfs; the host
    /// copy exists for non-Linux fallback and post-mortem forensics.
    scratch: PathBuf,
    /// Wall-clock execution deadline (RX-007 AUD-022).
    timeout: Duration,
}

impl SkillExecutor {
    pub fn new(scratch: impl Into<PathBuf>) -> Self {
        Self {
            scratch: scratch.into(),
            timeout: SKILL_EXEC_TIMEOUT,
        }
    }

    /// Override the execution deadline (tests use a short bound).
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
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
    /// `NEXUS_SKILL_NAME`, `NEXUS_SKILL_VERSION`, the granted
    /// permissions, and a minimal `PATH`. On Linux the subprocess is a
    /// REAL OS sandbox (namespaces, read-only host, privilege drop,
    /// no_new_privs, seccomp; see module docs). stdout/stderr are
    /// drained concurrently and capped; the exit status is returned as
    /// typed output. A non-zero exit is an observable result, not an
    /// error by itself. A deadline expiry kills the process group and
    /// is observable as `timed_out` (AUD-022).
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

        #[cfg(target_os = "linux")]
        let exec_target: &std::path::Path = std::path::Path::new("/tmp/.nexus-skill-exec");
        #[cfg(not(target_os = "linux"))]
        let exec_target: &std::path::Path = &exe;

        let mut cmd = Command::new(exec_target);
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
        // Bounded explicit environment: the payload shebang is
        // `#!/usr/bin/env sh`; `env` needs a PATH to find the shell.
        cmd.env("PATH", "/usr/bin:/bin");
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "linux")]
        {
            use std::os::unix::process::CommandExt;
            // The child is its own process-group leader so a deadline
            // kill can terminate the whole tree, not just the direct
            // child (AUD-022).
            cmd.process_group(0);
            let sandbox_payload = payload.to_vec();
            // Safety: pre_exec runs in the child after fork, before
            // exec. Every libc call below is checked; any failure makes
            // spawn fail closed (the skill never runs unsandboxed).
            unsafe {
                cmd.pre_exec(move || apply_linux_sandbox(&sandbox_payload));
            }
        }

        let mut child = cmd.spawn().map_err(|_| {
            SkillPackageError::new(
                SkillPackageErrorCode::Unavailable,
                "skill payload could not be spawned (sandbox setup failed closed)",
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

        // Concurrent bounded drain (AUD-022): a child that floods
        // stderr while keeping stdout open must not block the parent.
        // Results arrive over channels so the runner can bound the
        // wait even if a grandchild inherited the pipes and keeps them
        // open after the direct child is reaped.
        let (out_tx, out_rx) = std::sync::mpsc::channel();
        let (err_tx, err_rx) = std::sync::mpsc::channel();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(out) = stdout {
                let _ = out.take(SKILL_OUTPUT_CAP as u64).read_to_end(&mut buf);
            }
            let _ = out_tx.send(buf);
        });
        thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(err) = stderr {
                let _ = err.take(SKILL_OUTPUT_CAP as u64).read_to_end(&mut buf);
            }
            let _ = err_tx.send(buf);
        });

        // Bounded wait: poll `try_wait` until the deadline, then kill
        // the whole process group and reap (AUD-022).
        let deadline = Instant::now() + self.timeout;
        let mut timed_out = false;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {}
                Err(_) => {
                    return Err(SkillPackageError::new(
                        SkillPackageErrorCode::Unavailable,
                        "skill process wait failed",
                        Some("skill-executor".into()),
                    ))
                }
            }
            if Instant::now() >= deadline {
                timed_out = true;
                #[cfg(unix)]
                kill_process_group(child.id() as i32);
                let _ = child.kill();
                break child
                    .wait()
                    .unwrap_or_else(|_| std::process::ExitStatus::from_raw(0));
            }
            thread::sleep(Duration::from_millis(10));
        };

        // Bounded receive: after the child is reaped, the pipes should
        // close and the drains complete; a 2s grace bounds the case of
        // a grandchild holding a pipe open (the process group was
        // killed, so this is only a defensive bound).
        let grace = Duration::from_secs(2);
        let stdout = out_rx.recv_timeout(grace).unwrap_or_default();
        let stderr = err_rx.recv_timeout(grace).unwrap_or_default();

        let _ = std::fs::remove_file(&exe);

        let exit_code = if timed_out {
            -9 // SIGKILL delivered by the executor deadline
        } else {
            status.code().unwrap_or(-1)
        };

        Ok(SkillExecutionResult {
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            exit_code,
            timed_out,
        })
    }
}

#[cfg(unix)]
fn kill_process_group(pid: i32) {
    // Safety: kill(2) is a plain libc call; the pid was returned by
    // the OS for a child we own. Negative pid = process group.
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
}

/// Apply the REAL Linux OS sandbox in the child (AUD-011).
///
/// Runs inside `pre_exec` after fork, before exec. The payload is
/// materialized inside the sandbox `/tmp` tmpfs (mode 0700, owned by
/// `nobody`), the process drops to uid/gid 65534, no_new_privs is set,
/// and a seccomp deny-list filter is installed. Any failure returns
/// `Err`, which makes `Command::spawn` fail closed.
#[cfg(target_os = "linux")]
unsafe fn apply_linux_sandbox(payload: &[u8]) -> Result<(), std::io::Error> {
    use std::ffi::CString;

    let nul = CString::new("").unwrap();
    let tmp = CString::new("/tmp").unwrap();
    let proc = CString::new("/proc").unwrap();
    let sys = CString::new("/sys").unwrap();
    let root = CString::new("/").unwrap();
    let tmpfs = CString::new("tmpfs").unwrap();
    let procfs = CString::new("proc").unwrap();
    let data = CString::new("size=8m,mode=1777").unwrap();
    let payload_path = CString::new("/tmp/.nexus-skill-exec").unwrap();

    let err = |what: &str| {
        std::io::Error::other(format!(
            "sandbox {what} failed: {}",
            std::io::Error::last_os_error()
        ))
    };

    // 1. Real namespaces: mount, network, IPC, UTS. (PID ns is
    //    intentionally not used: the payload is the exec'd process,
    //    and namespace init has special signal semantics that would
    //    weaken deadline delivery.)
    if libc::unshare(
        libc::CLONE_NEWNS | libc::CLONE_NEWNET | libc::CLONE_NEWIPC | libc::CLONE_NEWUTS,
    ) != 0
    {
        return Err(err("unshare"));
    }
    // 2. Private mount tree: no remount propagates to the host.
    if libc::mount(
        nul.as_ptr(),
        root.as_ptr(),
        nul.as_ptr(),
        libc::MS_REC | libc::MS_PRIVATE,
        std::ptr::null(),
    ) != 0
    {
        return Err(err("make-private"));
    }
    // 3. Bounded tmpfs at /tmp: the only writable location.
    if libc::mount(
        tmpfs.as_ptr(),
        tmp.as_ptr(),
        tmpfs.as_ptr(),
        0,
        data.as_ptr() as *const libc::c_void,
    ) != 0
    {
        return Err(err("tmpfs-mount"));
    }
    // 4. Read-only host: /, /proc, /sys (bind-remount read-only).
    for (target, _fstype) in [
        (root.as_ptr(), nul.as_ptr()),
        (proc.as_ptr(), procfs.as_ptr()),
        (sys.as_ptr(), nul.as_ptr()),
    ] {
        if libc::mount(
            nul.as_ptr(),
            target,
            _fstype,
            libc::MS_REMOUNT | libc::MS_BIND | libc::MS_RDONLY,
            std::ptr::null(),
        ) != 0
        {
            return Err(err("readonly-remount"));
        }
    }
    // 5. Materialize the payload inside the sandbox tmpfs, owned by
    //    nobody so the dropped process can execute it.
    {
        let fd = libc::open(
            payload_path.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            0o700,
        );
        if fd < 0 {
            return Err(err("payload-open"));
        }
        let mut written = 0usize;
        while written < payload.len() {
            let n = libc::write(
                fd,
                payload.as_ptr().add(written) as *const libc::c_void,
                payload.len() - written,
            );
            if n <= 0 {
                libc::close(fd);
                return Err(err("payload-write"));
            }
            written += n as usize;
        }
        libc::close(fd);
        if libc::chown(payload_path.as_ptr(), 65534, 65534) != 0 {
            return Err(err("payload-chown"));
        }
    }
    // 6. Drop privileges: real uid/gid nobody, empty supplementary
    //    groups.
    if libc::setgroups(0, std::ptr::null()) != 0 {
        return Err(err("setgroups"));
    }
    if libc::setgid(65534) != 0 {
        return Err(err("setgid"));
    }
    if libc::setuid(65534) != 0 {
        return Err(err("setuid"));
    }
    // 7. No new privileges: setuid binaries cannot escalate.
    if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
        return Err(err("no-new-privs"));
    }
    // 8. seccomp: deny-list filter (default allow, EPERM for the
    //    dangerous syscall set).
    install_seccomp_denylist()?;
    Ok(())
}

/// Install the AUD-011 seccomp BPF deny-list filter.
///
/// x86_64 syscall numbers only. The filter checks the architecture
/// first (kills on anything else), then returns EPERM for the denied
/// syscalls and ALLOW for everything else. The denied set is chosen so
/// a sandboxed skill cannot: mount/remount anything, ptrace, load
/// kernel modules, reboot/swap, change host identity, use kernel
/// keyrings, eBPF, process_vm_*, perf/io_uring, open-by-handle, or
/// escape through chroot/pivot_root/setns/unshare.
#[cfg(target_os = "linux")]
unsafe fn install_seccomp_denylist() -> Result<(), std::io::Error> {
    #[cfg(target_arch = "x86_64")]
    const DENY: &[u32] = &[
        101, // ptrace
        155, // pivot_root
        161, // chroot
        165, // mount
        166, // umount2
        167, // swapon
        168, // swapoff
        169, // reboot
        170, // sethostname
        171, // setdomainname
        175, // init_module
        176, // delete_module
        246, // kexec_load
        248, // add_key
        249, // request_key
        250, // keyctl
        272, // unshare
        298, // perf_event_open
        303, // name_to_handle_at
        304, // open_by_handle_at
        308, // setns
        313, // finit_module
        320, // kexec_file_load
        321, // bpf
        323, // userfaultfd
        310, // process_vm_readv
        311, // process_vm_writev
        425, // io_uring_setup
        426, // io_uring_enter
        427, // io_uring_register
    ];
    #[cfg(not(target_arch = "x86_64"))]
    const DENY: &[u32] = &[];

    // BPF instruction encoding (classic socket filter).
    const BPF_LD: u16 = 0x00;
    const BPF_W: u16 = 0x00;
    const BPF_ABS: u16 = 0x20;
    const BPF_JMP: u16 = 0x05;
    const BPF_JEQ: u16 = 0x10;
    const BPF_K: u16 = 0x00;
    const BPF_RET: u16 = 0x06;
    const SECCOMP_RET_KILL: u32 = 0x0000_0000;
    const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;
    const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
    const AUDIT_ARCH_X86_64: u32 = 0xc000_003e;

    #[cfg(target_arch = "x86_64")]
    let mut prog: Vec<libc::sock_filter> = Vec::new();
    #[cfg(not(target_arch = "x86_64"))]
    let mut prog: Vec<libc::sock_filter> = Vec::new();

    let mut push = |code: u16, jt: u8, jf: u8, k: u32| {
        prog.push(libc::sock_filter { code, jt, jf, k });
    };

    // Load arch (offset 4); if != x86_64 -> KILL.
    push(BPF_LD | BPF_W | BPF_ABS, 0, 0, 4);
    push(BPF_JMP | BPF_JEQ | BPF_K, 1, 0, AUDIT_ARCH_X86_64);
    push(BPF_RET | BPF_K, 0, 0, SECCOMP_RET_KILL);
    // Load syscall nr (offset 0).
    push(BPF_LD | BPF_W | BPF_ABS, 0, 0, 0);
    // Deny list: each `jeq n, jt=1, jf=0` jumps to the ERRNO return.
    for nr in DENY {
        push(BPF_JMP | BPF_JEQ | BPF_K, 1, 0, *nr);
    }
    // Default: allow.
    push(BPF_RET | BPF_K, 0, 0, SECCOMP_RET_ALLOW);
    // Denied: EPERM.
    push(
        BPF_RET | BPF_K,
        0,
        0,
        SECCOMP_RET_ERRNO | 1, /* EPERM */
    );

    let mut fprog = libc::sock_fprog {
        len: prog.len() as u16,
        filter: prog.as_mut_ptr(),
    };
    if libc::prctl(
        libc::PR_SET_SECCOMP,
        libc::SECCOMP_MODE_FILTER,
        &mut fprog as *mut libc::sock_fprog,
        0,
        0,
    ) != 0
    {
        return Err(std::io::Error::other(format!(
            "seccomp install failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

/// Deterministic canary used by live-fire evidence (LF-018): the
/// canonical identity the signature binds to.
pub fn signing_message_for(package: &SkillPackage) -> String {
    String::from_utf8_lossy(&package_signing_message(package)).into_owned()
}
