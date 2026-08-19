//! Controlled local HTTP fixtures emitting REAL Postiz-shaped and REAL
//! X API v2-shaped responses over real std::net sockets (documented
//! surfaces from docs.postiz.com/public-api and docs.x.com/x-api).
//! Mocks control the PEER only; the transports/adapters under test are
//! never mocked.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub fn read_until_blank_line(stream: &mut TcpStream) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if buf.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    buf
}

pub fn parse_request_line(head: &[u8]) -> (String, String) {
    let text = String::from_utf8_lossy(head);
    let line = text.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    (method, path)
}

fn accept_one(listener: &TcpListener, deadline: Instant) -> Option<TcpStream> {
    loop {
        match listener.accept() {
            Ok((c, _)) => return Some(c),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > deadline {
                    return None;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
}

/// Server that accepts up to `connections` sequential connections and
/// dispatches each request to the handler.
pub fn spawn_server<F>(connections: usize, handler: F) -> (u16, JoinHandle<()>)
where
    F: Fn(&str, &str) -> (u16, &'static str, String) + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        listener.set_nonblocking(true).expect("nonblocking");
        for _ in 0..connections {
            let Some(mut stream) = accept_one(&listener, deadline) else {
                return;
            };
            let head = read_until_blank_line(&mut stream);
            let (method, path) = parse_request_line(&head);
            let (status, content_type, body) = handler(&method, &path);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (port, handle)
}
