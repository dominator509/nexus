//! EP-027 M3 HylaFAX transport (SPEC-014).
//!
//! Real hfaxd client-server protocol transport, built ONLY from
//! documented hfaxd behavior and observed real wire exchanges
//! (sendfax -vv trace + proxy control capture + tcpdump packet capture
//! with byte-for-byte integrity proof of the zlib-compressed document
//! channel). No guessed framing.
//!
//! Observed protocol (HylaFAX 6.0.6-8.1, pinned fixture):
//!   CONNECT -> 220 greeting (CRLF, single-line)
//!   USER u  -> 331 (password required) / 230 (localhost auto-auth)
//!   PASS p  -> 230 / 530 Login incorrect
//!   TZONE LOCAL -> 200
//!   FORM PS -> 200
//!   TYPE I  -> 200
//!   EPRT |1|IP|port|  -> 200  (client advertises its data listener)
//!   MODE Z  -> 200  (STOT data channel is zlib-compressed)
//!   STOT    -> 150 FILE: <tmp> (server opens data channel, client
//!             pushes zlib-compressed bytes) -> 226 Transfer complete
//!   LIST sendq -> 150 Opening new data connection ... -> client reads
//!             PLAINTEXT job rows until EOF -> 226 Transfer complete
//!   JNEW    -> 200 New job created: jobid: N groupid: N.
//!   JPARM k v -> 213 / 200
//!   JPARM DOCUMENT <tmp> -> 200 Added document ... as docq/docN.ps.M
//!   JSUBM   -> 200 Job N submitted.
//!             -> 460 scheduler NAK (incomplete required parameters)
//!             -> 504 missing parameter / already done
//!
//! Data channel (proven from pcap + probes): hfaxd is the ACTIVE side.
//! The client binds a listener, advertises it with EPRT, the server
//! connects, and application bytes flow per operation: STOT is an
//! UPLOAD (client writes zlib-compressed document bytes) while LIST is
//! a DOWNLOAD (client reads plaintext rows). MODE Z compression is
//! observed ONLY on the STOT upload; LIST payloads remain plaintext
//! even when MODE Z is active.
//!
//! The transport is a strict state machine: every transition is
//! validated, and invalid sequencing fails closed. One session owns
//! one control connection; job state is connection-scoped (JNEW ->
//! JPARM -> JSUBM must run on the same session).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::sync::Mutex;

use flate2::write::ZlibEncoder;
use flate2::Compression;

use nexus_fax::{FaxCarrierJobId, FaxError};

/// One hfaxd control/data session (real TCP transport).
///
/// Implementations must be connection-scoped: `connect_authenticate`
/// opens the session and every later call runs on the SAME connection
/// (job state is per-session on hfaxd).
pub trait HylaFaxTransport: Send + Sync {
    fn connect_authenticate(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<(), FaxError>;
    fn prepare_transfer(&self) -> Result<(), FaxError>;
    fn upload_document(&self, data: &[u8]) -> Result<String, FaxError>;
    fn create_job(&self) -> Result<String, FaxError>;
    fn set_job_parameter(&self, key: &str, value: &str) -> Result<(), FaxError>;
    fn attach_document(&self, server_file: &str) -> Result<(), FaxError>;
    fn submit_job(&self) -> Result<FaxCarrierJobId, FaxError>;
    fn query_job(&self, job_id: &str) -> Result<String, FaxError>;
    fn quit(&self) -> Result<(), FaxError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportState {
    Disconnected,
    Connected,
    Authenticated,
    TransferReady,
    DocumentUploaded,
    JobCreated,
    Submitted,
}

impl TransportState {
    fn rank(self) -> u8 {
        match self {
            TransportState::Disconnected => 0,
            TransportState::Connected => 1,
            TransportState::Authenticated => 2,
            TransportState::TransferReady => 3,
            TransportState::DocumentUploaded => 4,
            TransportState::JobCreated => 5,
            TransportState::Submitted => 6,
        }
    }

    fn require(self, expected: TransportState, what: &str) -> Result<(), FaxError> {
        if self == expected {
            Ok(())
        } else {
            Err(FaxError::unavailable(format!(
                "hylafax transport state {:?} does not permit {what} (fail closed)",
                self
            )))
        }
    }

    fn require_at_least(self, expected: TransportState, what: &str) -> Result<(), FaxError> {
        if self.rank() >= expected.rank() {
            Ok(())
        } else {
            Err(FaxError::unavailable(format!(
                "hylafax transport state {:?} does not permit {what} (fail closed)",
                self
            )))
        }
    }
}

/// Data transfer direction on the hfaxd active data channel.
///
/// The server initiates the TCP connection in BOTH cases (it is the
/// active connector); the direction describes which side writes
/// application bytes once the channel is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataDirection {
    /// Client writes application bytes (STOT document upload).
    Upload,
    /// Client reads application bytes (LIST job queue download).
    Download,
}

/// Parse one control response line into (code, rest).
fn parse_response(line: &str) -> Result<(u16, String), FaxError> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let mut parts = trimmed.splitn(2, ' ');
    let code_str = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").to_string();
    let code: u16 = code_str
        .parse()
        .map_err(|_| FaxError::external(format!("hylafax malformed response {line:?}")))?;
    Ok((code, rest))
}

struct Session {
    conn: TcpStream,
    reader: BufReader<TcpStream>,
}

/// Real TCP HylaFAX transport: one connection, explicit state machine.
pub struct HylaFaxTcpTransport {
    // Host/port/username/password are retained for the transport's
    // identity and diagnostics; sessions are opened explicitly through
    // `connect_authenticate(host, port, username, password)`.
    _host: String,
    _port: u16,
    _username: String,
    _password: String,
    session: Mutex<Option<Session>>,
    state: Mutex<TransportState>,
    last_stot_file: Mutex<Option<String>>,
    last_job_id: Mutex<Option<String>>,
}

impl HylaFaxTcpTransport {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            _host: host.into(),
            _port: port,
            _username: username.into(),
            _password: password.into(),
            session: Mutex::new(None),
            state: Mutex::new(TransportState::Disconnected),
            last_stot_file: Mutex::new(None),
            last_job_id: Mutex::new(None),
        }
    }

    fn with_session<T>(
        &self,
        f: impl FnOnce(&mut TcpStream, &mut BufReader<TcpStream>) -> Result<T, FaxError>,
    ) -> Result<T, FaxError> {
        let mut guard = self
            .session
            .lock()
            .map_err(|_| FaxError::unavailable("hylafax session lock poisoned"))?;
        let session = guard
            .as_mut()
            .ok_or_else(|| FaxError::unavailable("hylafax transport not connected"))?;
        f(&mut session.conn, &mut session.reader)
    }

    fn command(&self, line: &str) -> Result<(u16, String), FaxError> {
        self.with_session(|conn, reader| {
            conn.write_all(format!("{line}\r\n").as_bytes())
                .map_err(|e| FaxError::unavailable(format!("hylafax write failed: {e}")))?;
            conn.flush()
                .map_err(|e| FaxError::unavailable(format!("hylafax flush failed: {e}")))?;
            let mut buf = String::new();
            reader
                .read_line(&mut buf)
                .map_err(|e| FaxError::unavailable(format!("hylafax read failed: {e}")))?;
            if buf.is_empty() {
                return Err(FaxError::unavailable("hylafax connection closed by server"));
            }
            parse_response(&buf)
        })
    }

    /// Read one control response line without sending a command (used
    /// for the authoritative completion line that follows a data
    /// channel close).
    fn read_response_line(&self) -> Result<(u16, String), FaxError> {
        self.with_session(|_conn, reader| {
            let mut buf = String::new();
            reader.read_line(&mut buf).map_err(|e| {
                FaxError::unavailable(format!("hylafax completion read failed: {e}"))
            })?;
            if buf.is_empty() {
                return Err(FaxError::unavailable(
                    "hylafax connection closed before completion response",
                ));
            }
            parse_response(&buf)
        })
    }

    fn set_state(&self, state: TransportState) {
        if let Ok(mut s) = self.state.lock() {
            *s = state;
        }
    }

    fn state(&self) -> TransportState {
        self.state
            .lock()
            .map(|s| *s)
            .unwrap_or(TransportState::Disconnected)
    }

    fn set_stot_file(&self, file: String) {
        if let Ok(mut s) = self.last_stot_file.lock() {
            *s = Some(file);
        }
    }

    fn set_job_id(&self, id: String) {
        if let Ok(mut s) = self.last_job_id.lock() {
            *s = Some(id);
        }
    }

    fn read_greeting(&self) -> Result<(), FaxError> {
        let (code, rest) = self.with_session(|_conn, reader| {
            let mut buf = String::new();
            reader
                .read_line(&mut buf)
                .map_err(|e| FaxError::unavailable(format!("hylafax greeting read failed: {e}")))?;
            if buf.is_empty() {
                return Err(FaxError::unavailable(
                    "hylafax greeting missing (connection closed)",
                ));
            }
            parse_response(&buf)
        })?;
        if code != 220 {
            return Err(FaxError::unavailable(format!(
                "hylafax unexpected greeting code {code}: {rest}"
            )));
        }
        self.set_state(TransportState::Connected);
        Ok(())
    }

    /// Local IP of the control connection: the address hfaxd observes
    /// the client at, and the address the data listener must advertise
    /// in EPRT so the server can connect back (same route).
    fn control_ip(&self) -> Result<IpAddr, FaxError> {
        let ip = self.with_session(|conn, _reader| {
            conn.local_addr()
                .map(|a| a.ip())
                .map_err(|e| FaxError::unavailable(format!("hylafax control local addr: {e}")))
        })?;
        // Normalize IPv4-mapped IPv6 addresses; only IPv4 EPRT framing
        // is observed in the pinned fixture.
        let ip = ip.to_canonical();
        if !ip.is_ipv4() {
            return Err(FaxError::unavailable(
                "hylafax EPRT requires an IPv4 control address (observed framing |1|)",
            ));
        }
        Ok(ip)
    }

    /// Shared active data-channel primitive (observed hfaxd model).
    ///
    /// Sequence: bind listener -> EPRT (200) -> optional MODE Z (200)
    /// -> data command (preliminary 150) -> hfaxd ACTIVELY connects to
    /// the client listener -> application bytes flow per `direction`
    /// -> data socket closes -> authoritative completion (226).
    ///
    /// Returns (preliminary response text, payload bytes).
    fn data_exchange(
        &self,
        command: &str,
        direction: DataDirection,
        compress: bool,
        upload_data: Option<&[u8]>,
    ) -> Result<(String, Vec<u8>), FaxError> {
        self.state()
            .require_at_least(TransportState::Authenticated, "data channel")?;
        let ip = self.control_ip()?;
        let listener = TcpListener::bind((ip, 0)).map_err(|e| {
            FaxError::unavailable(format!("hylafax data listener bind failed: {e}"))
        })?;
        let port = listener
            .local_addr()
            .map_err(|e| FaxError::unavailable(format!("hylafax data listener addr failed: {e}")))?
            .port();

        let (code, rest) = self.command(&format!("EPRT |1|{ip}|{port}|"))?;
        if code != 200 {
            return Err(FaxError::unavailable(format!(
                "hylafax EPRT rejected ({code}): {rest}"
            )));
        }
        if compress {
            let (code, rest) = self.command("MODE Z")?;
            if code != 200 {
                return Err(FaxError::unavailable(format!(
                    "hylafax MODE Z rejected ({code}): {rest}"
                )));
            }
        }
        let (prelim_code, prelim_rest) = self.command(command)?;
        if prelim_code != 150 && prelim_code != 125 {
            return Err(FaxError::unavailable(format!(
                "hylafax {command} rejected ({prelim_code}): {prelim_rest}"
            )));
        }

        // hfaxd is the ACTIVE side: it connects to our listener.
        listener
            .set_nonblocking(true)
            .map_err(|e| FaxError::unavailable(format!("hylafax listener nonblocking: {e}")))?;
        let mut accepted = None;
        for _ in 0..50 {
            match listener.accept() {
                Ok((stream, _)) => {
                    accepted = Some(stream);
                    break;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => {
                    return Err(FaxError::unavailable(format!(
                        "hylafax data accept failed: {e}"
                    )));
                }
            }
        }
        let Some(mut stream) = accepted else {
            return Err(FaxError::unavailable(
                "hylafax data connection not established (server did not connect)",
            ));
        };
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .map_err(|e| FaxError::unavailable(format!("hylafax data read timeout: {e}")))?;
        stream
            .set_write_timeout(Some(std::time::Duration::from_secs(15)))
            .map_err(|e| FaxError::unavailable(format!("hylafax data write timeout: {e}")))?;

        let payload = match direction {
            DataDirection::Upload => {
                let data = upload_data
                    .ok_or_else(|| FaxError::internal("hylafax upload direction without data"))?;
                // MODE Z: push zlib-compressed document bytes.
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(data).map_err(|e| {
                    FaxError::unavailable(format!("hylafax zlib encode failed: {e}"))
                })?;
                let compressed = encoder.finish().map_err(|e| {
                    FaxError::unavailable(format!("hylafax zlib finish failed: {e}"))
                })?;
                stream.write_all(&compressed).map_err(|e| {
                    FaxError::unavailable(format!("hylafax data write failed: {e}"))
                })?;
                stream.flush().map_err(|e| {
                    FaxError::unavailable(format!("hylafax data flush failed: {e}"))
                })?;
                // Graceful close so the server sees EOF and completes
                // the transfer.
                stream.shutdown(std::net::Shutdown::Write).map_err(|e| {
                    FaxError::unavailable(format!("hylafax data shutdown failed: {e}"))
                })?;
                Vec::new()
            }
            DataDirection::Download => {
                // LIST payloads are observed PLAINTEXT even when MODE Z
                // is active; never force decompression.
                let mut buf = Vec::new();
                stream
                    .read_to_end(&mut buf)
                    .map_err(|e| FaxError::unavailable(format!("hylafax data read failed: {e}")))?;
                buf
            }
        };
        drop(listener);

        // The authoritative completion line arrives after the data
        // channel closes; it is the response to the data command, so
        // read it without sending anything. The PRELIMINARY rest (150)
        // is what callers inspect (e.g. the STOT FILE path); the
        // completion rest is only validated for the 2xx code.
        let (code, _completion_rest) = self.read_response_line()?;
        if !(200..=299).contains(&code) {
            return Err(FaxError::unavailable(format!(
                "hylafax {command} completion expected 2xx, got ({code}): {_completion_rest}"
            )));
        }
        Ok((prelim_rest, payload))
    }
}

impl HylaFaxTransport for HylaFaxTcpTransport {
    fn connect_authenticate(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<(), FaxError> {
        if self.state() != TransportState::Disconnected {
            return Err(FaxError::unavailable(
                "hylafax session already active (one session per transport)",
            ));
        }
        // Drop any previous session (after a clean quit) so a fresh
        // connection can be opened on the same transport.
        {
            let mut guard = self
                .session
                .lock()
                .map_err(|_| FaxError::unavailable("hylafax session lock poisoned"))?;
            *guard = None;
        }
        let conn = TcpStream::connect((host, port)).map_err(|e| {
            FaxError::unavailable(format!("hylafax connect {host}:{port} failed: {e}"))
        })?;
        conn.set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .map_err(|e| FaxError::unavailable(format!("hylafax read timeout setup: {e}")))?;
        conn.set_write_timeout(Some(std::time::Duration::from_secs(15)))
            .map_err(|e| FaxError::unavailable(format!("hylafax write timeout setup: {e}")))?;
        let reader = BufReader::new(
            conn.try_clone()
                .map_err(|e| FaxError::unavailable(format!("hylafax clone: {e}")))?,
        );
        {
            let mut guard = self
                .session
                .lock()
                .map_err(|_| FaxError::unavailable("hylafax session lock poisoned"))?;
            *guard = Some(Session { conn, reader });
        }
        self.read_greeting()?;

        let (code, _rest) = self.command(&format!("USER {username}"))?;
        match code {
            331 => {}
            230 => {
                // hfaxd auto-authenticated the connection (observed for
                // the fixture's localhost entry with an empty password).
                self.set_state(TransportState::Authenticated);
                return Ok(());
            }
            other => {
                return Err(FaxError::authorization(format!(
                    "hylafax USER rejected ({other})"
                )));
            }
        }
        let (code, rest) = self.command(&format!("PASS {password}"))?;
        if code != 230 {
            return Err(FaxError::authorization(format!(
                "hylafax authentication failed ({code}): {rest}"
            )));
        }
        self.set_state(TransportState::Authenticated);
        Ok(())
    }

    fn prepare_transfer(&self) -> Result<(), FaxError> {
        self.state()
            .require(TransportState::Authenticated, "transfer preparation")?;
        let (code, rest) = self.command("TZONE LOCAL")?;
        if code != 200 {
            return Err(FaxError::unavailable(format!(
                "hylafax TZONE rejected ({code}): {rest}"
            )));
        }
        let (code, rest) = self.command("FORM PS")?;
        if code != 200 {
            return Err(FaxError::unavailable(format!(
                "hylafax FORM rejected ({code}): {rest}"
            )));
        }
        let (code, rest) = self.command("TYPE I")?;
        if code != 200 {
            return Err(FaxError::unavailable(format!(
                "hylafax TYPE rejected ({code}): {rest}"
            )));
        }
        self.set_state(TransportState::TransferReady);
        Ok(())
    }

    fn upload_document(&self, data: &[u8]) -> Result<String, FaxError> {
        self.state()
            .require(TransportState::TransferReady, "document upload")?;
        // STOT is an UPLOAD on the active data channel with MODE Z
        // compression (observed wire bytes; zlib 78 9c header).
        let (rest, _payload) =
            self.data_exchange("STOT", DataDirection::Upload, true, Some(data))?;
        // Extract the server temp file path from the 150 preliminary:
        // "150 FILE: /tmp/docN.ps (...)"
        let file = rest
            .split("FILE:")
            .nth(1)
            .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| FaxError::external(format!("hylafax STOT missing FILE path: {rest}")))?;
        self.set_stot_file(file.clone());
        self.set_state(TransportState::DocumentUploaded);
        Ok(file)
    }

    fn create_job(&self) -> Result<String, FaxError> {
        // The server accepts JNEW on any authenticated session
        // (observed: JNEW right after PASS returns 200; document
        // upload is not a prerequisite for creating the job shell).
        self.state()
            .require_at_least(TransportState::Authenticated, "job creation")?;
        let (code, rest) = self.command("JNEW")?;
        if code != 200 {
            return Err(FaxError::unavailable(format!(
                "hylafax JNEW failed ({code}): {rest}"
            )));
        }
        // "New job created: jobid: 6 groupid: 6."
        let job_id = rest
            .split("jobid:")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .map(|s| s.trim_end_matches('.').to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| FaxError::external(format!("hylafax JNEW missing jobid: {rest}")))?;
        self.set_job_id(job_id.clone());
        self.set_state(TransportState::JobCreated);
        Ok(job_id)
    }

    fn set_job_parameter(&self, key: &str, value: &str) -> Result<(), FaxError> {
        self.state()
            .require(TransportState::JobCreated, "job parameter")?;
        let (code, rest) = self.command(&format!("JPARM {key} {value}"))?;
        if !(200..=299).contains(&code) {
            return Err(FaxError::unavailable(format!(
                "hylafax JPARM {key} rejected ({code}): {rest}"
            )));
        }
        Ok(())
    }

    fn attach_document(&self, server_file: &str) -> Result<(), FaxError> {
        self.state()
            .require(TransportState::JobCreated, "document attach")?;
        let (code, rest) = self.command(&format!("JPARM DOCUMENT {server_file}"))?;
        if !(200..=299).contains(&code) {
            return Err(FaxError::unavailable(format!(
                "hylafax JPARM DOCUMENT rejected ({code}): {rest}"
            )));
        }
        Ok(())
    }

    fn submit_job(&self) -> Result<FaxCarrierJobId, FaxError> {
        self.state()
            .require(TransportState::JobCreated, "submission")?;
        let (code, rest) = self.command("JSUBM")?;
        if code != 200 {
            return Err(FaxError::unavailable(format!(
                "hylafax JSUBM failed ({code}): {rest}"
            )));
        }
        // "Job 6 submitted." -> provider job id 6
        let id = rest
            .split_whitespace()
            .nth(1)
            .map(|s| s.trim_end_matches('.').to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| FaxError::external(format!("hylafax JSUBM missing job id: {rest}")))?;
        self.set_state(TransportState::Submitted);
        FaxCarrierJobId::new(id)
    }

    fn query_job(&self, job_id: &str) -> Result<String, FaxError> {
        self.state()
            .require(TransportState::Authenticated, "job query")?;
        // LIST is a DOWNLOAD on the same active data-channel primitive
        // (observed: EPRT -> 150 -> server connects -> client reads
        // plaintext rows -> 226). MODE Z is NOT applied to LIST even
        // when the session has it set.
        let (_rest, payload) =
            self.data_exchange("LIST sendq", DataDirection::Download, false, None)?;
        let text = String::from_utf8(payload)
            .map_err(|_| FaxError::external("hylafax LIST payload is not utf-8"))?;
        // Strict row parser: the pinned fixture emits one row per job
        // with a numeric leading job id. Any malformed row fails the
        // query closed (the JOBFMT layout is provider configuration,
        // never trusted implicitly).
        let mut found = None;
        for line in text.lines() {
            let row = line.trim_end_matches(['\r', '\n']);
            if row.trim().is_empty() {
                continue;
            }
            let first = row.split_whitespace().next().unwrap_or("");
            if !first.chars().all(|c| c.is_ascii_digit()) {
                return Err(FaxError::external(format!(
                    "hylafax LIST row with malformed job id {first:?}"
                )));
            }
            if first == job_id {
                found = Some(row.to_string());
            }
        }
        found.ok_or_else(|| {
            FaxError::not_found(format!("hylafax job {job_id} not present in send queue"))
        })
    }

    fn quit(&self) -> Result<(), FaxError> {
        let _ = self.command("QUIT");
        // The session is over; reset the state machine so a fresh
        // session may be opened on the same transport (e.g. readback).
        self.set_state(TransportState::Disconnected);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep027_unit_hylafax_parse_response() {
        let (code, rest) =
            parse_response("220 25024d3f7515 server (HylaFAX (tm) Version 6.0.6) ready.\r\n")
                .expect("parse");
        assert_eq!(code, 220);
        assert!(rest.contains("ready"));
        let (code, rest) =
            parse_response("200 New job created: jobid: 6 groupid: 6.\r\n").expect("parse");
        assert_eq!(code, 200);
        assert!(rest.contains("jobid: 6"));
        assert!(parse_response("garbage").is_err());
        assert!(parse_response("").is_err());
    }

    #[test]
    fn ep027_unit_hylafax_state_machine_fails_closed() {
        // A fresh transport is Disconnected; every operation without a
        // connection fails closed with unavailable.
        let t = HylaFaxTcpTransport::new("127.0.0.1", 1, "u", "p");
        assert_eq!(t.state(), TransportState::Disconnected);
        assert!(t.command("JNEW").is_err());
        assert!(t.prepare_transfer().is_err());
        assert!(t.create_job().is_err());
        assert!(t.upload_document(b"x").is_err());
        assert!(t.query_job("1").is_err());
    }

    #[test]
    fn ep027_unit_hylafax_job_id_parsing() {
        // Parse the exact observed JNEW response shape.
        let rest = "New job created: jobid: 6 groupid: 6.";
        let job_id = rest
            .split("jobid:")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .map(|s| s.trim_end_matches('.').to_string());
        assert_eq!(job_id.as_deref(), Some("6"));
        // Parse the exact observed JSUBM response shape.
        let rest = "Job 6 submitted.";
        let id = rest
            .split_whitespace()
            .nth(1)
            .map(|s| s.trim_end_matches('.').to_string());
        assert_eq!(id.as_deref(), Some("6"));
    }

    #[test]
    fn ep027_unit_hylafax_list_row_parser_fails_closed() {
        // The parser is the same strict logic used by query_job: a
        // numeric leading job id is required, the exact target must
        // match, and malformed rows must fail closed.
        fn parse_row(line: &str, target: &str) -> Result<Option<String>, FaxError> {
            let row = line.trim_end_matches(['\r', '\n']);
            if row.trim().is_empty() {
                return Ok(None);
            }
            let first = row.split_whitespace().next().unwrap_or("");
            if !first.chars().all(|c| c.is_ascii_digit()) {
                return Err(FaxError::external(format!(
                    "hylafax LIST row with malformed job id {first:?}"
                )));
            }
            if first == target {
                Ok(Some(row.to_string()))
            } else {
                Ok(None)
            }
        }

        // Observed fixture rows (exact bytes from LIST sendq).
        let row = "22   127 W nexust +155****0200     0:0   0:12         ";
        assert!(parse_row(row, "22").unwrap().is_some());
        assert!(parse_row(row, "21").unwrap().is_none());
        let row_b = "7    127 B   root 15551234567   0:0   0:12         Blocked by concurrent cal";
        assert!(parse_row(row_b, "7").unwrap().is_some());
        // Malformed rows fail closed.
        assert!(parse_row("header line", "22").is_err());
        assert!(parse_row("12x   127 W", "12").is_err());
        assert!(parse_row("", "22").unwrap().is_none());
    }
}
