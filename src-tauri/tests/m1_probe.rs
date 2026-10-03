mod common;
use ifuyo_tide_lib::core::download::url::validate_download_url;
use ifuyo_tide_lib::core::download::HttpTransport;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn probe_engine_transport_on_chunked() {
    let server = common::spawn().await;
    let url = validate_download_url(&server.url("/chunked/test.bin"), true).unwrap();
    let transport = HttpTransport::new().unwrap();
    let mut resp = transport.get(&url, None, None, true).await.unwrap();
    println!("status={} clen={:?}", resp.status, resp.content_length);
    let mut total = 0usize;
    loop {
        match resp.next_chunk().await {
            Ok(Some(b)) => total += b.len(),
            Ok(None) => {
                println!("EOF total={total}");
                break;
            }
            Err(e) => {
                println!("CHUNK_ERR code={} msg={}", e.code(), e.message());
                break;
            }
        }
    }
}
