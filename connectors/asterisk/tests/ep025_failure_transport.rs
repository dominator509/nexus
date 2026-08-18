//! EP-025 M4 transport failure classification (SPEC-014 directives
//! E/F/AB/U) proven over REAL sockets against a local fake Asterisk
//! HTTP server.
//!
//! The fake server is permitted for PARSER FAILURE and empty-body
//! cases only (directive 2): it never certifies Asterisk integration.
//! Every test uses a real TcpListener and real HTTP/1.1 responses
//! written over a real socket (std::io), with a fresh ephemeral port
//! per test.
//!
//! Proven here:
//!   E  silent peer (accepted, never responds) -> Timeout; refused
//!      port -> Unavailable. The two failure modes are DISTINCT and
//!      never conflated (a closed port is NOT a timeout).
//!   F  status-only 200 with an EMPTY body is success for ARI actions
//!      (answer, dtmf, moh start/stop, continue, redirect, addChannel)
//!      because those go through the status-only post()/delete()
//!      helper; the SAME empty body on a structured GET (channel
//!      state) fails closed as External because structured JSON is
//!      REQUIRED there.
//!   AB HTTP 401 -> Authorization, 404 -> NotFound, 409 -> Conflict,
//!      503 -> Unavailable, malformed JSON -> External.
//!   U  the ARI password (a distinct canary) never appears in any
//!      error Display or serialized JSON surface.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use nexus_asterisk::transport::{AriTransport, ChannelSelector, RestAriTransport};
use nexus_telephony::{CallError, CallErrorCode};

/// Bounded production-style request timeout for every test.
const TIMEOUT: Duration = Duration::from_secs(2);

/// Distinct canary password: if this string ever appears on an error
/// surface, the transport leaked credentials (directive U).
const CANARY: &str = "EP025PW_CANARY_7f3a";

/// Channel id used by the channel-scoped tests.
const CHANNEL_ID: &str = "PJSIP/ep025-c-00000001";

/// Malformed JSON body used by the parser-failure tests.
const GARBAGE_BODY: &str = "this is not json at all {";

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        401 => "Unauthorized",
        404 => "Not Found",
        409 => "Conflict",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Response",
    }
}

/// Read the HTTP request head (up to the CRLFCRLF terminator) from a
/// socket. Bounded by a 5s read timeout so a broken peer never hangs
/// the server thread.
fn read_request_head(stream: &mut std::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

/// Parse "METHOD SP request-target SP HTTP/1.1" from the head. The
/// query string is stripped so the recorded path is the clean ARI
/// route (reqwest appends ?key=value for POST action params).
fn parse_request_line(head: &str) -> (&str, &str) {
    let line = head.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let raw_path = parts.next().unwrap_or("/");
    let path = raw_path.split('?').next().unwrap_or(raw_path);
    (method, path)
}

/// Spawn a fake Asterisk HTTP server that serves EXACTLY ONE request,
/// then exits (listener drops). `handler` maps (method, path) to
/// (status, content_type, body). Returns the bound port and the
/// server thread handle; join it after the request so the listener
/// drops before the test ends. The accept loop is bounded so a failed
/// test can never leak a blocked thread forever.
fn spawn_server<F>(handler: F) -> (u16, JoinHandle<()>)
where
    F: Fn(&str, &str) -> (u16, &'static str, &'static str) + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake ARI listener");
    let port = listener.local_addr().expect("fake ARI local addr").port();
    let handle = std::thread::spawn(move || {
        listener.set_nonblocking(true).expect("nonblocking accept");
        let deadline = Instant::now() + Duration::from_secs(15);
        let (mut stream, _peer) = loop {
            match listener.accept() {
                Ok(conn) => break conn,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() > deadline {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return,
            }
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let head = read_request_head(&mut stream);
        let (method, path) = parse_request_line(&head);
        let (status, content_type, body) = handler(method, path);
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status,
            reason_phrase(status),
            content_type,
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });
    (port, handle)
}

/// Spawn a listener that ACCEPTS one connection and then stays
/// SILENT: it consumes the request head but never writes an HTTP
/// response byte, holding the socket open for `hold` so the client's
/// bounded timeout fires as Timeout. Returns the port plus an
/// `accepted` flag proving the connection was really accepted (a
/// refused port would be Unavailable instead).
fn spawn_silent_peer(hold: Duration) -> (u16, Arc<AtomicBool>) {
    let accepted = Arc::new(AtomicBool::new(false));
    let accepted_flag = Arc::clone(&accepted);
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind silent listener");
    let port = listener.local_addr().expect("silent local addr").port();
    std::thread::spawn(move || {
        listener.set_nonblocking(true).expect("nonblocking accept");
        let deadline = Instant::now() + Duration::from_secs(15);
        let (mut stream, _peer) = loop {
            match listener.accept() {
                Ok(conn) => break conn,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() > deadline {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return,
            }
        };
        accepted_flag.store(true, Ordering::SeqCst);
        let _ = read_request_head(&mut stream);
        std::thread::sleep(hold);
    });
    (port, accepted)
}

/// RestAriTransport pointed at a local port, production-style
/// credentials (never the canary unless a test asks for it).
fn transport(port: u16) -> RestAriTransport {
    RestAriTransport::new(
        format!("http://127.0.0.1:{port}"),
        "ep025-user",
        "ep025-pass",
        TIMEOUT,
    )
    .expect("rest ari transport")
}

/// Bind a probe listener, note its port, then DROP it so nothing is
/// listening: a port that is genuinely closed.
fn closed_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind probe listener");
    listener.local_addr().expect("probe local addr").port()
}

// ---------------------------------------------------------------------------
// E: silent peer vs refused port are distinct failure classes.
// ---------------------------------------------------------------------------

#[test]
fn ep025_failure_silent_peer_times_out() {
    // Real listener accepts the connection and then never sends an
    // HTTP response. The 2s bounded production timeout must classify
    // this as Timeout, NOT Unavailable.
    let (port, accepted) = spawn_silent_peer(Duration::from_secs(4));
    let transport = transport(port);
    let err = transport.health().expect_err("silent peer must fail");
    assert_eq!(err.code, CallErrorCode::Timeout, "err: {err}");
    assert!(
        accepted.load(Ordering::SeqCst),
        "the connection must have been accepted (silent, not refused)"
    );
}

#[test]
fn ep025_failure_refused_port_is_unavailable() {
    // No listener on the port: connection is refused immediately.
    // This must be Unavailable and explicitly NOT Timeout, proving
    // the two failure classes are never conflated.
    let port = closed_port();
    let transport = transport(port);
    let err = transport.health().expect_err("refused port must fail");
    assert_eq!(err.code, CallErrorCode::Unavailable, "err: {err}");
    assert_ne!(
        err.code,
        CallErrorCode::Timeout,
        "a closed port is refused, not a silent-peer timeout"
    );
}

// ---------------------------------------------------------------------------
// F: status-only success must NOT be parsed as mandatory JSON, while
// structured GETs fail closed on empty bodies.
// ---------------------------------------------------------------------------

#[test]
fn ep025_failure_empty_body_200_actions_accepted() {
    // Every ARI action whose success response is 200 with an EMPTY
    // body (real Asterisk behavior observed at M3) goes through the
    // status-only post()/delete() helpers: no JSON is expected or
    // decoded, so an empty body is success. Each call gets its own
    // one-request fake server; the observed (method, path) pairs
    // prove the exact DOCUMENTED ARI routes were exercised.
    let observed: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));

    let record = |observed: &Arc<Mutex<Vec<(String, String)>>>| {
        let observed = Arc::clone(observed);
        move |method: &str, path: &str| {
            observed
                .lock()
                .expect("observed lock")
                .push((method.to_string(), path.to_string()));
            (200, "application/json", "")
        }
    };

    let ch = ChannelSelector::new(CHANNEL_ID).expect("channel selector");

    let (port, server) = spawn_server(record(&observed));
    transport(port)
        .answer(&ch)
        .expect("answer: 200 empty body is status-only success");
    server.join().expect("fake server thread");

    let (port, server) = spawn_server(record(&observed));
    transport(port)
        .send_dtmf(&ch, "1")
        .expect("send_dtmf: 200 empty body is status-only success");
    server.join().expect("fake server thread");

    let (port, server) = spawn_server(record(&observed));
    transport(port)
        .start_moh(&ch)
        .expect("start_moh: 200 empty body is status-only success");
    server.join().expect("fake server thread");

    let (port, server) = spawn_server(record(&observed));
    transport(port)
        .stop_moh(&ch)
        .expect("stop_moh: 200 empty body is status-only success");
    server.join().expect("fake server thread");

    let (port, server) = spawn_server(record(&observed));
    transport(port)
        .r#continue(&ch, "internal", "100")
        .expect("continue: 200 empty body is status-only success");
    server.join().expect("fake server thread");

    let (port, server) = spawn_server(record(&observed));
    transport(port)
        .redirect(&ch, "internal", "100")
        .expect("redirect: 200 empty body is status-only success");
    server.join().expect("fake server thread");

    // add_channel_to_bridge is PUBLIC on RestAriTransport and uses the
    // status-only post() helper (POST /ari/bridges/{id}/addChannel).
    let (port, server) = spawn_server(record(&observed));
    transport(port)
        .add_channel_to_bridge("bridge-ep025", &ch)
        .expect("add_channel_to_bridge: 200 empty body is status-only success");
    server.join().expect("fake server thread");

    // Every required status-only route was really hit, with the
    // correct method. NOTE: create_bridge (POST /ari/bridges) is
    // intentionally NOT in this list: it returns an AriBridge object
    // and goes through post_json(), which REQUIRES a JSON body, so an
    // empty body must fail closed there (the structured-get doctrine,
    // proven in ep025_failure_empty_body_structured_get_fails_closed).
    let observed = observed.lock().expect("observed lock");
    let expected = [
        ("POST", "/ari/channels/PJSIP/ep025-c-00000001/answer"),
        ("POST", "/ari/channels/PJSIP/ep025-c-00000001/dtmf"),
        ("POST", "/ari/channels/PJSIP/ep025-c-00000001/moh"),
        ("DELETE", "/ari/channels/PJSIP/ep025-c-00000001/moh"),
        ("POST", "/ari/channels/PJSIP/ep025-c-00000001/continue"),
        ("POST", "/ari/channels/PJSIP/ep025-c-00000001/redirect"),
        ("POST", "/ari/bridges/bridge-ep025/addChannel"),
    ];
    for (method, path) in expected {
        assert!(
            observed.iter().any(|(m, p)| m == method && p == path),
            "expected {method} {path}, observed: {observed:?}"
        );
    }
    assert_eq!(
        observed.len(),
        expected.len(),
        "exactly the status-only routes must be exercised"
    );
}

#[test]
fn ep025_failure_empty_body_structured_get_fails_closed() {
    // GET /ari/channels/{id} returns structured JSON by contract.
    // A 200 with an EMPTY body is therefore NOT success: it must fail
    // closed as External (the status-only helper must never be used
    // for structured GETs).
    let (port, server) = spawn_server(|_method, _path| (200, "application/json", ""));
    let transport = transport(port);
    let ch = ChannelSelector::new(CHANNEL_ID).expect("channel selector");
    let err = transport
        .channel_state(&ch)
        .expect_err("empty structured body must fail closed");
    assert_eq!(err.code, CallErrorCode::External, "err: {err}");
    server.join().expect("fake server thread");
}

// ---------------------------------------------------------------------------
// AB: HTTP status and parser failures classify to typed codes.
// ---------------------------------------------------------------------------

#[test]
fn ep025_failure_malformed_json_fails_closed() {
    // 200 with Content-Type: application/json but a body that is NOT
    // JSON: the structured list call must fail closed as External.
    let (port, server) = spawn_server(|_method, _path| (200, "application/json", GARBAGE_BODY));
    let transport = transport(port);
    let err = transport
        .list_channels()
        .expect_err("malformed JSON must fail closed");
    assert_eq!(err.code, CallErrorCode::External, "err: {err}");
    server.join().expect("fake server thread");
}

#[test]
fn ep025_failure_http_401_authorization() {
    let (port, server) = spawn_server(|_method, _path| (401, "application/json", "{}"));
    let transport = transport(port);
    let err = transport.health().expect_err("401 must fail");
    assert_eq!(err.code, CallErrorCode::Authorization, "err: {err}");
    server.join().expect("fake server thread");
}

#[test]
fn ep025_failure_http_404_not_found() {
    let (port, server) = spawn_server(|_method, _path| (404, "application/json", "{}"));
    let transport = transport(port);
    let ch = ChannelSelector::new(CHANNEL_ID).expect("channel selector");
    let err = transport.channel_state(&ch).expect_err("404 must fail");
    assert_eq!(err.code, CallErrorCode::NotFound, "err: {err}");
    server.join().expect("fake server thread");
}

#[test]
fn ep025_failure_http_409_conflict() {
    // Non-Stasis DTMF (channel not in a Stasis application) is the
    // canonical ARI 409; the transport must classify it as Conflict.
    let (port, server) = spawn_server(|_method, _path| (409, "application/json", "{}"));
    let transport = transport(port);
    let ch = ChannelSelector::new(CHANNEL_ID).expect("channel selector");
    let err = transport.send_dtmf(&ch, "1").expect_err("409 must fail");
    assert_eq!(err.code, CallErrorCode::Conflict, "err: {err}");
    server.join().expect("fake server thread");
}

#[test]
fn ep025_failure_http_503_unavailable() {
    let (port, server) = spawn_server(|_method, _path| (503, "application/json", "{}"));
    let transport = transport(port);
    let err = transport.health().expect_err("503 must fail");
    assert_eq!(err.code, CallErrorCode::Unavailable, "err: {err}");
    server.join().expect("fake server thread");
}

// ---------------------------------------------------------------------------
// U: credentials never leak into error surfaces.
// ---------------------------------------------------------------------------

#[test]
fn ep025_failure_redaction_password_canary_never_in_errors() {
    // A transport built with a DISTINCT canary password must never
    // surface that password on any error path: Display text and
    // serialized JSON are both checked for zero occurrences. Every
    // failure class is exercised (Authorization, Unavailable via
    // HTTP 500, Timeout via silent peer, Unavailable via refused
    // port).
    let canary_transport = |port: u16| {
        RestAriTransport::new(
            format!("http://127.0.0.1:{port}"),
            "ep025-user",
            CANARY,
            TIMEOUT,
        )
        .expect("rest ari transport")
    };
    let assert_redacted = |label: &str, err: &CallError| {
        let display = err.to_string();
        let json = serde_json::to_string(err).expect("serialize error");
        assert!(
            !display.contains(CANARY) && !json.contains(CANARY),
            "{label}: canary leaked into error surface; display={display} json={json}"
        );
    };

    // 401 -> Authorization.
    let (port, server) = spawn_server(|_method, _path| (401, "application/json", "{}"));
    let err = canary_transport(port).health().expect_err("401 must fail");
    assert_eq!(err.code, CallErrorCode::Authorization, "err: {err}");
    assert_redacted("401", &err);
    server.join().expect("fake server thread");

    // 500 -> Unavailable (HTTP error path).
    let (port, server) = spawn_server(|_method, _path| (500, "application/json", "{}"));
    let err = canary_transport(port).health().expect_err("500 must fail");
    assert_eq!(err.code, CallErrorCode::Unavailable, "err: {err}");
    assert_redacted("500", &err);
    server.join().expect("fake server thread");

    // Silent peer -> Timeout.
    let (port, _server) = spawn_silent_peer(Duration::from_secs(4));
    let err = canary_transport(port)
        .health()
        .expect_err("silent peer must fail");
    assert_eq!(err.code, CallErrorCode::Timeout, "err: {err}");
    assert_redacted("silent peer", &err);

    // Refused port -> Unavailable.
    let port = closed_port();
    let err = canary_transport(port)
        .health()
        .expect_err("refused port must fail");
    assert_eq!(err.code, CallErrorCode::Unavailable, "err: {err}");
    assert_redacted("refused port", &err);
}
