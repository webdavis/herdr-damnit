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
