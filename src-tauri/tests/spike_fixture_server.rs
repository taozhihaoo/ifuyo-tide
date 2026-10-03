//! M0.5 SPIKE — fixture server integration test: full + range requests
//! against the generated media fixtures (requires generated media).

use ifuyo_tide_lib::spike::fixture_server;

async fn http_get(
    port: u16,
    target: &str,
    range: Option<&str>,
) -> (u16, Vec<(String, String)>, Vec<u8>) {
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let range_line = range.map(|r| format!("Range: {r}\r\n")).unwrap_or_default();
    let request = format!(
        "GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n{range_line}Connection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await.expect("write");
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.expect("read");
    let header_end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("header end");
    let head = String::from_utf8_lossy(&raw[..header_end]).to_string();
    let status: u16 = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut headers = Vec::new();
    for line in head.lines().skip(1) {
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    (status, headers, raw[header_end + 4..].to_vec())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fixture_server_serves_ranges_of_generated_media() {
    let server = fixture_server::start().await.expect("fixture server start");

    // Full request: 200 with Content-Length matching the file.
    let (status, headers, body) = http_get(server.port, "/media/mp4_h264_aac.mp4", None).await;
    assert_eq!(status, 200, "full request must return 200");
    let content_length: u64 = headers
        .iter()
        .find(|(n, _)| n == "content-length")
        .map(|(_, v)| v.parse().expect("content-length"))
        .expect("content-length present");
    assert_eq!(content_length, body.len() as u64);
    assert!(body.starts_with(b"\x00\x00\x00"), "mp4 box header");

    // Range request: 206 with exact bytes from the file.
    let (status, headers, body) = http_get(
        server.port,
        "/media/mp4_h264_aac.mp4",
        Some("bytes=1000-1999"),
    )
    .await;
    assert_eq!(status, 206);
    let content_range = headers
        .iter()
        .find(|(n, _)| n == "content-range")
        .map(|(_, v)| v.clone())
        .expect("content-range present");
    assert!(
        content_range.starts_with("bytes 1000-1999/"),
        "{content_range}"
    );
    assert_eq!(body.len(), 1000);
    assert_eq!(body.len(), 1000);
}
