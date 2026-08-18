//! EP-025 M4 real ARI WebSocket events consumer (SPEC-014; directive A).
//!
//! Asterisk 22 delivers authoritative terminal CALL OUTCOMES only in the
//! ARI event stream: a 486/603 final response destroys the outbound
//! channel before REST polling can observe any intermediate state, and
//! the only typed discriminator is `ChannelDestroyed.cause` (17 = User
//! Busy, 21 = Call Rejected, 18/19 = No Answer, ...). REST alone cannot
//! classify BUSY vs REJECTED vs NO_ANSWER (observed: 20 ms polling still
//! misses the terminal state; the cause arrives only in the event).
//!
//! This module implements a MINIMAL REAL RFC6455 WebSocket client over
//! `std::net::TcpStream` (no new framework dependency; the wire format is
//! verified byte-for-byte against a live capture, EP-022 D-Bus precedent):
//!
//!   1. TCP connect to the ARI HTTP endpoint;
//!   2. HTTP/1.1 Upgrade handshake with `api_key` + `app` query params
//!      and a real Sec-WebSocket-Key;
//!   3. validates Sec-WebSocket-Accept = base64(SHA1(key + RFC6455 GUID));
//!   4. parses server frames (FIN/opcode/7-16-64-bit lengths, unmasked
//!      server frames), replies PONG to PING, tolerates continuation;
//!   5. delivers JSON events to a bounded store (recent ring + per-channel
//!      terminal cause map).
//!
//! The store is bounded: the cause map is pruned to the most recent
//! `max_causes` entries and the ring to `max_events`. Raw audio,
//! credentials, and Authorization headers never enter the store.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use serde_json::Value;

use nexus_telephony::{CallError, CallErrorCode};

/// RFC6455 magic GUID used for the accept digest.
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Bound on the terminal-cause map (channel id -> cause).
const MAX_CAUSES: usize = 1024;
/// Bound on the recent-events ring.
const MAX_EVENTS: usize = 512;

/// One parsed ARI event with typed accessors.
#[derive(Debug, Clone)]
pub struct AriEvent {
    /// Raw event JSON (schema-validated by Asterisk).
    pub raw: Value,
}

impl AriEvent {
    pub fn event_type(&self) -> Option<&str> {
        self.raw.get("type").and_then(Value::as_str)
    }

    pub fn channel_id(&self) -> Option<&str> {
        self.raw
            .get("channel")
            .and_then(|c| c.get("id"))
            .and_then(Value::as_str)
    }

    /// Terminal hangup cause (ChannelHangupRequest / ChannelDestroyed).
    pub fn cause(&self) -> Option<u32> {
        self.raw
            .get("cause")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
    }

    pub fn cause_txt(&self) -> Option<&str> {
        self.raw.get("cause_txt").and_then(Value::as_str)
    }

    pub fn state(&self) -> Option<&str> {
        self.raw
            .get("channel")
            .and_then(|c| c.get("state"))
            .and_then(Value::as_str)
    }
}

/// Bounded store of real ARI events + terminal causes.
#[derive(Debug, Clone, Default)]
pub struct EventStore {
    /// channel id -> (cause, cause_txt) for the most recent terminal
    /// hangup observed per channel (bounded, FIFO-pruned).
    pub causes: HashMap<String, (u32, String)>,
    /// Insertion order of cause keys (FIFO pruning).
    cause_order: VecDeque<String>,
    /// Recent event types with channel ids (bounded ring, oldest first).
    pub recent: VecDeque<String>,
    /// Count of events consumed since start.
    pub consumed: u64,
    /// Whether the consumer currently holds a live WebSocket
    /// subscription (observability, directive G/V): the event stream
    /// may be temporarily unavailable; terminal classification must
    /// NOT fabricate outcomes from a missing stream.
    pub connected: bool,
}

impl EventStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn record(&mut self, event: &AriEvent) {
        self.consumed += 1;
        if let Some(ev_type) = event.event_type() {
            let ch = event.channel_id().unwrap_or("-");
            let line = format!("{ev_type} channel={ch}");
            self.recent.push_back(line);
            if self.recent.len() > MAX_EVENTS {
                self.recent.pop_front();
            }
            if let Some(cause) = event.cause() {
                if let Some(ch) = event.channel_id() {
                    let txt = event.cause_txt().unwrap_or("").to_string();
                    self.causes.insert(ch.to_string(), (cause, txt));
                    if !self.cause_order.contains(&ch.to_string()) {
                        self.cause_order.push_back(ch.to_string());
                    }
                    while self.causes.len() > MAX_CAUSES {
                        // FIFO prune the OLDEST inserted channel.
                        if let Some(oldest) = self.cause_order.pop_front() {
                            self.causes.remove(&oldest);
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// Minimal real RFC6455 client for the ARI events WebSocket.
pub struct AriEventsClient {
    stream: TcpStream,
}

impl AriEventsClient {
    /// Connect + handshake against the REAL ARI events endpoint.
    ///
    /// `host`/`port` are the ARI HTTP listener; `user`/`pass` become the
    /// `api_key` query credential; `app` is the Stasis application name.
    /// The handshake is verified (101 + accept digest) - a wrong
    /// credential or non-WS peer fails closed with a typed error.
    pub fn connect(
        host: &str,
        port: u16,
        user: &str,
        pass: &str,
        app: &str,
        timeout: Duration,
    ) -> Result<Self, CallError> {
        let stream = TcpStream::connect((host, port)).map_err(|e| {
            CallError::new(
                CallErrorCode::Unavailable,
                format!("ari events connect failed: {host}:{port}: {e}"),
                None,
                None,
            )
        })?;
        stream.set_read_timeout(Some(timeout)).map_err(|e| {
            CallError::new(
                CallErrorCode::External,
                format!("ari events set_read_timeout failed: {e}"),
                None,
                None,
            )
        })?;
        stream.set_write_timeout(Some(timeout)).map_err(|e| {
            CallError::new(
                CallErrorCode::External,
                format!("ari events set_write_timeout failed: {e}"),
                None,
                None,
            )
        })?;
        let mut client = Self { stream };
        client.handshake(host, user, pass, app)?;
        Ok(client)
    }

    fn handshake(
        &mut self,
        host: &str,
        user: &str,
        pass: &str,
        app: &str,
    ) -> Result<(), CallError> {
        let key_bytes: [u8; 16] = rand::random();
        let key = B64.encode(key_bytes);
        let path = format!("/ari/events?api_key={user}:{pass}&app={app}");
        let request = format!(
            "GET {path} HTTP/1.1\r\n\
             Host: {host}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {key}\r\n\
             Sec-WebSocket-Version: 13\r\n\
             \r\n"
        );
        self.stream.write_all(request.as_bytes()).map_err(|e| {
            CallError::new(
                CallErrorCode::Unavailable,
                format!("ari events handshake write failed: {e}"),
                None,
                None,
            )
        })?;
        // Read the response head (up to \r\n\r\n), bounded.
        let mut head = Vec::new();
        let mut buf = [0u8; 1024];
        let deadline = Instant::now() + Duration::from_secs(10);
        while !contains_double_crlf(&head) && Instant::now() < deadline {
            let n = self.stream.read(&mut buf).map_err(|e| {
                CallError::new(
                    CallErrorCode::Timeout,
                    format!("ari events handshake read failed: {e}"),
                    None,
                    None,
                )
            })?;
            if n == 0 {
                break;
            }
            head.extend_from_slice(&buf[..n]);
        }
        let head_str = String::from_utf8_lossy(&head);
        if !head_str.starts_with("HTTP/1.1 101") && !head_str.starts_with("HTTP/1.0 101") {
            return Err(CallError::new(
                CallErrorCode::External,
                format!(
                    "ari events handshake rejected (expected 101): {}",
                    first_line(&head_str)
                ),
                None,
                None,
            ));
        }
        let expected = accept_digest(&key);
        let actual = extract_header(&head_str, "sec-websocket-accept");
        match actual {
            Some(value) if value.trim() == expected => Ok(()),
            Some(value) => Err(CallError::new(
                CallErrorCode::External,
                format!("ari events accept digest mismatch (got {value:?})"),
                None,
                None,
            )),
            None => Err(CallError::new(
                CallErrorCode::External,
                "ari events handshake missing Sec-WebSocket-Accept".to_string(),
                None,
                None,
            )),
        }
    }

    /// Read the next text event (blocking, bounded by the read timeout).
    /// PING frames are answered with PONG; CLOSE terminates the stream
    /// with a typed error; fragmented messages are reassembled.
    pub fn next_event(&mut self) -> Result<AriEvent, CallError> {
        let mut fragments: Vec<Vec<u8>> = Vec::new();
        loop {
            let (opcode, fin, payload) = self.read_frame()?;
            match opcode {
                0x1 => {
                    fragments.push(payload);
                    if fin {
                        return self.parse_payload(&fragments.concat());
                    }
                }
                0x2 => {
                    return Err(CallError::new(
                        CallErrorCode::External,
                        "ari events unexpected binary frame".to_string(),
                        None,
                        None,
                    ));
                }
                0x8 => {
                    return Err(CallError::new(
                        CallErrorCode::External,
                        "ari events stream closed by server".to_string(),
                        None,
                        None,
                    ));
                }
                0x9 => {
                    // PING: reply PONG with the same payload.
                    self.write_pong(&payload)?;
                }
                0xA => { /* PONG from server: ignore */ }
                0x0 => {
                    if fragments.is_empty() {
                        return Err(CallError::new(
                            CallErrorCode::External,
                            "ari events orphan continuation frame".to_string(),
                            None,
                            None,
                        ));
                    }
                    fragments.push(payload);
                    if fin {
                        return self.parse_payload(&fragments.concat());
                    }
                }
                other => {
                    return Err(CallError::new(
                        CallErrorCode::External,
                        format!("ari events unknown opcode {other:#x}"),
                        None,
                        None,
                    ));
                }
            }
        }
    }

    fn parse_payload(&self, payload: &[u8]) -> Result<AriEvent, CallError> {
        let value: Value = serde_json::from_slice(payload).map_err(|e| {
            CallError::new(
                CallErrorCode::External,
                format!("ari events malformed JSON: {e}"),
                None,
                None,
            )
        })?;
        Ok(AriEvent { raw: value })
    }

    fn read_exact(&mut self, out: &mut [u8]) -> Result<(), CallError> {
        let mut filled = 0;
        while filled < out.len() {
            let n = self.stream.read(&mut out[filled..]).map_err(|e| {
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut
                {
                    CallError::new(
                        CallErrorCode::Timeout,
                        format!("ari events read timed out: {e}"),
                        None,
                        None,
                    )
                } else {
                    CallError::new(
                        CallErrorCode::External,
                        format!("ari events read failed: {e}"),
                        None,
                        None,
                    )
                }
            })?;
            if n == 0 {
                return Err(CallError::new(
                    CallErrorCode::External,
                    "ari events connection closed".to_string(),
                    None,
                    None,
                ));
            }
            filled += n;
        }
        Ok(())
    }

    fn read_frame(&mut self) -> Result<(u8, bool, Vec<u8>), CallError> {
        let mut header = [0u8; 2];
        self.read_exact(&mut header)?;
        let fin = header[0] & 0x80 != 0;
        let opcode = header[0] & 0x0f;
        let masked = header[1] & 0x80 != 0;
        let len7 = (header[1] & 0x7f) as u64;
        let mut len = len7;
        if len7 == 126 {
            let mut ext = [0u8; 2];
            self.read_exact(&mut ext)?;
            len = u16::from_be_bytes(ext) as u64;
        } else if len7 == 127 {
            let mut ext = [0u8; 8];
            self.read_exact(&mut ext)?;
            len = u64::from_be_bytes(ext);
        }
        if masked {
            // Server-to-client frames are NEVER masked (RFC6455 5.1).
            // A masked server frame indicates a non-conforming peer.
            return Err(CallError::new(
                CallErrorCode::External,
                "ari events masked server frame (protocol violation)".to_string(),
                None,
                None,
            ));
        }
        if len > 16 * 1024 * 1024 {
            return Err(CallError::new(
                CallErrorCode::External,
                format!("ari events frame too large ({len})"),
                None,
                None,
            ));
        }
        let mut payload = vec![0u8; len as usize];
        self.read_exact(&mut payload)?;
        Ok((opcode, fin, payload))
    }

    fn write_pong(&mut self, payload: &[u8]) -> Result<(), CallError> {
        let mut frame = Vec::with_capacity(payload.len() + 2);
        frame.push(0x8A);
        let len = payload.len();
        if len < 126 {
            frame.push(len as u8);
        } else if len < 65536 {
            frame.push(126);
            frame.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            frame.push(127);
            frame.extend_from_slice(&(len as u64).to_be_bytes());
        }
        frame.extend_from_slice(payload);
        self.stream.write_all(&frame).map_err(|e| {
            CallError::new(
                CallErrorCode::External,
                format!("ari events pong write failed: {e}"),
                None,
                None,
            )
        })
    }
}

/// Consume the ARI event stream forever, recording into the store.
/// Bounded reconnect with backoff (directive W: reconnect is a bounded
/// owned action). Runs until the stop flag is set.
pub fn run_event_consumer(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
    app: &str,
    store: std::sync::Arc<std::sync::Mutex<EventStore>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let mut attempt: u64 = 0;
    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(mut client) =
            AriEventsClient::connect(host, port, user, pass, app, Duration::from_secs(30))
        {
            attempt = 0;
            if let Ok(mut s) = store.lock() {
                s.connected = true;
            }
            loop {
                if stop.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                match client.next_event() {
                    Ok(event) => {
                        if let Ok(mut s) = store.lock() {
                            s.record(&event);
                        }
                    }
                    Err(_) => break,
                }
            }
            if let Ok(mut s) = store.lock() {
                s.connected = false;
            }
        }
        // Bounded backoff: 1s * attempt clamped to [1, 15]s.
        let delay = Duration::from_secs(attempt.clamp(1, 15));
        attempt = attempt.saturating_add(1);
        let deadline = Instant::now() + delay;
        while Instant::now() < deadline && !stop.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

fn contains_double_crlf(buf: &[u8]) -> bool {
    buf.windows(4).any(|w| w == b"\r\n\r\n")
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").to_string()
}

fn extract_header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    for line in head.lines().skip(1) {
        let mut parts = line.splitn(2, ':');
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            if k.trim().eq_ignore_ascii_case(name) {
                return Some(v.trim());
            }
        }
    }
    None
}

/// Compute the RFC6455 Sec-WebSocket-Accept digest for a client key.
pub fn accept_digest(key: &str) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WS_GUID.as_bytes());
    B64.encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep025_unit_ws_accept_digest_rfc6455_vector() {
        // RFC6455 1.3 example: key "dGhlIHNhbXBsZSBub25jZQ==" ->
        // accept "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        assert_eq!(
            accept_digest("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn ep025_unit_ws_header_extraction() {
        let head = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nSec-WebSocket-Accept: abc\r\n\r\n";
        assert_eq!(extract_header(head, "sec-websocket-accept"), Some("abc"));
        assert_eq!(extract_header(head, "upgrade"), Some("websocket"));
        assert!(contains_double_crlf(head.as_bytes()));
    }

    #[test]
    fn ep025_unit_ws_event_store_bounded() {
        let mut store = EventStore::new();
        let ev = |t: &str, id: &str, cause: Option<u32>| -> AriEvent {
            let mut raw = serde_json::json!({"type": t, "channel": {"id": id, "state": "Up"}});
            if let Some(c) = cause {
                raw["cause"] = serde_json::json!(c);
                raw["cause_txt"] = serde_json::json!("test");
            }
            AriEvent { raw }
        };
        for i in 0..2000 {
            store.record(&ev("ChannelDestroyed", &format!("ch{i}"), Some(17)));
        }
        assert!(store.causes.len() <= MAX_CAUSES);
        assert!(store.recent.len() <= MAX_EVENTS);
        assert_eq!(store.consumed, 2000);
        assert_eq!(store.causes.get("ch1999"), Some(&(17, "test".to_string())));
    }

    #[test]
    fn ep025_unit_ws_cause_mapping_helpers() {
        let mut store = EventStore::new();
        store.record(&AriEvent {
            raw: serde_json::json!({
                "type": "ChannelDestroyed",
                "channel": {"id": "abc", "state": "Down"},
                "cause": 17,
                "cause_txt": "User busy"
            }),
        });
        assert_eq!(
            store.causes.get("abc"),
            Some(&(17, "User busy".to_string()))
        );
        assert_eq!(store.recent.back().unwrap(), "ChannelDestroyed channel=abc");
    }

    /// LIVE-STACK (ignored): connect the minimal RFC6455 client to the
    /// REAL Asterisk ARI events endpoint and confirm it receives a real
    /// event. Requires the fixture env (/tmp/ep025-ast.env) and a
    /// running Asterisk container.
    #[test]
    #[ignore]
    fn ep025_live_ws_events_connect_real_asterisk() {
        let env = std::fs::read_to_string("/tmp/ep025-ast.env").unwrap();
        let mut user = String::new();
        let mut pass = String::new();
        for line in env.lines() {
            if let Some(v) = line.strip_prefix("NEXUS_ARI_USER=") {
                user = v.to_string();
            }
            if let Some(v) = line.strip_prefix("NEXUS_ARI_PASSWORD=") {
                pass = v.to_string();
            }
        }
        assert!(!user.is_empty() && !pass.is_empty());
        let mut client = AriEventsClient::connect(
            "127.0.0.1",
            8088,
            &user,
            &pass,
            "nexus-telephony",
            Duration::from_secs(10),
        )
        .expect("handshake against real Asterisk");
        // Trigger a real event: originate a call to endpoint-d (auto
        // answer) and expect at least one event (StasisStart etc).
        let _ = std::process::Command::new("curl")
            .args([
                "-s",
                "-u",
                &format!("{user}:{pass}"),
                "-X",
                "POST",
                "http://127.0.0.1:8088/ari/channels?endpoint=PJSIP/endpoint-d&app=nexus-telephony&appArgs=wsprobe&callerId=wsprobe&timeout=15",
            ])
            .output();
        let event = client.next_event().expect("real event within read timeout");
        assert!(event.event_type().is_some());
        assert!(event.raw.is_object());
        // Cleanup: hang up the probe channel.
        if let Some(cid) = event.channel_id() {
            let _ = std::process::Command::new("curl")
                .args([
                    "-s",
                    "-u",
                    &format!("{user}:{pass}"),
                    "-X",
                    "DELETE",
                    &format!("http://127.0.0.1:8088/ari/channels/{cid}"),
                ])
                .output();
        }
    }
}
