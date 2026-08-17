//! Minimal REAL D-Bus client over a Unix socket.
//!
//! Implements exactly the D-Bus wire surface needed by the Bluetooth
//! probe: SASL EXTERNAL authentication and
//! org.freedesktop.DBus.GetNameOwner. This is real wire traffic on a
//! real Unix socket against a real bus daemon - not a mock and not a
//! canned result. The replacement boundary is the full zbus/bluer
//! client libraries (recorded in the EP-022 Decision Log); this module
//! exists so the connector owns its transport with no heavyweight
//! dependency tree.

use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// Errors from the real D-Bus wire exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusError {
    /// Could not open or talk to the bus socket at all.
    Connect(String),
    /// The bus rejected our credentials.
    Auth(String),
    /// A real read deadline elapsed.
    Timeout,
    /// The bus reports the name has no owner (real forced failure).
    NameHasNoOwner,
    /// The bus returned a malformed or unexpected message.
    Malformed(String),
    /// A well-formed D-Bus error reply with an error name.
    Call { error_name: String, message: String },
    /// Raw I/O failure.
    Io(String),
}

impl std::fmt::Display for BusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(s) => write!(f, "bus connect failed: {s}"),
            Self::Auth(s) => write!(f, "bus auth rejected: {s}"),
            Self::Timeout => f.write_str("bus timed out"),
            Self::NameHasNoOwner => f.write_str("name has no owner on the bus"),
            Self::Malformed(s) => write!(f, "malformed D-Bus message: {s}"),
            Self::Call {
                error_name,
                message,
            } => write!(f, "D-Bus error {error_name}: {message}"),
            Self::Io(s) => write!(f, "D-Bus I/O failure: {s}"),
        }
    }
}

impl std::error::Error for BusError {}

/// A real, blocking D-Bus client for the single call we need.
pub struct DbusClient {
    stream: UnixStream,
    serial: u32,
    /// Persistent receive buffer: the daemon may batch a reply with a
    /// signal (e.g. NameAcquired after Hello) in one read, so leftover
    /// bytes must survive across calls.
    recv_buf: Vec<u8>,
}

impl DbusClient {
    /// Connect to a bus address ("unix:path=..." or a plain socket
    /// path), complete SASL EXTERNAL authentication against the real
    /// daemon, and register with the mandatory Hello call (real
    /// clients must be registered before ordinary calls; the bus
    /// denies unregistered connections).
    pub fn connect(address: &str, timeout: Duration) -> Result<Self, BusError> {
        let path = bus_address_path(address)?;
        let mut stream =
            UnixStream::connect(&path).map_err(|e| BusError::Connect(format!("{path}: {e}")))?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|e| BusError::Io(e.to_string()))?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|e| BusError::Io(e.to_string()))?;
        authenticate(&mut stream)?;
        let mut client = Self {
            stream,
            serial: 1,
            recv_buf: Vec::new(),
        };
        let unique = client.hello()?;
        if unique.is_empty() {
            return Err(BusError::Connect(
                "bus returned an empty unique name".into(),
            ));
        }
        Ok(client)
    }

    /// Register with the bus (Hello) and return the unique name.
    pub fn hello(&mut self) -> Result<String, BusError> {
        self.call("org.freedesktop.DBus", "Hello", &[])
    }

    /// Resolve a well-known name through the real bus. Returns the
    /// owner string, or `BusError::NameHasNoOwner` when the name is
    /// not owned (the real BlueZ-absent mechanism).
    pub fn get_name_owner(&mut self, name: &str) -> Result<String, BusError> {
        let mut body = Vec::new();
        push_string(&mut body, name);
        self.call("org.freedesktop.DBus", "GetNameOwner", &body)
    }

    fn call(&mut self, destination: &str, member: &str, body: &[u8]) -> Result<String, BusError> {
        let serial = self.serial;
        self.serial = self.serial.wrapping_add(1);
        let message = build_method_call(serial, destination, member, body);
        self.stream
            .write_all(&message)
            .map_err(|e| BusError::Io(e.to_string()))?;
        self.read_reply(serial)
    }

    fn read_reply(&mut self, serial: u32) -> Result<String, BusError> {
        loop {
            if let Some((reply, consumed)) = parse_message(&self.recv_buf)? {
                self.recv_buf.drain(..consumed);
                match reply.msg_type {
                    // METHOD_RETURN or ERROR: the reply to our call.
                    2 | 3 => {
                        if let Some(reply_serial) = reply.reply_serial {
                            if reply_serial != serial {
                                return Err(BusError::Malformed(format!(
                                    "reply serial mismatch: got {reply_serial}, want {serial}"
                                )));
                            }
                        }
                        return reply_to_owner(reply);
                    }
                    // Signals and anything else are skipped; the real
                    // reply will follow.
                    _ => continue,
                }
            }
            let mut tmp = [0u8; 4096];
            match self.stream.read(&mut tmp) {
                Ok(0) => return Err(BusError::Connect("bus closed the connection".into())),
                Ok(n) => self.recv_buf.extend_from_slice(&tmp[..n]),
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    return Err(BusError::Timeout);
                }
                Err(e) => return Err(BusError::Io(e.to_string())),
            }
        }
    }
}

fn bus_address_path(address: &str) -> Result<String, BusError> {
    if let Some(rest) = address.strip_prefix("unix:path=") {
        Ok(rest.to_string())
    } else if address.contains(':') {
        Err(BusError::Connect(format!(
            "unsupported bus address: {address}"
        )))
    } else {
        Ok(address.to_string())
    }
}

fn authenticate(stream: &mut UnixStream) -> Result<(), BusError> {
    // EXTERNAL auth data is the hex encoding of the decimal uid
    // string: uid 0 -> "0" -> bytes [0x30] -> "30" (matches the dbus
    // reference implementation).
    let uid = std::fs::metadata("/proc/self")
        .map(|m| m.uid())
        .unwrap_or(0);
    let hex_uid: String = uid
        .to_string()
        .bytes()
        .map(|b| format!("{b:02x}"))
        .collect();
    let mut auth = Vec::new();
    auth.push(0u8); // initial NUL byte required by the D-Bus auth protocol
    auth.extend_from_slice(format!("AUTH EXTERNAL {hex_uid}\r\n").as_bytes());
    stream
        .write_all(&auth)
        .map_err(|e| BusError::Io(e.to_string()))?;
    let line = read_line(stream)?;
    if line.starts_with(b"OK") {
        stream
            .write_all(b"BEGIN\r\n")
            .map_err(|e| BusError::Io(e.to_string()))?;
        Ok(())
    } else if line.starts_with(b"REJECTED") {
        Err(BusError::Auth(String::from_utf8_lossy(&line).into_owned()))
    } else {
        Err(BusError::Malformed(format!(
            "unexpected auth response: {}",
            String::from_utf8_lossy(&line)
        )))
    }
}

fn read_line(stream: &mut UnixStream) -> Result<Vec<u8>, BusError> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => return Err(BusError::Connect("bus closed during auth".into())),
            Ok(_) => {
                buf.push(byte[0]);
                if buf.ends_with(b"\r\n") {
                    buf.truncate(buf.len() - 2);
                    return Ok(buf);
                }
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                return Err(BusError::Timeout);
            }
            Err(e) => return Err(BusError::Io(e.to_string())),
        }
    }
}

fn align_up(value: usize, alignment: usize) -> usize {
    value.div_ceil(alignment) * alignment
}

fn align(buf: &mut Vec<u8>, alignment: usize) {
    while !buf.len().is_multiple_of(alignment) {
        buf.push(0);
    }
}

fn push_byte(buf: &mut Vec<u8>, b: u8) {
    buf.push(b);
}

fn push_u32(buf: &mut Vec<u8>, v: u32) {
    align(buf, 4);
    buf.extend_from_slice(&v.to_le_bytes());
}

fn push_string(buf: &mut Vec<u8>, s: &str) {
    push_u32(buf, s.len() as u32);
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
}

fn push_signature(buf: &mut Vec<u8>, s: &str) {
    buf.push(s.len() as u8);
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
}

fn build_method_call(serial: u32, destination: &str, member: &str, body: &[u8]) -> Vec<u8> {
    // Header fields, ordered by field code like the reference
    // implementation: PATH, INTERFACE, MEMBER, DESTINATION, plus
    // SIGNATURE when the body is non-empty. Wire layout (verified
    // against a real dbus-send capture): the fields array length sits
    // at offset 12 (right after the 12-byte fixed header), the first
    // element starts at offset 16, elements are 8-aligned, and the
    // length counts from the first element start to the end of the
    // last element (no leading or trailing pad).
    let mut fields: Vec<(u8, &str, &str)> = vec![
        (1, "o", "/org/freedesktop/DBus"), // PATH
        (2, "s", "org.freedesktop.DBus"),  // INTERFACE
        (3, "s", member),                  // MEMBER
        (6, "s", destination),             // DESTINATION
    ];
    if !body.is_empty() {
        fields.push((8, "g", "s")); // SIGNATURE of the body
    }
    let mut msg = vec![0x6C, 1, 0, 1]; // endian, METHOD_CALL, flags, version
    msg.extend_from_slice(&[0u8; 8]); // body length + serial placeholders
    let len_pos = msg.len(); // 12
    push_u32(&mut msg, 0); // array length placeholder (at offset 12)
    let content_start = msg.len(); // 16
    for (code, signature, value) in fields {
        align(&mut msg, 8); // element alignment
        push_byte(&mut msg, code);
        push_signature(&mut msg, signature);
        if signature == "o" || signature == "s" {
            align(&mut msg, 4);
            push_string(&mut msg, value);
        } else {
            // "g" (signature value): alignment 1
            push_signature(&mut msg, value);
        }
    }
    let array_len = msg.len() - content_start;
    msg[len_pos..len_pos + 4].copy_from_slice(&(array_len as u32).to_le_bytes());
    align(&mut msg, 8); // body alignment
    let body_start = msg.len();
    msg.extend_from_slice(body);
    let body_len = msg.len() - body_start;
    msg[4..8].copy_from_slice(&(body_len as u32).to_le_bytes());
    msg[8..12].copy_from_slice(&serial.to_le_bytes());
    // No trailing padding: the message ends exactly at the body end
    // (verified byte-for-byte against a real dbus-send capture). Any
    // extra trailing bytes would be parsed by the daemon as the start
    // of the next message and close the connection.
    msg
}

struct ParsedReply {
    msg_type: u8,
    error_name: Option<String>,
    reply_serial: Option<u32>,
    reply_signature: Option<String>,
    body: Vec<u8>,
}

enum VariantValue {
    String(String),
    U32(u32),
    Skipped,
}

/// Parse a D-Bus message; returns None when the buffer does not yet
/// hold the whole message, else the reply plus the exact number of
/// bytes consumed (so the caller can drain without losing any bytes
/// the daemon batched behind this message).
fn parse_message(buf: &[u8]) -> Result<Option<(ParsedReply, usize)>, BusError> {
    if buf.len() < 12 {
        return Ok(None);
    }
    if buf[0] != 0x6C {
        return Err(BusError::Malformed(
            "big-endian message not supported".into(),
        ));
    }
    let msg_type = buf[1];
    let body_len = u32::from_le_bytes(buf[4..8].try_into().unwrap()) as usize;
    // The fields array length sits at offset 12 (right after the fixed
    // header); the first element starts at offset 16. Verified against
    // a real dbus-send capture.
    if buf.len() < 16 {
        return Ok(None);
    }
    let arr_len = u32::from_le_bytes(buf[12..16].try_into().unwrap()) as usize;
    let elements_start = 16;
    let elements_end = elements_start + arr_len;
    let body_start = align_up(elements_end, 8);
    let total = body_start + body_len;
    if buf.len() < total {
        return Ok(None);
    }
    let mut cursor = Cursor {
        buf,
        pos: elements_start,
        end: elements_end,
    };
    let mut error_name = None;
    let mut reply_serial = None;
    let mut reply_signature = None;
    while cursor.pos < cursor.end {
        cursor.align(8)?;
        if cursor.pos >= cursor.end {
            break;
        }
        let code = cursor.u8()?;
        let signature = cursor.signature()?;
        let value = cursor.variant_value(&signature)?;
        match code {
            4 => {
                if let VariantValue::String(s) = value {
                    error_name = Some(s);
                }
            }
            5 => {
                if let VariantValue::U32(v) = value {
                    reply_serial = Some(v);
                }
            }
            8 => {
                if let VariantValue::String(s) = value {
                    reply_signature = Some(s);
                }
            }
            _ => {}
        }
    }
    let body = buf[body_start..total].to_vec();
    Ok(Some((
        ParsedReply {
            msg_type,
            error_name,
            reply_serial,
            reply_signature,
            body,
        },
        total,
    )))
}

fn reply_to_owner(reply: ParsedReply) -> Result<String, BusError> {
    match reply.msg_type {
        2 => {
            // METHOD_RETURN: GetNameOwner body is a single string.
            let signature = reply.reply_signature.as_deref().unwrap_or("s");
            if signature != "s" {
                return Err(BusError::Malformed(format!(
                    "unexpected reply signature {signature}"
                )));
            }
            let mut cursor = Cursor {
                buf: &reply.body,
                pos: 0,
                end: reply.body.len(),
            };
            cursor.align(4)?;
            cursor.string()
        }
        3 => {
            // ERROR reply: error name in header, message string body.
            let name = reply
                .error_name
                .unwrap_or_else(|| "org.freedesktop.DBus.Error.Failed".to_string());
            let message = {
                let mut cursor = Cursor {
                    buf: &reply.body,
                    pos: 0,
                    end: reply.body.len(),
                };
                cursor
                    .align(4)
                    .ok()
                    .and_then(|_| cursor.string().ok())
                    .unwrap_or_default()
            };
            match name.as_str() {
                "org.freedesktop.DBus.Error.NameHasNoOwner"
                | "org.freedesktop.DBus.Error.ServiceUnknown" => Err(BusError::NameHasNoOwner),
                "org.freedesktop.DBus.Error.AccessDenied" => {
                    Err(BusError::Auth("access denied by bus policy".into()))
                }
                _ => Err(BusError::Call {
                    error_name: name,
                    message,
                }),
            }
        }
        other => Err(BusError::Malformed(format!(
            "unexpected message type {other}"
        ))),
    }
}

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
    end: usize,
}

impl<'a> Cursor<'a> {
    fn align(&mut self, alignment: usize) -> Result<(), BusError> {
        self.pos = align_up(self.pos, alignment);
        if self.pos > self.end {
            return Err(BusError::Malformed("value runs past buffer end".into()));
        }
        Ok(())
    }

    fn u8(&mut self) -> Result<u8, BusError> {
        if self.pos + 1 > self.end {
            return Err(BusError::Malformed("short read".into()));
        }
        let value = self.buf[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, BusError> {
        self.align(4)?;
        if self.pos + 4 > self.end {
            return Err(BusError::Malformed("short read".into()));
        }
        let value = u32::from_le_bytes(self.buf[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Ok(value)
    }

    fn string(&mut self) -> Result<String, BusError> {
        let len = self.u32()? as usize;
        if self.pos + len + 1 > self.end {
            return Err(BusError::Malformed("short string".into()));
        }
        let value = String::from_utf8_lossy(&self.buf[self.pos..self.pos + len]).into_owned();
        self.pos += len + 1;
        Ok(value)
    }

    fn signature(&mut self) -> Result<String, BusError> {
        let len = self.u8()? as usize;
        if self.pos + len + 1 > self.end {
            return Err(BusError::Malformed("short signature".into()));
        }
        let value = String::from_utf8_lossy(&self.buf[self.pos..self.pos + len]).into_owned();
        self.pos += len + 1;
        Ok(value)
    }

    fn variant_value(&mut self, signature: &str) -> Result<VariantValue, BusError> {
        match signature {
            "s" | "o" => {
                self.align(4)?;
                Ok(VariantValue::String(self.string()?))
            }
            "g" => {
                // Signature values are u8 length + bytes + NUL at
                // alignment 1 (not strings).
                Ok(VariantValue::String(self.signature()?))
            }
            "u" => {
                self.align(4)?;
                Ok(VariantValue::U32(self.u32()?))
            }
            "y" => {
                let _ = self.u8()?;
                Ok(VariantValue::Skipped)
            }
            other => Err(BusError::Malformed(format!(
                "unsupported variant signature {other}"
            ))),
        }
    }
}
