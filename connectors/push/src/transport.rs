//! EP-032 push transport (M2): real byte-level transport for the
//! mobile push channel.
//!
//! The transport writes the canonical `NotificationEnvelope` as a JSON
//! line and reads one JSON ack line from the peer. The ack wire shape
//! is owned and documented by this connector (anti-hallucination: no
//! external push provider API is invented or claimed):
//!
//! ```json
//! {"provider_ref":"...","delivered":true,"delivered_at_ms":123,"error":null}
//! ```
//!
//! The transport works over any duplex byte source (socket, pipe,
//! file). Malformed or unknown acks fail closed (External) and are
//! never guessed.

use std::io::{BufRead, BufReader, Read, Write};

use nexus_domain::CorrelationId;
use nexus_notifications::{NotificationEnvelope, NotificationError, NotificationErrorCode};

/// A normalized provider ack (documented connector wire shape).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PushAck {
    /// Provider-side reference for the delivery.
    pub provider_ref: String,
    /// Whether the provider reports delivery.
    pub delivered: bool,
    /// Provider timestamp in epoch milliseconds, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivered_at_ms: Option<u64>,
    /// Provider error detail (never the notification body).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The push transport boundary: write one envelope, read one ack.
pub trait PushTransport {
    /// Deliver an envelope over the wire and return the ack.
    fn deliver(&mut self, envelope: &NotificationEnvelope) -> Result<PushAck, NotificationError>;
}

/// Real JSON-lines push transport over an arbitrary duplex byte
/// source (socket, pipe, file).
#[derive(Debug)]
pub struct JsonPushTransport<R, W> {
    reader: BufReader<R>,
    writer: W,
    correlation: CorrelationId,
}

impl<R: Read, W: Write> JsonPushTransport<R, W> {
    pub fn new(reader: R, writer: W, correlation: CorrelationId) -> Self {
        Self {
            reader: BufReader::new(reader),
            writer,
            correlation,
        }
    }
}

impl<R: Read, W: Write> PushTransport for JsonPushTransport<R, W> {
    fn deliver(&mut self, envelope: &NotificationEnvelope) -> Result<PushAck, NotificationError> {
        let wire = serde_json::to_string(envelope).map_err(|e| {
            NotificationError::new(
                NotificationErrorCode::Internal,
                format!("envelope serialization failed: {e}"),
                Some(self.correlation.as_str().to_string()),
                None,
                None,
                Some("NotificationEnvelope".to_string()),
            )
        })?;
        writeln!(self.writer, "{wire}").map_err(|e| {
            NotificationError::new(
                NotificationErrorCode::External,
                format!("push transport write failed: {e}"),
                Some(self.correlation.as_str().to_string()),
                None,
                None,
                Some("push transport".to_string()),
            )
        })?;
        self.writer.flush().map_err(|e| {
            NotificationError::new(
                NotificationErrorCode::External,
                format!("push transport flush failed: {e}"),
                Some(self.correlation.as_str().to_string()),
                None,
                None,
                Some("push transport".to_string()),
            )
        })?;
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).map_err(|e| {
            NotificationError::new(
                NotificationErrorCode::External,
                format!("push transport read failed: {e}"),
                Some(self.correlation.as_str().to_string()),
                None,
                None,
                Some("push transport".to_string()),
            )
        })?;
        if n == 0 {
            return Err(NotificationError::new(
                NotificationErrorCode::External,
                "push transport closed before ack",
                Some(self.correlation.as_str().to_string()),
                None,
                None,
                Some("push transport".to_string()),
            ));
        }
        let ack: PushAck = serde_json::from_str(line.trim()).map_err(|e| {
            NotificationError::new(
                NotificationErrorCode::External,
                format!("malformed push ack: {e}"),
                Some(self.correlation.as_str().to_string()),
                None,
                None,
                Some("push ack".to_string()),
            )
        })?;
        Ok(ack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::{NotificationChannel, PersonId, Privacy};
    use nexus_notifications::{NotificationId, NotificationUrgency};

    fn envelope() -> NotificationEnvelope {
        NotificationEnvelope::new(
            NotificationId::new("n-1").unwrap(),
            PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            NotificationUrgency::High,
            Privacy::Personal,
            "Suspicious sign-in",
            "A new device signed in to your account.",
            vec![NotificationChannel::MobilePush],
            "2026-08-21T12:00:00Z",
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
        )
        .unwrap()
    }

    /// Real std::net socket pair helper: bind a listener, connect a
    /// client, hand the accepted peer stream to the closure. Returns
    /// the client plus the peer handle (join AFTER the client I/O).
    fn duplex_pair<F>(peer: F) -> (std::net::TcpStream, std::thread::JoinHandle<()>)
    where
        F: FnOnce(std::net::TcpStream) + Send + 'static,
    {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (b, _) = listener.accept().unwrap();
            peer(b);
        });
        let client = std::net::TcpStream::connect(addr).unwrap();
        (client, handle)
    }

    #[test]
    fn ep032_unit_push_transport_roundtrip_over_real_duplex() {
        // Real std::net duplex socket pair; the peer reads the
        // envelope line and writes a real ack back.
        let correlation = CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap();
        let (client, handle) = duplex_pair(|mut b| {
            let mut line = String::new();
            let mut reader = BufReader::new(b.try_clone().unwrap());
            reader.read_line(&mut line).unwrap();
            let env: NotificationEnvelope = serde_json::from_str(line.trim()).unwrap();
            assert_eq!(env.notification_id.as_str(), "n-1");
            assert_eq!(env.channels, vec![NotificationChannel::MobilePush]);
            writeln!(
                b,
                "{{\"provider_ref\":\"p-1\",\"delivered\":true,\"delivered_at_ms\":1700000000000,\"error\":null}}"
            )
            .unwrap();
            b.flush().unwrap();
        });
        let mut transport = JsonPushTransport::new(
            client.try_clone().unwrap(),
            client.try_clone().unwrap(),
            correlation,
        );
        let ack = transport.deliver(&envelope()).unwrap();
        assert!(ack.delivered);
        assert_eq!(ack.provider_ref, "p-1");
        assert_eq!(ack.delivered_at_ms, Some(1_700_000_000_000));
        handle.join().unwrap();
    }

    #[test]
    fn ep032_unit_push_transport_malformed_ack_fails_closed() {
        let correlation = CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap();
        let (client, handle) = duplex_pair(|mut b| {
            let mut line = String::new();
            let mut reader = BufReader::new(b.try_clone().unwrap());
            reader.read_line(&mut line).unwrap();
            writeln!(b, "this is not json").unwrap();
            b.flush().unwrap();
        });
        let mut transport = JsonPushTransport::new(
            client.try_clone().unwrap(),
            client.try_clone().unwrap(),
            correlation,
        );
        let err = transport.deliver(&envelope()).unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::External);
        assert!(err.correlation.is_some());
        handle.join().unwrap();
    }

    #[test]
    fn ep032_unit_push_transport_peer_closed_fails_closed() {
        let correlation = CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap();
        let (client, handle) = duplex_pair(|b| {
            let mut line = String::new();
            let mut reader = BufReader::new(b.try_clone().unwrap());
            reader.read_line(&mut line).unwrap();
            // Drop b without ack.
            drop(b);
        });
        let mut transport = JsonPushTransport::new(
            client.try_clone().unwrap(),
            client.try_clone().unwrap(),
            correlation,
        );
        let err = transport.deliver(&envelope()).unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::External);
        handle.join().unwrap();
    }

    #[test]
    fn ep032_unit_push_transport_ack_with_unknown_field_rejected() {
        // deny_unknown_fields on PushAck: an unknown ack field is
        // rejected rather than guessed.
        let json = "{\"provider_ref\":\"p\",\"delivered\":true,\"mystery\":1}";
        let res: Result<PushAck, _> = serde_json::from_str(json);
        assert!(res.is_err());
        // Documented shape parses.
        let ack: PushAck = serde_json::from_str(
            "{\"provider_ref\":\"p\",\"delivered\":false,\"error\":\"cooldown\"}",
        )
        .unwrap();
        assert!(!ack.delivered);
        assert_eq!(ack.error.as_deref(), Some("cooldown"));
    }
}
