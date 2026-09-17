//! A loopback HTTP double. Every client test runs against this, never against Todoist.

use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A canned response, served once.
pub struct Double {
    pub base_url: String,
    /// The request line and headers the client sent, filled in once a request arrives.
    pub request: Arc<Mutex<String>>,
}

/// Serve one request with `status`, `headers` (each a full `Name: value` line) and `body`.
pub async fn serve_once(status: &str, headers: &[&str], body: &'static str) -> Double {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let base_url = format!("http://{}", listener.local_addr().expect("addr"));
    let request = Arc::new(Mutex::new(String::new()));
    let seen = Arc::clone(&request);
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{body}",
        body.len(),
        headers
            .iter()
            .map(|header| format!("{header}\r\n"))
            .collect::<String>()
    );
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut buffer = [0u8; 2048];
        let read = socket.read(&mut buffer).await.unwrap_or(0);
        *seen.lock().expect("lock") = String::from_utf8_lossy(&buffer[..read]).to_string();
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.shutdown().await;
    });
    Double { base_url, request }
}

/// A port nothing listens on, for the network-failure case.
pub async fn dead_base_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("addr");
    drop(listener);
    format!("http://{address}")
}

/// A double that keeps answering, choosing a canned body per request.
pub struct Routed {
    pub base_url: String,
    /// The request target of every request served, in order.
    pub targets: Arc<Mutex<Vec<String>>>,
}

/// Serve every request with the body of the first route whose needle appears in the request
/// target. An unmatched request is answered 404 rather than left hanging.
pub async fn serve_routes(routes: &'static [(&'static str, &'static str)]) -> Routed {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let base_url = format!("http://{}", listener.local_addr().expect("addr"));
    let targets = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&targets);
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buffer = [0u8; 2048];
            let read = socket.read(&mut buffer).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let target = request
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_string();
            let body = routes
                .iter()
                .find(|(needle, _)| target.contains(needle))
                .map(|(_, body)| *body);
            seen.lock().expect("lock").push(target);
            let response = match body {
                Some(body) => format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                ),
                None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_string(),
            };
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });
    Routed { base_url, targets }
}
