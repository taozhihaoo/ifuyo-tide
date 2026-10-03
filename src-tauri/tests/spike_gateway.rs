//! M0.5 SPIKE — gateway security & range integration tests (experiments
//! G / 13.x). Uses a hermetic temp file, no media fixtures required.

use ifuyo_tide_lib::core::resource::ByteRange;
use ifuyo_tide_lib::spike::gateway::Gateway;
use ifuyo_tide_lib::spike::resource::FileResource;
use ifuyo_tide_lib::spike::resource::SpikeResource;
use std::io::Write;
use std::time::Duration;

const LEN: u64 = 1_048_576; // 1 MiB

/// Patterned content: byte i = (i % 251) — verifiable at any offset.
fn pattern_byte(i: u64) -> u8 {
    (i % 251) as u8
}

fn make_temp_file(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tide-spike-gw-{}-{}",
        label,
        ifuyo_tide_lib::core::ids::new_id()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let path = dir.join("content.bin");
    let mut file = std::fs::File::create(&path).expect("create");
    // write in chunks with correct absolute offsets
    let mut buf = vec![0u8; 64 * 1024];
    let mut offset = 0u64;
    while offset < LEN {
        let take = (LEN - offset).min(64 * 1024) as usize;
        for (i, b) in buf.iter_mut().enumerate().take(take) {
            *b = pattern_byte(offset + i as u64);
        }
        file.write_all(&buf[..take]).expect("write");
        offset += take as u64;
    }
    drop(file);
    path
}

struct RawResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

async fn http_request(port: u16, target: &str, host: &str, method: &str) -> RawResponse {
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    let request =
        format!("{method} {target} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.expect("write");
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.expect("read");
    parse_response(&raw)
}

async fn http_request_with_range(
    port: u16,
    target: &str,
    host: &str,
    range: Option<&str>,
) -> RawResponse {
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    let range_line = range.map(|r| format!("Range: {r}\r\n")).unwrap_or_default();
    let request =
        format!("GET {target} HTTP/1.1\r\nHost: {host}\r\n{range_line}Connection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.expect("write");
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.expect("read");
    parse_response(&raw)
}

fn parse_response(raw: &[u8]) -> RawResponse {
    let header_end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("header end");
    let head = String::from_utf8_lossy(&raw[..header_end]).to_string();
    let mut lines = head.lines();
    let status_line = lines.next().unwrap_or_default();
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut headers = Vec::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    RawResponse {
        status,
        headers,
        body: raw[header_end + 4..].to_vec(),
    }
}

async fn setup_gateway(file: &std::path::Path) -> (Gateway, String) {
    let gateway = Gateway::start().await.expect("gateway start");
    let resource = SpikeResource::File(FileResource::open(file.to_path_buf()).expect("file"));
    let token = gateway
        .create_session(resource, "test".to_string(), None)
        .expect("session");
    (gateway, token)
}

#[tokio::test]
async fn gateway_serves_full_file_with_200() {
    let file = make_temp_file("full");
    let (gateway, token) = setup_gateway(&file).await;
    let response = http_request(
        gateway.port,
        &format!("/stream/{token}"),
        "127.0.0.1:9",
        "GET",
    )
    .await;
    assert_eq!(response.status, 200);
    assert_eq!(response.body.len() as u64, LEN);
    assert_eq!(response.body[0], pattern_byte(0));
    assert_eq!(response.body[LEN as usize - 1], pattern_byte(LEN - 1));
    assert_eq!(
        response
            .headers
            .iter()
            .find(|(n, _)| n == "accept-ranges")
            .map(|(_, v)| v.as_str()),
        Some("bytes")
    );
    assert_eq!(
        response
            .headers
            .iter()
            .find(|(n, _)| n == "content-length")
            .map(|(_, v)| v.as_str()),
        Some(LEN.to_string().as_str())
    );
}

#[tokio::test]
async fn gateway_serves_closed_range_206() {
    let file = make_temp_file("closed-range");
    let (gateway, token) = setup_gateway(&file).await;
    let response = http_request_with_range(
        gateway.port,
        &format!("/stream/{token}"),
        "127.0.0.1:9",
        Some("bytes=1000-1999"),
    )
    .await;
    assert_eq!(response.status, 206);
    assert_eq!(response.body.len(), 1000);
    for (i, b) in response.body.iter().enumerate() {
        assert_eq!(*b, pattern_byte(1000 + i as u64));
    }
    let content_range = response
        .headers
        .iter()
        .find(|(n, _)| n == "content-range")
        .map(|(_, v)| v.clone());
    assert_eq!(
        content_range.as_deref(),
        Some(&*format!("bytes 1000-1999/{LEN}"))
    );
    assert_eq!(
        response
            .headers
            .iter()
            .find(|(n, _)| n == "content-length")
            .map(|(_, v)| v.as_str()),
        Some("1000")
    );
}

#[tokio::test]
async fn gateway_serves_suffix_range_from_eof() {
    let file = make_temp_file("suffix");
    let (gateway, token) = setup_gateway(&file).await;
    let response = http_request_with_range(
        gateway.port,
        &format!("/stream/{token}"),
        "127.0.0.1:9",
        Some("bytes=-512"),
    )
    .await;
    assert_eq!(response.status, 206);
    assert_eq!(response.body.len(), 512);
    for (i, b) in response.body.iter().enumerate() {
        assert_eq!(*b, pattern_byte(LEN - 512 + i as u64));
    }
}

#[tokio::test]
async fn gateway_416_on_out_of_bounds() {
    let file = make_temp_file("oob");
    let (gateway, token) = setup_gateway(&file).await;
    // start far beyond the 1 MiB file
    let response = http_request_with_range(
        gateway.port,
        &format!("/stream/{token}"),
        "127.0.0.1:9",
        Some("bytes=9999999-10000000"),
    )
    .await;
    assert_eq!(response.status, 416);
    assert_eq!(
        response
            .headers
            .iter()
            .find(|(n, _)| n == "content-range")
            .map(|(_, v)| v.clone()),
        Some(format!("bytes */{LEN}"))
    );
}

#[tokio::test]
async fn gateway_400_on_reversed_range() {
    let file = make_temp_file("reversed");
    let (gateway, token) = setup_gateway(&file).await;
    let response = http_request_with_range(
        gateway.port,
        &format!("/stream/{token}"),
        "127.0.0.1:9",
        Some("bytes=500-400"),
    )
    .await;
    assert_eq!(response.status, 400);
}

#[tokio::test]
async fn gateway_multi_range_gets_full_body() {
    let file = make_temp_file("multi");
    let (gateway, token) = setup_gateway(&file).await;
    let response = http_request_with_range(
        gateway.port,
        &format!("/stream/{token}"),
        "127.0.0.1:9",
        Some("bytes=0-1,5-9"),
    )
    .await;
    assert_eq!(response.status, 200);
    assert_eq!(response.body.len() as u64, LEN);
}

#[tokio::test]
async fn gateway_rejects_unknown_token_and_paths() {
    let file = make_temp_file("paths");
    let (gateway, _token) = setup_gateway(&file).await;

    // random unguessable token → 404
    let random = ifuyo_tide_lib::core::ids::new_id();
    let response = http_request(
        gateway.port,
        &format!("/stream/{random}"),
        "127.0.0.1:9",
        "GET",
    )
    .await;
    assert_eq!(response.status, 404);

    // path-style access attempts → 404 (never a file error)
    for evil in [
        "/stream/../../etc/passwd",
        "/stream/C:%5CUsers%5Cme",
        "/stream/../..",
        "/file/C:/Users/me",
        "/stream?path=C:\\Users",
    ] {
        let response = http_request(gateway.port, evil, "127.0.0.1:9", "GET").await;
        assert_eq!(response.status, 404, "path {evil} must be rejected");
    }
}

#[tokio::test]
async fn gateway_expires_sessions() {
    let file = make_temp_file("expiry");
    let gateway = Gateway::start().await.expect("gateway start");
    let resource = SpikeResource::File(FileResource::open(file.to_path_buf()).expect("file"));
    let token = gateway
        .create_session(
            resource,
            "short-lived".to_string(),
            Some(Duration::from_millis(50)),
        )
        .expect("session");
    tokio::time::sleep(Duration::from_millis(120)).await;
    let response = http_request(
        gateway.port,
        &format!("/stream/{token}"),
        "127.0.0.1:9",
        "GET",
    )
    .await;
    assert_eq!(response.status, 404, "expired session must be rejected");
}

#[tokio::test]
async fn gateway_revoked_session_is_rejected() {
    let file = make_temp_file("revoke");
    let (gateway, token) = setup_gateway(&file).await;
    assert!(gateway.remove_session(&token));
    let response = http_request(
        gateway.port,
        &format!("/stream/{token}"),
        "127.0.0.1:9",
        "GET",
    )
    .await;
    assert_eq!(response.status, 404);
}

#[tokio::test]
async fn gateway_rejects_bad_host() {
    let file = make_temp_file("host");
    let (gateway, token) = setup_gateway(&file).await;
    let response = http_request(
        gateway.port,
        &format!("/stream/{token}"),
        "evil.example.com",
        "GET",
    )
    .await;
    assert_eq!(response.status, 403);
}

#[tokio::test]
async fn gateway_head_has_headers_but_no_body() {
    let file = make_temp_file("head");
    let (gateway, token) = setup_gateway(&file).await;
    let response = http_request(
        gateway.port,
        &format!("/stream/{token}"),
        "127.0.0.1:9",
        "HEAD",
    )
    .await;
    assert_eq!(response.status, 200);
    assert!(response.body.is_empty());
    assert_eq!(
        response
            .headers
            .iter()
            .find(|(n, _)| n == "content-length")
            .map(|(_, v)| v.as_str()),
        Some(LEN.to_string().as_str())
    );
}

#[tokio::test]
async fn gateway_binds_loopback_only_random_port() {
    let gateway = Gateway::start().await.expect("gateway start");
    // A random port must have been chosen (non-zero, not a fixed value).
    assert_ne!(gateway.port, 0);
    assert_ne!(gateway.port, 3030);
    assert_ne!(gateway.port, 8080);
    // The listener socket must be loopback-only: connecting via the LAN
    // interface is not testable hermetically, so we assert on the bind
    // address indirectly: gateway.session_url uses 127.0.0.1 and the
    // listener was created with "127.0.0.1:0" (see gateway.rs).
    let url = gateway.session_url("abc");
    assert!(url.starts_with("http://127.0.0.1:"));
    let _ = ByteRange::new(0, 1).unwrap();
}
